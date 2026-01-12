// PII (Personally Identifiable Information) Detection Guard
//
// Detects and optionally masks PII in MCP requests and responses.
// Reuses the existing LLM PII recognizers for consistency.
//
// Supported PII types:
// - Email addresses
// - Phone numbers (US, GB, DE, IL, IN, CA, BR)
// - US Social Security Numbers (SSN)
// - Credit card numbers (Visa, Mastercard, Amex, Discover, Diners Club)
// - Canadian Social Insurance Numbers (SIN)
// - URLs

use serde::{Deserialize, Serialize};

use super::NativeGuard;
use crate::mcp::security::{DenyReason, GuardContext, GuardDecision, GuardResult, ModifyAction};
use crate::pii;

// Re-export PiiType from the shared pii module
pub use crate::pii::PiiType;

/// Action to take when PII is detected
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PiiAction {
	/// Mask detected PII with <ENTITY_TYPE> placeholder
	#[default]
	Mask,
	/// Reject the request/response entirely
	Reject,
}

/// Configuration for PII Guard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PiiGuardConfig {
	/// Which PII types to detect (defaults to all)
	#[serde(default = "default_pii_types")]
	pub detect: Vec<PiiType>,

	/// Action to take when PII is detected
	#[serde(default)]
	pub action: PiiAction,

	/// Minimum confidence score to trigger detection (0.0 - 1.0)
	#[serde(default = "default_min_score")]
	pub min_score: f32,

	/// Custom rejection message (only used when action is Reject)
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rejection_message: Option<String>,
}

fn default_pii_types() -> Vec<PiiType> {
	PiiType::all()
}

fn default_min_score() -> f32 {
	0.3 // Low threshold to catch most PII
}

impl Default for PiiGuardConfig {
	fn default() -> Self {
		Self {
			detect: default_pii_types(),
			action: PiiAction::default(),
			min_score: default_min_score(),
			rejection_message: None,
		}
	}
}

/// PII Detection Guard for MCP Security
pub struct PiiGuard {
	config: PiiGuardConfig,
}

impl PiiGuard {
	pub fn new(config: PiiGuardConfig) -> Self {
		tracing::info!(
			detect_types = ?config.detect,
			action = ?config.action,
			min_score = config.min_score,
			"PiiGuard::new - creating guard with config"
		);
		Self { config }
	}

	/// Scan text for all configured PII types
	fn scan_text(&self, text: &str) -> Vec<pii::RecognizerResult> {
		let mut all_results = Vec::new();

		for pii_type in &self.config.detect {
			let results = pii_type.recognizer().recognize(text);

			// Filter by minimum score
			for result in results {
				if result.score >= self.config.min_score {
					all_results.push(result);
				}
			}
		}

		// Sort by position (reverse order for masking)
		all_results.sort_by(|a, b| b.start.cmp(&a.start));
		all_results
	}

	/// Apply masking to text, replacing PII with <ENTITY_TYPE> placeholders
	fn mask_text(&self, text: &str, results: &[pii::RecognizerResult]) -> String {
		if results.is_empty() {
			return text.to_string();
		}

		// Filter out overlapping results (keep first match when overlapping)
		// Results are already sorted in reverse order (end to start)
		let mut non_overlapping: Vec<&pii::RecognizerResult> = Vec::new();
		for result in results {
			// Validate byte indices are within bounds and at char boundaries
			if result.start > text.len()
				|| result.end > text.len()
				|| !text.is_char_boundary(result.start)
				|| !text.is_char_boundary(result.end)
			{
				continue;
			}

			// Check for overlap with already selected results
			let overlaps = non_overlapping.iter().any(|existing| {
				// Since results are sorted in reverse, existing results start after current
				result.end > existing.start && result.start < existing.end
			});

			if !overlaps {
				non_overlapping.push(result);
			}
		}

		// Build new string with replacements (processing from end to start)
		let mut masked = text.to_string();
		for result in non_overlapping {
			masked.replace_range(
				result.start..result.end,
				&format!("<{}>", result.entity_type.to_uppercase()),
			);
		}

		masked
	}

