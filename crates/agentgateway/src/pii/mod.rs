//! PII (Personally Identifiable Information) Detection Module
//!
//! This module provides shared PII detection infrastructure for all backend types
//! (LLM, MCP, A2A). It includes recognizers for common PII patterns and a unified
//! PiiType enum for configuration.
//!
//! # Supported PII Types
//! - Email addresses
//! - Phone numbers (US, GB, DE, IL, IN, CA, BR)
//! - US Social Security Numbers (SSN)
//! - Credit card numbers (Visa, Mastercard, Amex, Discover, Diners Club)
//! - Canadian Social Insurance Numbers (SIN)
//! - URLs

#![allow(dead_code)]

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use email_recognizer::EmailRecognizer;
use phone_recognizer::PhoneRecognizer;

mod ca_sin_recognizer;
mod credit_card_recognizer;
mod email_recognizer;
mod pattern_recognizer;
mod phone_recognizer;
mod recognizer;
mod recognizer_result;
mod url_recognizer;
mod us_ssn_recognizer;

// Re-export core types
pub use recognizer::Recognizer;
pub use recognizer_result::RecognizerResult;

/// PII types that can be detected
///
/// This enum is shared across all backend types (LLM, MCP, A2A) for consistent
/// PII detection configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PiiType {
	/// Email addresses (e.g., user@example.com)
	Email,
	/// Phone numbers (international formats supported)
	#[serde(alias = "phone_number")]
	PhoneNumber,
	/// US Social Security Numbers (e.g., 123-45-6789)
	Ssn,
	/// Credit card numbers (Visa, Mastercard, Amex, Discover, Diners Club)
	CreditCard,
	/// Canadian Social Insurance Numbers (e.g., 123-456-789)
	CaSin,
	/// URLs (http, https, and common TLDs)
	Url,
}

impl PiiType {
	/// Get the recognizer for this PII type
	pub fn recognizer(&self) -> &'static (dyn Recognizer + Sync + Send) {
		match self {
			PiiType::Email => &**EMAIL,
			PiiType::PhoneNumber => &**PHONE,
			PiiType::Ssn => &**SSN,
			PiiType::CreditCard => &**CC,
			PiiType::CaSin => &**CA_SIN,
			PiiType::Url => &**URL,
		}
	}

	/// Get all available PII types
	pub fn all() -> Vec<PiiType> {
		vec![
			PiiType::Email,
			PiiType::PhoneNumber,
			PiiType::Ssn,
			PiiType::CreditCard,
			PiiType::CaSin,
			PiiType::Url,
		]
	}
}

// Lazy-initialized singleton recognizers
pub static EMAIL: Lazy<Box<dyn Recognizer + Sync + Send + 'static>> =
	Lazy::new(|| Box::new(EmailRecognizer::new()));

pub static PHONE: Lazy<Box<dyn Recognizer + Sync + Send + 'static>> =
	Lazy::new(|| Box::new(PhoneRecognizer::new()));

pub static CC: Lazy<Box<dyn Recognizer + Sync + Send + 'static>> =
	Lazy::new(|| Box::new(credit_card_recognizer::CreditCardRecognizer::new()));

pub static SSN: Lazy<Box<dyn Recognizer + Sync + Send + 'static>> =
	Lazy::new(|| Box::new(us_ssn_recognizer::UsSsnRecognizer::new()));

pub static CA_SIN: Lazy<Box<dyn Recognizer + Sync + Send + 'static>> =
	Lazy::new(|| Box::new(ca_sin_recognizer::CaSinRecognizer::new()));

pub static URL: Lazy<Box<dyn Recognizer + Sync + Send + 'static>> =
	Lazy::new(|| Box::new(url_recognizer::UrlRecognizer::new()));

/// Convenience function to run a recognizer on text
#[allow(clippy::borrowed_box)]
pub fn recognize(
	r: &Box<dyn Recognizer + Sync + Send + 'static>,
	text: &str,
) -> Vec<RecognizerResult> {
	r.recognize(text)
}

/// Scan text for specific PII types and return all matches
pub fn scan_text(text: &str, types: &[PiiType]) -> Vec<RecognizerResult> {
	let mut results = Vec::new();
	for pii_type in types {
		results.extend(pii_type.recognizer().recognize(text));
	}
	results
}

/// Scan text for all PII types and return all matches
pub fn scan_all(text: &str) -> Vec<RecognizerResult> {
	scan_text(text, &PiiType::all())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
