// HTTP webhook transport

use crate::subscriptions::Subscription;
use narayana_core::{Error, Result};
use reqwest::Client;
use serde_json::json;

/// Deliver event via HTTP webhook
pub async fn deliver_webhook(
    subscription: &Subscription,
    payload: &serde_json::Value,
) -> Result<()> {
    let webhook_url = subscription.config.get("webhook_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Storage("webhook_url not configured".to_string()))?;

    // Validate URL
    let url = reqwest::Url::parse(webhook_url)
        .map_err(|e| Error::Storage(format!("Invalid webhook URL: {}", e)))?;
    
    // Security: Only allow http/https
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(Error::Storage("Webhook URL must use http or https".to_string()));
    }
    
    // Security: Prevent SSRF attacks - block localhost and private IPs
    // Parse IP address properly to prevent encoding bypasses
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    
    if let Some(host) = url.host_str() {
        let host_lower = host.to_lowercase();
        
        // Block localhost domain variations
        if host_lower == "localhost" {
            return Err(Error::Storage("Webhook URL cannot point to localhost (SSRF protection)".to_string()));
        }
        
        // Try to parse as IP address
        if let Ok(ip) = host.parse::<IpAddr>() {
            match ip {
                IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    // Block 127.x.x.x (localhost)
                    if octets[0] == 127 {
                        return Err(Error::Storage("Webhook URL cannot point to localhost (SSRF protection)".to_string()));
                    }
                    // Block 0.0.0.0
                    if octets == [0, 0, 0, 0] {
                        return Err(Error::Storage("Webhook URL cannot point to 0.0.0.0 (SSRF protection)".to_string()));
                    }
                    // Block 10.x.x.x (private)
                    if octets[0] == 10 {
                        return Err(Error::Storage("Webhook URL cannot point to private network (SSRF protection)".to_string()));
                    }
                    // Block 192.168.x.x (private)
                    if octets[0] == 192 && octets[1] == 168 {
                        return Err(Error::Storage("Webhook URL cannot point to private network (SSRF protection)".to_string()));
                    }
                    // Block 172.16-31.x.x (private)
                    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
                        return Err(Error::Storage("Webhook URL cannot point to private network (SSRF protection)".to_string()));
                    }
                }
                IpAddr::V6(ipv6) => {
                    let segments = ipv6.segments();
                    // Block ::1 (localhost)
                    if segments == [0, 0, 0, 0, 0, 0, 0, 1] {
                        return Err(Error::Storage("Webhook URL cannot point to localhost (SSRF protection)".to_string()));
                    }
                    // Block :: (unspecified)
                    if segments == [0, 0, 0, 0, 0, 0, 0, 0] {
                        return Err(Error::Storage("Webhook URL cannot point to unspecified address (SSRF protection)".to_string()));
                    }
                    // Block link-local (fe80::/10)
                    if segments[0] == 0xfe80 && (segments[1] & 0xc000) == 0x8000 {
                        return Err(Error::Storage("Webhook URL cannot point to link-local address (SSRF protection)".to_string()));
                    }
                    // Block unique-local (fc00::/7)
                    if (segments[0] & 0xfe00) == 0xfc00 {
                        return Err(Error::Storage("Webhook URL cannot point to unique-local address (SSRF protection)".to_string()));
                    }
                }
            }
        }
        
        // Also check for encoded IPs in domain (e.g., 127.1, 0x7f.0.0.1)
        // Block if it looks like an encoded IP
        if host_lower.starts_with("127.") ||
           host_lower.starts_with("192.168.") ||
           host_lower.starts_with("10.") ||
           (host_lower.starts_with("172.") && host_lower.len() > 4) {
            return Err(Error::Storage("Webhook URL cannot point to localhost or private network (SSRF protection)".to_string()));
        }
    }
    
    // Validate URL length
    if webhook_url.len() > 2048 {
        return Err(Error::Storage("Webhook URL too long (max 2048 chars)".to_string()));
    }

    // Reuse client with timeout and connection pooling
    // Note: In production, this should be a shared static client
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Storage(format!("Failed to create HTTP client: {}", e)))?;
    
    // Build webhook payload
    let webhook_payload = json!({
        "event_name": subscription.event_name.to_string(),
        "payload": payload,
        "timestamp": chrono::Utc::now().timestamp(),
    });

    // Add HMAC signature if secret is configured
    let mut request = client.post(webhook_url).json(&webhook_payload);
    
    if let Some(secret) = subscription.config.get("webhook_secret").and_then(|v| v.as_str()) {
        let signature = generate_hmac(&webhook_payload, secret)?;
        request = request.header("X-Narayana-Signature", signature);
    }

    // Add custom headers if configured
    // Security: Validate headers to prevent injection
    if let Some(headers) = subscription.config.get("headers").and_then(|v| v.as_object()) {
        for (key, value) in headers {
            // Validate header name (prevent injection)
            if key.is_empty() || key.len() > 256 {
                continue; // Skip invalid header names
            }
            // Prevent control characters in header names
            if key.chars().any(|c| c.is_control() || c == ':') {
                continue; // Skip invalid header names
            }
            // Block dangerous headers
            let key_lower = key.to_lowercase();
            if key_lower == "host" || 
               key_lower == "content-length" || 
               key_lower.starts_with("x-forwarded") ||
               key_lower.starts_with("x-real-ip") {
                continue; // Block dangerous headers
            }
            
            if let Some(header_value) = value.as_str() {
                // Validate header value length
                if header_value.len() > 4096 {
                    continue; // Skip overly long headers
                }
                // Prevent control characters in header values
                if header_value.chars().any(|c| c == '\r' || c == '\n') {
                    continue; // Prevent header injection
                }
                request = request.header(key, header_value);
            }
        }
    }

    // Send webhook with retry logic
    let mut retries = 3;
    let mut delay = std::time::Duration::from_millis(100);
    const MAX_DELAY: std::time::Duration = std::time::Duration::from_secs(30);

    loop {
        // Clone request safely
        let request_clone = match request.try_clone() {
            Some(req) => req,
            None => {
                // If can't clone, create new request
                let mut new_request = client.post(webhook_url).json(&webhook_payload);
                if let Some(secret) = subscription.config.get("webhook_secret").and_then(|v| v.as_str()) {
                    if let Ok(signature) = generate_hmac(&webhook_payload, secret) {
                        new_request = new_request.header("X-Narayana-Signature", signature);
                    }
                }
                // SECURITY: Re-validate headers when recreating request
                if let Some(headers) = subscription.config.get("headers").and_then(|v| v.as_object()) {
                    for (key, value) in headers {
                        // Validate header name (prevent injection)
                        if key.is_empty() || key.len() > 256 {
                            continue;
                        }
                        if key.chars().any(|c| c.is_control() || c == ':') {
                            continue;
                        }
                        // Block dangerous headers
                        let key_lower = key.to_lowercase();
                        if key_lower == "host" || 
                           key_lower == "content-length" || 
                           key_lower.starts_with("x-forwarded") ||
                           key_lower.starts_with("x-real-ip") {
                            continue;
                        }
                        
                        if let Some(header_value) = value.as_str() {
                            if header_value.len() > 4096 {
                                continue;
                            }
                            if header_value.chars().any(|c| c == '\r' || c == '\n') {
                                continue;
                            }
                            new_request = new_request.header(key, header_value);
                        }
                    }
                }
                new_request
            }
        };
        
        match request_clone.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    return Ok(());
                } else {
                    tracing::warn!("Webhook returned non-success status: {}", response.status());
                }
            }
            Err(e) => {
                tracing::warn!("Webhook delivery failed: {}", e);
            }
        }

        retries -= 1;
        if retries == 0 {
            return Err(Error::Storage("Failed to deliver webhook after retries".to_string()));
        }

        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(MAX_DELAY); // Exponential backoff with cap
    }
}

/// Generate HMAC signature
fn generate_hmac(payload: &serde_json::Value, secret: &str) -> Result<String> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    // Validate secret
    if secret.is_empty() {
        return Err(Error::Storage("HMAC secret cannot be empty".to_string()));
    }
    if secret.len() > 1024 {
        return Err(Error::Storage("HMAC secret too long (max 1024 chars)".to_string()));
    }
    // Prevent control characters in secret
    if secret.chars().any(|c| c.is_control()) {
        return Err(Error::Storage("HMAC secret cannot contain control characters".to_string()));
    }
    
    // SECURITY: Ensure secret never appears in error messages or logs
    // (Already handled - we only return generic errors)

    let payload_str = serde_json::to_string(payload)
        .map_err(|e| Error::Storage(format!("Failed to serialize payload: {}", e)))?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| Error::Storage(format!("HMAC error: {}", e)))?;
    mac.update(payload_str.as_bytes());
    let result = mac.finalize();
    let signature = hex::encode(result.into_bytes());
    Ok(format!("sha256={}", signature))
}

