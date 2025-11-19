// Comprehensive Bug Detection and Edge Case Prevention
// Systematic detection of bugs, race conditions, memory safety issues, etc.

use narayana_core::{Error, Result};
use std::time::SystemTime;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use tracing::{warn, error};

/// Bug detector - finds potential bugs and edge cases
pub struct BugDetector {
    detected_issues: Arc<RwLock<Vec<BugReport>>>,
    checks_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct BugReport {
    pub severity: BugSeverity,
    pub category: BugCategory,
    pub location: String,
    pub description: String,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BugSeverity {
    Critical,  // Security vulnerability or data corruption
    High,      // Potential crash or incorrect behavior
    Medium,    // Edge case that might cause issues
    Low,       // Minor issue or code smell
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BugCategory {
    RaceCondition,
    MemorySafety,
    IntegerOverflow,
    DivisionByZero,
    NullDereference,
    IndexOutOfBounds,
    UnwrapPanic,
    ResourceLeak,
    LogicError,
    EdgeCase,
    UnsafeCode,
}

impl BugDetector {
    pub fn new() -> Self {
        Self {
            detected_issues: Arc::new(RwLock::new(Vec::new())),
            checks_enabled: true,
        }
    }

    /// Check for unsafe memory operations
    pub fn check_unsafe_operations(&self, location: &str, operation: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        // Check for potential issues in unsafe blocks
        if operation.contains("unsafe") {
            if !operation.contains("size_of") {
                issues.push(BugReport {
                    severity: BugSeverity::High,
                    category: BugCategory::UnsafeCode,
                    location: location.to_string(),
                    description: "Unsafe block without proper size validation".to_string(),
                    suggested_fix: "Ensure size calculations are validated before unsafe operations".to_string(),
                });
            }
        }

        issues
    }

    /// Check for potential integer overflow
    pub fn check_integer_overflow(&self, location: &str, operation: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        // Check for multiplication without overflow checks
        if operation.contains("*") && !operation.contains("checked_mul") {
            issues.push(BugReport {
                severity: BugSeverity::High,
                category: BugCategory::IntegerOverflow,
                location: location.to_string(),
                description: "Multiplication without overflow check".to_string(),
                suggested_fix: "Use checked_mul() or saturating_mul() to prevent overflow".to_string(),
            });
        }

        // Check for addition without overflow checks
        if operation.contains("+") && !operation.contains("checked_add") && operation.contains("size") {
            issues.push(BugReport {
                severity: BugSeverity::High,
                category: BugCategory::IntegerOverflow,
                location: location.to_string(),
                description: "Size calculation without overflow check".to_string(),
                suggested_fix: "Use checked_add() or saturating_add() for size calculations".to_string(),
            });
        }

        issues
    }

    /// Check for potential division by zero
    pub fn check_division_by_zero(&self, location: &str, operation: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        if operation.contains("/") && !operation.contains("if") && !operation.contains("!= 0") {
            issues.push(BugReport {
                severity: BugSeverity::Critical,
                category: BugCategory::DivisionByZero,
                location: location.to_string(),
                description: "Division without zero check".to_string(),
                suggested_fix: "Check denominator != 0 before division".to_string(),
            });
        }

        issues
    }

    /// Check for unwrap/expect that might panic
    pub fn check_unwrap_usage(&self, location: &str, code: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        // SystemTime unwraps should be safe but could theoretically fail
        if code.contains("SystemTime::now().duration_since(UNIX_EPOCH).unwrap()") {
            issues.push(BugReport {
                severity: BugSeverity::Low,
                category: BugCategory::UnwrapPanic,
                location: location.to_string(),
                description: "SystemTime unwrap could theoretically fail if clock is before epoch".to_string(),
                suggested_fix: "Use duration_since(UNIX_EPOCH).unwrap_or_default() or handle error".to_string(),
            });
        }

        // Regex unwraps should be validated
        if code.contains("Regex::new") && code.contains(".unwrap()") {
            issues.push(BugReport {
                severity: BugSeverity::Medium,
                category: BugCategory::UnwrapPanic,
                location: location.to_string(),
                description: "Regex compilation unwrap could panic on invalid pattern".to_string(),
                suggested_fix: "Validate regex patterns at compile time or handle errors".to_string(),
            });
        }

        issues
    }

    /// Check for potential race conditions
    pub fn check_race_conditions(&self, location: &str, code: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        // Check for read-then-write pattern
        if code.contains(".read()") && code.contains(".write()") && 
           code.contains("drop") && code.contains("get") {
            issues.push(BugReport {
                severity: BugSeverity::High,
                category: BugCategory::RaceCondition,
                location: location.to_string(),
                description: "Potential race condition: read lock released before write lock acquired".to_string(),
                suggested_fix: "Use atomic operations or ensure lock ordering prevents races".to_string(),
            });
        }

        issues
    }

    /// Check for index out of bounds
    pub fn check_index_bounds(&self, location: &str, code: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        // Check for array indexing without bounds check
        if (code.contains("[") || code.contains(".get(")) && 
           !code.contains(".len()") && !code.contains(".get(") {
            issues.push(BugReport {
                severity: BugSeverity::High,
                category: BugCategory::IndexOutOfBounds,
                location: location.to_string(),
                description: "Array/vector access without bounds checking".to_string(),
                suggested_fix: "Use .get() or check bounds before indexing".to_string(),
            });
        }

        issues
    }

    /// Check for memory leaks
    pub fn check_memory_leaks(&self, location: &str, code: &str) -> Vec<BugReport> {
        let mut issues = Vec::new();
        
        // Check for Arc/Rc cycles
        if code.contains("Arc::new(Arc::") || code.contains("Rc::new(Rc::") {
            issues.push(BugReport {
                severity: BugSeverity::Medium,
                category: BugCategory::ResourceLeak,
                location: location.to_string(),
                description: "Potential circular reference causing memory leak".to_string(),
                suggested_fix: "Use Weak references to break cycles".to_string(),
            });
        }

        issues
    }

    /// Scan code for all potential bugs
    pub fn scan_code(&self, location: &str, code: &str) -> Vec<BugReport> {
        let mut all_issues = Vec::new();
        
        all_issues.extend(self.check_unsafe_operations(location, code));
        all_issues.extend(self.check_integer_overflow(location, code));
        all_issues.extend(self.check_division_by_zero(location, code));
        all_issues.extend(self.check_unwrap_usage(location, code));
        all_issues.extend(self.check_race_conditions(location, code));
        all_issues.extend(self.check_index_bounds(location, code));
        all_issues.extend(self.check_memory_leaks(location, code));
        
        // Store issues
        if self.checks_enabled {
            let mut issues = self.detected_issues.write();
            issues.extend(all_issues.clone());
        }
        
        all_issues
    }

    /// Get all detected issues
    pub fn get_all_issues(&self) -> Vec<BugReport> {
        self.detected_issues.read().clone()
    }

    /// Get critical issues only
    pub fn get_critical_issues(&self) -> Vec<BugReport> {
        self.detected_issues.read()
            .iter()
            .filter(|issue| issue.severity == BugSeverity::Critical)
            .cloned()
            .collect()
    }

    /// Clear all issues
    pub fn clear_issues(&self) {
        self.detected_issues.write().clear();
    }
}

/// Safe math utilities to prevent overflow/underflow
pub struct SafeMath;

impl SafeMath {
    /// Safe multiplication with overflow check
    pub fn safe_mul(a: usize, b: usize) -> Result<usize> {
        a.checked_mul(b)
            .ok_or_else(|| Error::Storage(format!("Integer overflow: {} * {}", a, b)))
    }

    /// Safe addition with overflow check
    pub fn safe_add(a: usize, b: usize) -> Result<usize> {
        a.checked_add(b)
            .ok_or_else(|| Error::Storage(format!("Integer overflow: {} + {}", a, b)))
    }

    /// Safe subtraction with underflow check
    pub fn safe_sub(a: usize, b: usize) -> Result<usize> {
        a.checked_sub(b)
            .ok_or_else(|| Error::Storage(format!("Integer underflow: {} - {}", a, b)))
    }

    /// Safe division with zero check
    pub fn safe_div(a: usize, b: usize) -> Result<usize> {
        if b == 0 {
            return Err(Error::Storage("Division by zero".to_string()));
        }
        Ok(a / b)
    }

    /// Safe modulo with zero check
    pub fn safe_mod(a: usize, b: usize) -> Result<usize> {
        if b == 0 {
            return Err(Error::Storage("Modulo by zero".to_string()));
        }
        Ok(a % b)
    }
}

/// Safe system time - never panics
pub struct SafeSystemTime;

impl SafeSystemTime {
    /// Get current timestamp (never panics)
    pub fn now_seconds() -> u64 {
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Get current timestamp as nanos (never panics)
    pub fn now_nanos() -> u64 {
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

/// Safe regex builder - validates at compile time when possible
pub struct SafeRegexBuilder;

impl SafeRegexBuilder {
    /// Create regex (validated at initialization)
    pub fn new(pattern: &'static str) -> Result<regex::Regex> {
        regex::Regex::new(pattern)
            .map_err(|e| Error::Storage(format!("Invalid regex pattern '{}': {}", pattern, e)))
    }
}

/// Safe bounds checker
pub struct SafeBounds;

impl SafeBounds {
    /// Check if index is valid
    pub fn is_valid_index(index: usize, length: usize) -> bool {
        index < length
    }

    /// Get element safely
    pub fn get<T>(slice: &[T], index: usize) -> Option<&T> {
        slice.get(index)
    }

    /// Get element mutably safely
    pub fn get_mut<T>(slice: &mut [T], index: usize) -> Option<&mut T> {
        slice.get_mut(index)
    }

    /// Validate bounds before unsafe operation
    pub fn validate_bounds<T>(slice: &[T], count: usize) -> Result<()> {
        if slice.len() < count {
            return Err(Error::Storage(format!(
                "Index out of bounds: slice len {} < count {}",
                slice.len(),
                count
            )));
        }
        Ok(())
    }

    /// Validate memory size for unsafe operations
    pub fn validate_size_for_type<T>(data_len: usize, expected_count: usize) -> Result<()> {
        let size = std::mem::size_of::<T>();
        let expected_len = SafeMath::safe_mul(expected_count, size)?;
        
        if data_len != expected_len {
            return Err(Error::Storage(format!(
                "Invalid data length: got {}, expected {}",
                data_len, expected_len
            )));
        }
        
        Ok(())
    }
}

/// Edge case validator
pub struct EdgeCaseValidator;

impl EdgeCaseValidator {
    /// Check for empty collections
    pub fn check_empty<T>(collection: &[T], context: &str) -> Result<()> {
        if collection.is_empty() {
            return Err(Error::Storage(format!("Empty collection in context: {}", context)));
        }
        Ok(())
    }

    /// Check for maximum values
    pub fn check_max_value<T: PartialOrd + Copy>(value: T, max: T, context: &str) -> Result<()> {
        if value > max {
            return Err(Error::Storage(format!(
                "Value exceeds maximum in context {}: value > max",
                context
            )));
        }
        Ok(())
    }

    /// Check for minimum values
    pub fn check_min_value<T: PartialOrd + Copy>(value: T, min: T, context: &str) -> Result<()> {
        if value < min {
            return Err(Error::Storage(format!(
                "Value below minimum in context {}: value < min",
                context
            )));
        }
        Ok(())
    }

    /// Validate string length
    pub fn validate_string_length(s: &str, max_len: usize, context: &str) -> Result<()> {
        if s.len() > max_len {
            return Err(Error::Storage(format!(
                "String too long in context {}: len {} > max {}",
                context, s.len(), max_len
            )));
        }
        Ok(())
    }

    /// Validate numeric range
    pub fn validate_range<T: PartialOrd + Copy + std::fmt::Debug>(
        value: T,
        min: T,
        max: T,
        context: &str,
    ) -> Result<()> {
        if value < min || value > max {
            return Err(Error::Storage(format!(
                "Value out of range in context {}: {} not in [{}, {}]",
                context, format!("{:?}", value), format!("{:?}", min), format!("{:?}", max)
            )));
        }
        Ok(())
    }
}