	/// Recursively mask PII in a JSON value, returning true if any masking occurred
	fn mask_json_value(&self, value: &mut serde_json::Value) -> bool {
		let mut any_masked = false;

		match value {
			serde_json::Value::String(s) => {
				let results = self.scan_text(s);
				if !results.is_empty() {
					*s = self.mask_text(s, &results);
					any_masked = true;
				}
			},
			serde_json::Value::Array(arr) => {
				for item in arr {
					if self.mask_json_value(item) {
						any_masked = true;
					}
				}
			},
			serde_json::Value::Object(obj) => {
				for (_, val) in obj {
					if self.mask_json_value(val) {
						any_masked = true;
					}
				}
			},
			_ => {},
		}

		any_masked
	}

	/// Scan JSON for PII and collect all detections with their paths
	fn collect_detections(&self, value: &serde_json::Value) -> Vec<PiiDetection> {
		let mut detections = Vec::new();
		self.collect_detections_recursive(value, Vec::new(), &mut detections);
		detections
	}

	fn collect_detections_recursive(
		&self,
		value: &serde_json::Value,
		path: Vec<String>,
		results: &mut Vec<PiiDetection>,
	) {
		match value {
			serde_json::Value::String(s) => {
				let scan_results = self.scan_text(s);
				for result in scan_results {
					results.push(PiiDetection {
						path: path.clone(),
						entity_type: result.entity_type.clone(),
						score: result.score,
					});
				}
			},
			serde_json::Value::Array(arr) => {
				for (i, item) in arr.iter().enumerate() {
					let mut new_path = path.clone();
					new_path.push(i.to_string());
					self.collect_detections_recursive(item, new_path, results);
				}
			},
			serde_json::Value::Object(obj) => {
				for (key, val) in obj {
					let mut new_path = path.clone();
					new_path.push(key.clone());
					self.collect_detections_recursive(val, new_path, results);
				}
			},
			_ => {}, // Numbers, bools, nulls - skip
		}
	}

	/// Evaluate a JSON value for PII and return the appropriate decision
	fn evaluate_json(&self, json: &serde_json::Value, context: &GuardContext) -> GuardResult {
		let detections = self.collect_detections(json);

		if detections.is_empty() {
			return Ok(GuardDecision::Allow);
		}

		tracing::warn!(
				server = %context.server_name,
				detection_count = detections.len(),
				types = ?detections.iter().map(|d| &d.entity_type).collect::<Vec<_>>(),
				"PII detected in MCP message"
		);

		match self.config.action {
			PiiAction::Reject => {
				let message = self.config.rejection_message.clone().unwrap_or_else(|| {
					format!(
						"Request rejected: {} PII item(s) detected",
						detections.len()
					)
				});

				let details = serde_json::json!({
						"detections": detections.iter().map(|d| {
								serde_json::json!({
										"type": d.entity_type,
										"path": d.path.join("."),
										"score": d.score,
								})
						}).collect::<Vec<_>>()
				});

				Ok(GuardDecision::Deny(DenyReason {
					code: "pii_detected".to_string(),
					message,
					details: Some(details),
				}))
			},
			PiiAction::Mask => {
				// Return Modify decision with Transform action containing masked JSON
				let mut masked_json = json.clone();
				self.mask_json_value(&mut masked_json);

				Ok(GuardDecision::Modify(ModifyAction::Transform(masked_json)))
			},
		}
	}
}

#[derive(Debug)]
struct PiiDetection {
	path: Vec<String>,
	entity_type: String,
	score: f32,
}

