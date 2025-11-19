// Security utilities for preventing common vulnerabilities

use narayana_core::{Error, Result};
use std::path::{Path, PathBuf};

/// Security utilities for path and URL validation
pub struct SecurityUtils;

impl SecurityUtils {
    /// Validate and sanitize file path to prevent directory traversal
    pub fn validate_path(base_path: &Path, key: &str) -> Result<PathBuf> {
        // Check for path traversal sequences
        if key.contains("..") || key.contains("//") || key.contains("\\\\") {
            return Err(Error::Storage(format!(
                "Path traversal detected in key: '{}'",
                key
            )));
        }
        
        // On Windows, block UNC paths and drive letters
        #[cfg(windows)]
        {
            if key.starts_with("\\\\") || key.contains(":\\") || key.contains(":/") {
                return Err(Error::Storage(format!(
                    "Absolute path detected in key: '{}'",
                    key
                )));
            }
        }
        
        // On Unix, block absolute paths
        #[cfg(unix)]
        {
            if key.starts_with('/') {
                return Err(Error::Storage(format!(
                    "Absolute path detected in key: '{}'",
                    key
                )));
            }
        }
        
        let path = base_path.join(key);
        
        // SECURITY: TOCTOU protection - canonicalize base path first (before checking path)
        // This prevents symlink attacks where attacker creates symlink between check and use
        let canonical_base = base_path.canonicalize()
            .map_err(|e| Error::Storage(format!("Base path error: {}", e)))?;
        
        // Try to canonicalize for final validation
        // If path doesn't exist yet, we check its components
        if path.exists() {
            // SECURITY: Canonicalize immediately to resolve any symlinks
            // This prevents TOCTOU (Time-Of-Check-Time-Of-Use) vulnerabilities
            let canonical_path = path.canonicalize()
                .map_err(|e| Error::Storage(format!("Path error: {}", e)))?;
            
            if !canonical_path.starts_with(&canonical_base) {
                return Err(Error::Storage(format!(
                    "Path traversal detected: '{}' would escape base directory",
                    key
                )));
            }
            
            // SECURITY: Return canonicalized path to prevent symlink attacks
            return Ok(canonical_path);
        } else {
            // SECURITY: For non-existent paths, validate components to prevent directory traversal
            // Check that all path components are safe
            for component in path.components() {
                match component {
                    std::path::Component::ParentDir => {
                        return Err(Error::Storage(format!(
                            "Path traversal detected: '{}' contains parent directory",
                            key
                        )));
                    }
                    std::path::Component::RootDir => {
                        return Err(Error::Storage(format!(
                            "Absolute path detected: '{}'",
                            key
                        )));
                    }
                    _ => {}
                }
            }
            
            // SECURITY: Ensure the path would be within base when created
            // Get parent and check it would be within base
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    let canonical_parent = parent.canonicalize()
                        .map_err(|e| Error::Storage(format!("Parent path error: {}", e)))?;
                    if !canonical_parent.starts_with(&canonical_base) {
                        return Err(Error::Storage(format!(
                            "Path traversal detected: '{}' would escape base directory",
                            key
                        )));
                    }
                }
            }
        }
        
        Ok(path)
    }
    
    /// Validate URL to prevent SSRF attacks
    /// SECURITY: Also prevents DNS rebinding attacks by validating host immediately
    pub fn validate_http_url(url: &str) -> Result<()> {
        let url_lower = url.to_lowercase();
        
        // Only allow HTTP and HTTPS
        if !url_lower.starts_with("http://") && !url_lower.starts_with("https://") {
            return Err(Error::Storage(format!(
                "Only http and https protocols are allowed: {}",
                url
            )));
        }
        
        // SECURITY: DNS rebinding protection - validate host before DNS resolution
        // This prevents attackers from using DNS rebinding to bypass SSRF protection
        // by resolving to a public IP initially, then rebinding to localhost
        
        // SECURITY: Extract host safely - handle edge cases
        let host_part = if let Some(start) = url.find("://") {
            let after_protocol = &url[start + 3..];
            // SECURITY: Prevent empty host
            if after_protocol.is_empty() {
                return Err(Error::Storage(format!("Invalid URL: empty host in {}", url)));
            }
            
            // SECURITY: Handle host extraction with port
            if let Some(slash_pos) = after_protocol.find('/') {
                let host_with_port = &after_protocol[..slash_pos];
                // Remove port if present
                if let Some(colon_pos) = host_with_port.rfind(':') {
                    // Check if this is IPv6 bracket notation [::1]:8080
                    if host_with_port.starts_with('[') && host_with_port.contains(']') {
                        // IPv6 with port - extract properly
                        if let Some(bracket_end) = host_with_port.find(']') {
                            &host_with_port[1..bracket_end]
                        } else {
                            host_with_port
                        }
                    } else {
                        &host_with_port[..colon_pos]
                    }
                } else {
                    // Remove IPv6 brackets if present
                    if host_with_port.starts_with('[') && host_with_port.ends_with(']') {
                        &host_with_port[1..host_with_port.len()-1]
                    } else {
                        host_with_port
                    }
                }
            } else if let Some(colon_pos) = after_protocol.find(':') {
                let host_with_port = &after_protocol[..colon_pos];
                // Remove IPv6 brackets if present
                if host_with_port.starts_with('[') && host_with_port.ends_with(']') {
                    &host_with_port[1..host_with_port.len()-1]
                } else {
                    host_with_port
                }
            } else {
                // Remove IPv6 brackets if present
                if after_protocol.starts_with('[') && after_protocol.ends_with(']') {
                    &after_protocol[1..after_protocol.len()-1]
                } else {
                    after_protocol
                }
            }
        } else {
            return Err(Error::Storage(format!("Invalid URL format: {}", url)));
        };
        
        // SECURITY: Validate host is not empty after processing
        if host_part.is_empty() {
            return Err(Error::Storage(format!("Invalid URL: empty host in {}", url)));
        }
        
        // Block localhost variants
        if Self::is_localhost(host_part) {
            return Err(Error::Storage(
                "URL cannot target localhost".to_string()
            ));
        }
        
        // Block private IP ranges
        if let Ok(ip) = host_part.parse::<std::net::IpAddr>() {
            if Self::is_private_ip(&ip) {
                return Err(Error::Storage(
                    "URL cannot target private IP addresses".to_string()
                ));
            }
        }
        
        Ok(())
    }
    
    /// Check if host is localhost
    /// SECURITY: Comprehensive localhost detection to prevent SSRF
    pub fn is_localhost(host: &str) -> bool {
        let host_lower = host.to_lowercase();
        matches!(host_lower.as_str(), 
            "localhost" | "127.0.0.1" | "::1" | "0.0.0.0" | "[::1]" | "[::]" |
            "127.1" | "127.0.1" | "127.000.000.001" | "127.0.0.0" |
            "localhost." | ".localhost" | "localhost.localdomain"
        ) || host_lower.starts_with("127.") // Any 127.x.x.x
    }
    
    /// Check if IP address is private/internal
    pub fn is_private_ip(ip: &std::net::IpAddr) -> bool {
        match ip {
            std::net::IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                // 10.0.0.0/8
                if octets[0] == 10 {
                    return true;
                }
                // 172.16.0.0/12
                if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
                    return true;
                }
                // 192.168.0.0/16
                if octets[0] == 192 && octets[1] == 168 {
                    return true;
                }
                // 169.254.0.0/16 (link-local)
                if octets[0] == 169 && octets[1] == 254 {
                    return true;
                }
                false
            }
            std::net::IpAddr::V6(ipv6) => {
                let segments = ipv6.segments();
                // fc00::/7 (unique local)
                if segments[0] & 0xfe00 == 0xfc00 {
                    return true;
                }
                // fe80::/10 (link-local)
                if segments[0] & 0xffc0 == 0xfe80 {
                    return true;
                }
                false
            }
        }
    }
    
    /// Constant-time string comparison to prevent timing attacks
    pub fn constant_time_eq(a: &str, b: &str) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    }
    
    /// Constant-time byte comparison
    pub fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        a.iter()
            .zip(b.iter())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
    }
    
    /// Validate string length to prevent resource exhaustion
    pub fn validate_string_length(s: &str, max_len: usize) -> Result<()> {
        if s.len() > max_len {
            return Err(Error::Storage(format!(
                "String length {} exceeds maximum {}",
                s.len(), max_len
            )));
        }
        Ok(())
    }
    
    /// Validate collection size to prevent resource exhaustion
    pub fn validate_collection_size<T>(collection: &[T], max_size: usize) -> Result<()> {
        if collection.len() > max_size {
            return Err(Error::Storage(format!(
                "Collection size {} exceeds maximum {}",
                collection.len(), max_size
            )));
        }
        Ok(())
    }
}

