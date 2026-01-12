// PII Detection Guard
//
// Detects Personally Identifiable Information (PII) in MCP tool descriptions and schemas
// to prevent sensitive data leakage through tool metadata.
//
// Detectable PII types:
// - Email addresses
// - Phone numbers
// - Social Security Numbers (SSN)
// - Credit card numbers
// - IP addresses
// - Physical addresses

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{build_regex_set, NativeGuard};
use crate::mcp::security::{DenyReason, GuardContext, GuardDecision, GuardError, GuardResult};

/// Configuration for PII Detection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PiiDetectionConfig {
    /// PII types to detect
    #[serde(default = "default_pii_types")]
    pub pii_types: Vec<PiiType>,

    /// Minimum confidence threshold (0.0-1.0)
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f32,

    /// Fields to scan in tool metadata
    #[serde(default = "default_scan_fields")]
    pub scan_fields: Vec<ScanField>,

    /// Action to take when PII is detected
    #[serde(default = "default_action")]
    pub action: PiiAction,
}

fn default_pii_types() -> Vec<PiiType> {
    vec![
        PiiType::EmailAddress,
        PiiType::PhoneNumber,
        PiiType::SocialSecurityNumber,
        PiiType::CreditCardNumber,
    ]
}

fn default_confidence_threshold() -> f32 {
    0.80
}

fn default_scan_fields() -> Vec<ScanField> {
    vec![ScanField::Description, ScanField::InputSchema]
}