impl NativeGuard for PiiGuard {
	fn evaluate_tools_list(
		&self,
		tools: &[rmcp::model::Tool],
		context: &GuardContext,
	) -> GuardResult {
		tracing::debug!(
				tool_count = tools.len(),
				server = %context.server_name,
				"PiiGuard::evaluate_tools_list called"
		);

		// For tools/list, we scan tool descriptions
		for tool in tools {
			// Scan tool description
			if let Some(desc) = &tool.description {
				let results = self.scan_text(desc.as_ref());
				if !results.is_empty() {
					match self.config.action {
						PiiAction::Reject => {
							return Ok(GuardDecision::Deny(DenyReason {
								code: "pii_in_tool_description".to_string(),
								message: format!("PII detected in tool '{}' description", tool.name),
								details: None,
							}));
						},
						PiiAction::Mask => {
							// For tools_list, we log warning but allow (can't modify the slice)
							tracing::warn!(
									tool = %tool.name,
									"PII detected in tool description (mask mode - allowing)"
							);
						},
					}
				}
			}
		}

		Ok(GuardDecision::Allow)
	}

	fn evaluate_tool_invoke(
		&self,
		tool_name: &str,
		arguments: &serde_json::Value,
		context: &GuardContext,
	) -> GuardResult {
		tracing::info!(
				tool = %tool_name,
				server = %context.server_name,
				arguments = %arguments,
				action = ?self.config.action,
				detect_types = ?self.config.detect,
				"PiiGuard::evaluate_tool_invoke called"
		);

		let result = self.evaluate_json(arguments, context);
		tracing::info!(result = ?result, "PiiGuard::evaluate_tool_invoke result");
		result
	}

	fn evaluate_request(&self, request: &serde_json::Value, context: &GuardContext) -> GuardResult {
		tracing::debug!(
				server = %context.server_name,
				"PiiGuard::evaluate_request called"
		);

		self.evaluate_json(request, context)
	}

