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
pub mod recognizer;
pub mod recognizer_result;
mod url_recognizer;
mod us_ssn_recognizer;

// Re-export commonly used types
pub use recognizer::Recognizer;
pub use recognizer_result::RecognizerResult;

/// PII types that can be detected using the LLM PII recognizers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum PiiType {
	/// Email addresses
	Email,
	/// Phone numbers (US, GB, DE, IL, IN, CA, BR)
	PhoneNumber,
	/// US Social Security Numbers
	Ssn,
	/// Credit card numbers (Visa, Mastercard, Amex, Discover, Diners Club)
	CreditCard,
	/// Canadian Social Insurance Numbers
	CaSin,
	/// URLs (http/https)
	Url,
}

impl PiiType {
	/// Returns all PII types
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

	/// Returns the recognizer for this PII type
	pub fn recognizer(&self) -> &'static (dyn Recognizer + Sync + Send) {
		match self {
			PiiType::Email => EMAIL.as_ref(),
			PiiType::PhoneNumber => PHONE.as_ref(),
			PiiType::Ssn => SSN.as_ref(),
			PiiType::CreditCard => CC.as_ref(),
			PiiType::CaSin => CA_SIN.as_ref(),
			PiiType::Url => URL.as_ref(),
		}
	}
}

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

#[allow(clippy::borrowed_box)]
pub fn recognizer(
	r: &Box<dyn Recognizer + Sync + Send + 'static>,
	text: &str,
) -> Vec<recognizer_result::RecognizerResult> {
	r.recognize(text)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