fn default_action() -> PiiAction {
    PiiAction::Block
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiType {
    EmailAddress,
    PhoneNumber,
    SocialSecurityNumber,
    CreditCardNumber,
    IpAddress,
    PhysicalAddress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PiiAction {
    Block,  // Deny the request
    Warn,   // Log warning but allow
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanField {
    Name,
    Description,
    InputSchema,
}

/// PII Detection implementation
pub struct PiiDetector {
    config: PiiDetectionConfig,
    patterns: Vec<(PiiType, Regex, f32)>, // (type, pattern, confidence)
}

impl PiiDetector {
    pub fn new(config: PiiDetectionConfig) -> Result<Self, GuardError> {
        let patterns = build_pii_patterns(&config.pii_types)?;
        Ok(Self { config, patterns })
    }

    /// Scan tool fields for PII
    fn scan_tool(&self, tool: &rmcp::model::Tool) -> Vec<DetectedPii> {
        let mut detections = Vec::new();

        // Scan tool name
        if self.config.scan_fields.contains(&ScanField::Name) {
            detections.extend(self.scan_text(&tool.name, "tool.name"));
        }

        // Scan tool description
        if self.config.scan_fields.contains(&ScanField::Description) {
            if let Some(desc) = tool.description.as_ref() {
                detections.extend(self.scan_text(desc, "tool.description"));
            }
        }

        // Scan input schema
        if self.config.scan_fields.contains(&ScanField::InputSchema) {
            if let Ok(schema_json) = serde_json::to_string(&tool.input_schema) {
                detections.extend(self.scan_text(&schema_json, "tool.input_schema"));
            }
        }

        detections
    }

    /// Scan text for PII patterns
    fn scan_text(&self, text: &str, field: &str) -> Vec<DetectedPii> {
        let mut detections = Vec::new();

        for (pii_type, pattern, confidence) in &self.patterns {
            if let Some(mat) = pattern.find(text) {
                detections.push(DetectedPii {
                    pii_type: *pii_type,
                    field: field.to_string(),
                    matched_text: redact_pii(&mat.as_str(), pii_type),
                    confidence: *confidence,
                });
            }
        }

        detections
    }
}

impl NativeGuard for PiiDetector {
    fn evaluate_tools_list(
        &self,
        tools: &[rmcp::model::Tool],
        _context: &GuardContext,
    ) -> GuardResult {
        let mut all_detections = Vec::new();

        for tool in tools {
            let detections = self.scan_tool(tool);
            all_detections.extend(detections);
        }

        // Filter by confidence threshold
        all_detections.retain(|pii| pii.confidence >= self.config.confidence_threshold);

        if !all_detections.is_empty() {
            let detection_details = all_detections
                .iter()
                .map(|d| serde_json::json!({
                    "type": format!("{:?}", d.pii_type),
                    "field": d.field,
                    "redacted_value": d.matched_text,
                    "confidence": d.confidence,
                }))
                .collect::<Vec<_>>();

            match self.config.action {
                PiiAction::Block => Ok(GuardDecision::Deny(DenyReason {
                    code: "pii_detected".to_string(),
                    message: format!(
                        "Detected {} PII field(s) in MCP server response",
                        all_detections.len()
                    ),
                    details: Some(serde_json::json!({
                        "detections": detection_details,
                        "threshold": self.config.confidence_threshold,
                    })),
                })),
                PiiAction::Warn => {
                    tracing::warn!(
                        "PII detected in {} tool field(s): {:?}",
                        all_detections.len(),
                        detection_details
                    );
                    Ok(GuardDecision::Allow)
                }
            }
        } else {
            Ok(GuardDecision::Allow)
        }
    }
}

#[derive(Debug, Clone)]
struct DetectedPii {
    pii_type: PiiType,
    field: String,
    matched_text: String, // Redacted version
    confidence: f32,
}

/// Build regex patterns for specified PII types
fn build_pii_patterns(types: &[PiiType]) -> Result<Vec<(PiiType, Regex, f32)>, GuardError> {
    let mut patterns = Vec::new();

    for pii_type in types {
        match pii_type {
            PiiType::EmailAddress => {
                patterns.push((
                    *pii_type,
                    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")
                        .map_err(|e| GuardError::ConfigError(format!("Invalid regex: {}", e)))?,
                    0.90,
                ));
            }
            PiiType::PhoneNumber => {
                patterns.push((
                    *pii_type,
                    Regex::new(r"\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b")
                        .map_err(|e| GuardError::ConfigError(format!("Invalid regex: {}", e)))?,
                    0.80,
                ));
            }
            PiiType::SocialSecurityNumber => {
                patterns.push((
                    *pii_type,
                    Regex::new(r"\b\d{3}-\d{2}-\d{4}\b")
                        .map_err(|e| GuardError::ConfigError(format!("Invalid regex: {}", e)))?,
                    0.95,
                ));
            }
            PiiType::CreditCardNumber => {
                // Match credit card patterns (13-19 digits with optional spaces/dashes)
                patterns.push((
                    *pii_type,
                    Regex::new(r"\b(?:\d[ -]*?){13,19}\b")
                        .map_err(|e| GuardError::ConfigError(format!("Invalid regex: {}", e)))?,
                    0.75, // Lower confidence due to false positives
                ));
            }
            PiiType::IpAddress => {
                patterns.push((
                    *pii_type,
                    Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b")
                        .map_err(|e| GuardError::ConfigError(format!("Invalid regex: {}", e)))?,
                    0.70, // Lower confidence - IP addresses are common in tech contexts
                ));
            }
            PiiType::PhysicalAddress => {
                patterns.push((
                    *pii_type,
                    Regex::new(r"\d+\s+[A-Za-z]+\s+(?:Street|St|Avenue|Ave|Road|Rd|Drive|Dr|Lane|Ln|Boulevard|Blvd)\b")
                        .map_err(|e| GuardError::ConfigError(format!("Invalid regex: {}", e)))?,
                    0.65, // Lower confidence - partial address pattern
                ));
            }
        }
    }

    Ok(patterns)
}

/// Redact PII value for logging/reporting
fn redact_pii(value: &str, pii_type: &PiiType) -> String {
    match pii_type {
        PiiType::EmailAddress => {
            if let Some(at_pos) = value.find('@') {
                format!("{}***@{}", &value[..1.min(value.len())], &value[at_pos + 1..])
            } else {
                "[REDACTED_EMAIL]".to_string()
            }
        }
        PiiType::PhoneNumber => {
            let digits: String = value.chars().filter(|c| c.is_numeric()).collect();
            if digits.len() >= 4 {
                format!("***-***-{}", &digits[digits.len() - 4..])
            } else {
                "[REDACTED_PHONE]".to_string()
            }
        }
        PiiType::SocialSecurityNumber => "***-**-****".to_string(),
        PiiType::CreditCardNumber => {
            let digits: String = value.chars().filter(|c| c.is_numeric()).collect();
            if digits.len() >= 4 {
                format!("****-****-****-{}", &digits[digits.len() - 4..])
            } else {
                "[REDACTED_CC]".to_string()
            }
        }
        PiiType::IpAddress => {
            let parts: Vec<&str> = value.split('.').collect();
            if parts.len() == 4 {
                format!("***.***.***.{}", parts[3])
            } else {
                "[REDACTED_IP]".to_string()
            }
        }
        PiiType::PhysicalAddress => "[REDACTED_ADDRESS]".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Tool;
    use std::borrow::Cow;
    use std::sync::Arc;

    fn create_test_tool(name: &str, description: Option<&str>) -> Tool {
        Tool {
            name: Cow::Owned(name.to_string()),
            description: description.map(|s| Cow::Owned(s.to_string())),
            icons: None,
            title: None,
            meta: None,
            input_schema: Arc::new(
                serde_json::from_value(serde_json::json!({"type": "object"})).unwrap(),
            ),
            annotations: None,
            output_schema: None,
        }
    }

    fn create_test_context() -> GuardContext {
        GuardContext {
            server_name: "test-server".to_string(),
            identity: None,
            metadata: serde_json::json!({}),
        }
    }

    #[test]
    fn test_detects_email() {
        let config = PiiDetectionConfig {
            pii_types: vec![PiiType::EmailAddress],
            confidence_threshold: 0.8,
            scan_fields: vec![ScanField::Description],
            action: PiiAction::Block,
        };

        let detector = PiiDetector::new(config).unwrap();
        let context = create_test_context();

        let tool_with_email = create_test_tool(
            "email_tool",
            Some("Contact us at support@example.com for help"),
        );

        let result = detector.evaluate_tools_list(&[tool_with_email], &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_detects_phone_number() {
        let config = PiiDetectionConfig {
            pii_types: vec![PiiType::PhoneNumber],
            confidence_threshold: 0.7,
            scan_fields: vec![ScanField::Description],
            action: PiiAction::Block,
        };

        let detector = PiiDetector::new(config).unwrap();
        let context = create_test_context();

        let tool_with_phone = create_test_tool(
            "phone_tool",
            Some("Call 555-123-4567 for support"),
        );

        let result = detector.evaluate_tools_list(&[tool_with_phone], &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_detects_ssn() {
        let config = PiiDetectionConfig {
            pii_types: vec![PiiType::SocialSecurityNumber],
            confidence_threshold: 0.9,
            scan_fields: vec![ScanField::Description],
            action: PiiAction::Block,
        };

        let detector = PiiDetector::new(config).unwrap();
        let context = create_test_context();

        let tool_with_ssn = create_test_tool(
            "ssn_tool",
            Some("SSN: 123-45-6789"),
        );

        let result = detector.evaluate_tools_list(&[tool_with_ssn], &context);
        assert!(matches!(result, Ok(GuardDecision::Deny(_))));
    }

    #[test]
    fn test_allows_clean_tools() {
        let config = PiiDetectionConfig {
            pii_types: vec![
                PiiType::EmailAddress,
                PiiType::PhoneNumber,
                PiiType::SocialSecurityNumber,
            ],
            confidence_threshold: 0.8,
            scan_fields: vec![ScanField::Name, ScanField::Description],
            action: PiiAction::Block,
        };

        let detector = PiiDetector::new(config).unwrap();
        let context = create_test_context();

        let clean_tool = create_test_tool(
            "file_reader",
            Some("Reads files from the local filesystem"),
        );

        let result = detector.evaluate_tools_list(&[clean_tool], &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_warn_action() {
        let config = PiiDetectionConfig {
            pii_types: vec![PiiType::EmailAddress],
            confidence_threshold: 0.8,
            scan_fields: vec![ScanField::Description],
            action: PiiAction::Warn,
        };

        let detector = PiiDetector::new(config).unwrap();
        let context = create_test_context();

        let tool_with_email = create_test_tool(
            "email_tool",
            Some("Email: test@example.com"),
        );

        let result = detector.evaluate_tools_list(&[tool_with_email], &context);
        // Warn action should allow but log warning
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_confidence_threshold() {
        let config = PiiDetectionConfig {
            pii_types: vec![PiiType::IpAddress], // Lower confidence (0.70)
            confidence_threshold: 0.75,          // Higher than IP confidence
            scan_fields: vec![ScanField::Description],
            action: PiiAction::Block,
        };

        let detector = PiiDetector::new(config).unwrap();
        let context = create_test_context();

        let tool_with_ip = create_test_tool(
            "ip_tool",
            Some("Server IP: 192.168.1.1"),
        );

        let result = detector.evaluate_tools_list(&[tool_with_ip], &context);
        // Should allow because IP confidence (0.70) < threshold (0.75)
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }
}