	fn evaluate_response(&self, response: &serde_json::Value, context: &GuardContext) -> GuardResult {
		tracing::debug!(
				server = %context.server_name,
				"PiiGuard::evaluate_response called"
		);

		self.evaluate_json(response, context)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn create_test_context() -> GuardContext {
		GuardContext {
			server_name: "test-server".to_string(),
			identity: None,
			metadata: serde_json::json!({}),
		}
	}

	#[test]
	fn test_mask_email_in_json() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
				"message": "Contact me at test@example.com",
				"nested": {
						"email": "another@test.org"
				}
		});

		let result = guard.evaluate_request(&request, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				let msg = masked["message"].as_str().unwrap();
				assert!(
					msg.contains("<EMAIL_ADDRESS>"),
					"Expected masked email in message: {}",
					msg
				);
				assert!(
					!msg.contains("test@example.com"),
					"Expected email to be removed: {}",
					msg
				);

				let nested_email = masked["nested"]["email"].as_str().unwrap();
				assert!(
					nested_email.contains("<EMAIL_ADDRESS>"),
					"Expected masked email in nested: {}",
					nested_email
				);
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}

	#[test]
	fn test_reject_on_ssn() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Ssn],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: Some("SSN data not allowed".to_string()),
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
				"data": "My SSN is 123-45-6789"
		});

		let result = guard.evaluate_request(&request, &context);

		match result {
			Ok(GuardDecision::Deny(reason)) => {
				assert_eq!(reason.code, "pii_detected");
				assert!(reason.message.contains("SSN data not allowed"));
			},
			other => panic!("Expected Deny decision, got {:?}", other),
		}
	}

	#[test]
	fn test_allow_clean_request() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email, PiiType::PhoneNumber, PiiType::Ssn],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
				"query": "What is the weather today?",
				"location": "New York"
		});

		let result = guard.evaluate_request(&request, &context);
		assert!(matches!(result, Ok(GuardDecision::Allow)));
	}

	#[test]
	fn test_multiple_pii_types() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email, PiiType::PhoneNumber],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
				"email": "test@example.com",
				"phone": "(555) 123-4567"
		});

		let result = guard.evaluate_request(&request, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				assert!(masked["email"]
					.as_str()
					.unwrap()
					.contains("<EMAIL_ADDRESS>"));
				assert!(masked["phone"].as_str().unwrap().contains("<PHONE_NUMBER>"));
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}

	#[test]
	fn test_min_score_filtering() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Ssn],
			action: PiiAction::Reject,
			min_score: 0.6, // High threshold - weak SSN patterns won't trigger
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		// Weak SSN pattern (just 9 digits) has low confidence
		let request = serde_json::json!({
				"data": "ID: 123456789"
		});

		let result = guard.evaluate_request(&request, &context);
		// Should allow because the score is below threshold
		assert!(matches!(result, Ok(GuardDecision::Allow)));
	}

	#[test]
	fn test_config_deserialization() {
		let yaml = r#"
detect:
  - email
  - phone_number
  - credit_card
action: reject
min_score: 0.5
rejection_message: "PII not allowed in MCP requests"
"#;

		let config: PiiGuardConfig = serde_yaml::from_str(yaml).unwrap();
		assert_eq!(config.detect.len(), 3);
		assert_eq!(config.action, PiiAction::Reject);
		assert_eq!(config.min_score, 0.5);
		assert!(config.rejection_message.is_some());
	}

	#[test]
	fn test_default_config() {
		let config = PiiGuardConfig::default();
		assert_eq!(config.detect.len(), 6); // All PII types
		assert_eq!(config.action, PiiAction::Mask);
		assert_eq!(config.min_score, 0.3);
		assert!(config.rejection_message.is_none());
	}

	#[test]
	fn test_credit_card_detection() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::CreditCard],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: Some("Credit card not allowed".to_string()),
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		// Test various credit card formats
		let test_cases = vec![
			("4111111111111111", true),           // Visa - 16 digits no spaces
			("4111 1111 1111 1111", true),        // Visa - with spaces
			("4111-1111-1111-1111", true),        // Visa - with dashes
			("5500000000000004", true),           // Mastercard
			("371449635398431", true),            // Amex (15 digits, starts with 3)
			("6011111111111117", true),           // Discover
			("1234567890", false),                // Too short - should not match
			("hello world", false),               // No numbers
		];

		for (card, should_detect) in test_cases {
			let request = serde_json::json!({
				"card_number": card
			});

			let result = guard.evaluate_request(&request, &context);

			if should_detect {
				assert!(
					matches!(result, Ok(GuardDecision::Deny(_))),
					"Expected credit card '{}' to be detected and rejected, got {:?}",
					card, result
				);
			} else {
				assert!(
					matches!(result, Ok(GuardDecision::Allow)),
					"Expected '{}' to be allowed (no credit card), got {:?}",
					card, result
				);
			}
		}
	}

	#[test]
	fn test_array_scanning() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
				"contacts": [
						{"email": "first@example.com"},
						{"email": "second@example.com"}
				]
		});

		let result = guard.evaluate_request(&request, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				let contacts = masked["contacts"].as_array().unwrap();
				for contact in contacts {
					let email = contact["email"].as_str().unwrap();
					assert!(email.contains("<EMAIL_ADDRESS>"));
				}
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}

	#[test]
	fn test_url_detection() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Url],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let test_cases = vec![
			("https://example.com/path", true),
			("http://api.service.io/v1/data", true),
			("https://subdomain.example.org:8080/path?query=1", true),
			("just some text", false),
			("not a url at all", false),
		];

		for (url, should_detect) in test_cases {
			let request = serde_json::json!({
				"link": url
			});

			let result = guard.evaluate_request(&request, &context);

			if should_detect {
				assert!(
					matches!(result, Ok(GuardDecision::Modify(_))),
					"Expected URL '{}' to be detected and masked, got {:?}",
					url, result
				);
			} else {
				assert!(
					matches!(result, Ok(GuardDecision::Allow)),
					"Expected '{}' to be allowed (no URL), got {:?}",
					url, result
				);
			}
		}
	}

	#[test]
	fn test_phone_number_formats() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::PhoneNumber],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: Some("Phone numbers not allowed".to_string()),
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		// Test various phone formats (based on phonenumber library validation)
		let test_cases = vec![
			("(123) 456-7890", true),         // US format with parens
			("555-123-4567", true),           // US format with dashes
			("+1-800-555-1234", true),        // International US with dashes
			("555.123.4567", true),           // US format with dots
			("12345", false),                 // Too short
			("hello world", false),           // No numbers
		];

		for (phone, should_detect) in test_cases {
			let request = serde_json::json!({
				"contact": phone
			});

			let result = guard.evaluate_request(&request, &context);

			if should_detect {
				assert!(
					matches!(result, Ok(GuardDecision::Deny(_))),
					"Expected phone '{}' to be detected and rejected, got {:?}",
					phone, result
				);
			} else {
				assert!(
					matches!(result, Ok(GuardDecision::Allow)),
					"Expected '{}' to be allowed (no phone), got {:?}",
					phone, result
				);
			}
		}
	}

	#[test]
	fn test_canadian_sin_detection() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::CaSin],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: Some("Canadian SIN not allowed".to_string()),
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let test_cases = vec![
			("046-454-286", true),            // Formatted with dashes
			("046 454 286", true),            // Formatted with spaces
			("046454286", true),              // Unformatted 9 digits
			("12345", false),                 // Too short
			("hello world", false),           // No numbers
		];

		for (sin, should_detect) in test_cases {
			let request = serde_json::json!({
				"id": sin
			});

			let result = guard.evaluate_request(&request, &context);

			if should_detect {
				assert!(
					matches!(result, Ok(GuardDecision::Deny(_))),
					"Expected SIN '{}' to be detected and rejected, got {:?}",
					sin, result
				);
			} else {
				assert!(
					matches!(result, Ok(GuardDecision::Allow)),
					"Expected '{}' to be allowed (no SIN), got {:?}",
					sin, result
				);
			}
		}
	}

	#[test]
	fn test_tool_invoke_evaluation() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email, PiiType::Ssn],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let arguments = serde_json::json!({
			"user_email": "john.doe@company.com",
			"ssn": "123-45-6789",
			"query": "find user data"
		});

		let result = guard.evaluate_tool_invoke("search_tool", &arguments, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				assert!(
					masked["user_email"].as_str().unwrap().contains("<EMAIL_ADDRESS>"),
					"Expected email to be masked"
				);
				assert!(
					masked["ssn"].as_str().unwrap().contains("<SSN>"),
					"Expected SSN to be masked"
				);
				assert_eq!(
					masked["query"].as_str().unwrap(),
					"find user data",
					"Non-PII field should not be modified"
				);
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}

	#[test]
	fn test_tool_invoke_rejection() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::CreditCard],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: Some("Credit card data not allowed in tool calls".to_string()),
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let arguments = serde_json::json!({
			"payment_info": "4111111111111111"
		});

		let result = guard.evaluate_tool_invoke("process_payment", &arguments, &context);

		match result {
			Ok(GuardDecision::Deny(reason)) => {
				assert_eq!(reason.code, "pii_detected");
				assert!(reason.message.contains("Credit card data not allowed"));
			},
			other => panic!("Expected Deny decision, got {:?}", other),
		}
	}

	#[test]
	fn test_response_evaluation() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email, PiiType::PhoneNumber],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let response = serde_json::json!({
			"result": {
				"users": [
					{"name": "John", "email": "john@example.com", "phone": "(555) 111-2222"},
					{"name": "Jane", "email": "jane@example.com", "phone": "(555) 333-4444"}
				]
			}
		});

		let result = guard.evaluate_response(&response, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				let users = masked["result"]["users"].as_array().unwrap();
				for user in users {
					assert!(
						user["email"].as_str().unwrap().contains("<EMAIL_ADDRESS>"),
						"Expected email to be masked in response"
					);
					assert!(
						user["phone"].as_str().unwrap().contains("<PHONE_NUMBER>"),
						"Expected phone to be masked in response"
					);
				}
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}

	#[test]
	fn test_tools_list_pii_in_description_reject() {
		use rmcp::model::Tool;
		use std::borrow::Cow;
		use std::sync::Arc;

		let config = PiiGuardConfig {
			detect: vec![PiiType::Email],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let tool_with_pii = Tool {
			name: Cow::Owned("email_tool".to_string()),
			description: Some(Cow::Owned(
				"Contact support at admin@internal.company.com for help".to_string(),
			)),
			icons: None,
			title: None,
			meta: None,
			input_schema: Arc::new(
				serde_json::from_value(serde_json::json!({"type": "object"})).unwrap(),
			),
			annotations: None,
			output_schema: None,
		};

		let result = guard.evaluate_tools_list(&[tool_with_pii], &context);

		match result {
			Ok(GuardDecision::Deny(reason)) => {
				assert_eq!(reason.code, "pii_in_tool_description");
				assert!(reason.message.contains("email_tool"));
			},
			other => panic!("Expected Deny decision, got {:?}", other),
		}
	}

	#[test]
	fn test_tools_list_clean_descriptions() {
		use rmcp::model::Tool;
		use std::borrow::Cow;
		use std::sync::Arc;

		let config = PiiGuardConfig {
			detect: vec![PiiType::Email, PiiType::PhoneNumber, PiiType::Ssn],
			action: PiiAction::Reject,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let clean_tool = Tool {
			name: Cow::Owned("file_reader".to_string()),
			description: Some(Cow::Owned(
				"Reads files from the local filesystem".to_string(),
			)),
			icons: None,
			title: None,
			meta: None,
			input_schema: Arc::new(
				serde_json::from_value(serde_json::json!({"type": "object"})).unwrap(),
			),
			annotations: None,
			output_schema: None,
		};

		let result = guard.evaluate_tools_list(&[clean_tool], &context);
		assert!(matches!(result, Ok(GuardDecision::Allow)));
	}

	#[test]
	fn test_deeply_nested_pii() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
			"level1": {
				"level2": {
					"level3": {
						"level4": {
							"email": "deeply@nested.com"
						}
					}
				}
			}
		});

		let result = guard.evaluate_request(&request, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				let email = masked["level1"]["level2"]["level3"]["level4"]["email"]
					.as_str()
					.unwrap();
				assert!(
					email.contains("<EMAIL_ADDRESS>"),
					"Expected deeply nested email to be masked: {}",
					email
				);
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}

	#[test]
	fn test_mixed_pii_types_in_single_field() {
		let config = PiiGuardConfig {
			detect: vec![PiiType::Email, PiiType::PhoneNumber],
			action: PiiAction::Mask,
			min_score: 0.0,
			rejection_message: None,
		};

		let guard = PiiGuard::new(config);
		let context = create_test_context();

		let request = serde_json::json!({
			"message": "Contact me at user@example.com or call (555) 123-4567"
		});

		let result = guard.evaluate_request(&request, &context);

		match result {
			Ok(GuardDecision::Modify(ModifyAction::Transform(masked))) => {
				let msg = masked["message"].as_str().unwrap();
				assert!(
					msg.contains("<EMAIL_ADDRESS>"),
					"Expected email to be masked: {}",
					msg
				);
				assert!(
					msg.contains("<PHONE_NUMBER>"),
					"Expected phone to be masked: {}",
					msg
				);
				assert!(
					!msg.contains("user@example.com"),
					"Original email should be removed: {}",
					msg
				);
				assert!(
					!msg.contains("(555) 123-4567"),
					"Original phone should be removed: {}",
					msg
				);
			},
			other => panic!("Expected Modify decision, got {:?}", other),
		}
	}
}
