// MCP Security Guards Framework
//
// This module provides a pluggable security guard system for MCP protocol operations.
// Guards can inspect and modify requests/responses to detect and prevent security threats
// specific to the Model Context Protocol.
//
// Architecture:
// - Native guards: Compiled into binary, fastest performance (< 1ms latency)
// - WASM guards: Loaded at runtime, good performance (~5-10ms latency)
// - External guards: Webhook/gRPC services for complex analysis

use serde::{Deserialize, Serialize};
use std::time::Duration;

pub mod native;
pub mod wasm;

// Re-export core types
pub use native::{
	PiiGuard, RugPullDetector, ServerWhitelistChecker, ToolPoisoningDetector, ToolShadowingDetector,
};

/// Security guard that can be applied to MCP protocol operations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct McpSecurityGuard {
	/// Unique identifier for this guard
	pub id: String,

	/// Human-readable description
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,

	/// Execution priority (lower = runs first)
	#[serde(default = "default_priority")]
	pub priority: u32,

	/// Behavior when guard fails to execute
	#[serde(default)]
	pub failure_mode: FailureMode,

	/// Maximum time allowed for guard execution
	#[serde(default = "default_timeout")]
	pub timeout_ms: u64,

	/// Which phases this guard runs on
	#[serde(default)]
	pub runs_on: Vec<GuardPhase>,

	/// Whether guard is enabled
	#[serde(default = "default_enabled")]
	pub enabled: bool,

	/// The specific guard implementation
	#[serde(flatten)]
	pub kind: McpGuardKind,
}

fn default_priority() -> u32 {
	100
}

fn default_timeout() -> u64 {
	100
}

fn default_enabled() -> bool {
	true
}

/// Guard implementation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpGuardKind {
	/// Tool Poisoning Detection (native)
	ToolPoisoning(native::ToolPoisoningConfig),

	/// Rug Pull Detection (native)
	RugPull(native::RugPullConfig),

	/// Tool Shadowing Prevention (native)
	ToolShadowing(native::ToolShadowingConfig),

	/// Server Whitelist Enforcement (native)
	ServerWhitelist(native::ServerWhitelistConfig),
	/// PII Detection and Masking (native)
	Pii(native::PiiGuardConfig),

	/// Custom WASM module
	#[cfg(feature = "wasm-guards")]
	Wasm(wasm::WasmGuardConfig),
}

/// Execution phase for guards
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum GuardPhase {
	/// Before establishing connection to MCP server
	/// Used for server whitelisting, typosquat detection, TLS validation
	Connection,

	/// Before forwarding client request to MCP server
	#[default]
	Request,

	/// After receiving response from MCP server
	Response,

	/// Specifically for tools/list responses
	ToolsList,

	/// Specifically for tool invocations (tools/call)
	ToolInvoke,
}

/// How to behave when guard execution fails (timeout, error, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FailureMode {
	/// Block request on failure (secure default)
	#[default]
	FailClosed,

	/// Allow request on failure (availability over security)
	FailOpen,
}

/// Decision made by a security guard
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardDecision {
	/// Allow the operation to proceed
	Allow,

	/// Block the operation
	Deny(DenyReason),

	/// Modify the request/response
	Modify(ModifyAction),
}

/// Reason for denying an operation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenyReason {
	/// Short reason code (e.g., "tool_poisoning_detected")
	pub code: String,

	/// Human-readable message
	pub message: String,

	/// Optional details for debugging/auditing
	#[serde(skip_serializing_if = "Option::is_none")]
	pub details: Option<serde_json::Value>,
}

/// Action to modify request/response
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModifyAction {
	/// Mask sensitive data in response
	MaskFields(Vec<String>),

	/// Add warning headers
	AddWarning(String),

	/// Transform content
	Transform(serde_json::Value),
}

/// Context provided to guards for evaluation
#[derive(Debug, Clone)]
pub struct GuardContext {
	/// Server/target name
	pub server_name: String,

	/// Optional session/user identity
	pub identity: Option<String>,

	/// Request metadata
	pub metadata: serde_json::Value,
}

/// Result of guard execution
pub type GuardResult = Result<GuardDecision, GuardError>;

/// Errors that can occur during guard execution
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
	#[error("Guard execution timeout after {0:?}")]
	Timeout(Duration),

	#[error("Guard execution error: {0}")]
	ExecutionError(String),

	#[error("Guard configuration error: {0}")]
	ConfigError(String),

	#[error("WASM module error: {0}")]
	#[cfg(feature = "wasm-guards")]
	WasmError(String),
}

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for shared GuardExecutor instances, keyed by backend name.
/// This enables hot-reload of security guards across existing SSE sessions.
#[derive(Clone, Default)]
pub struct GuardExecutorRegistry {
	executors: Arc<RwLock<HashMap<String, Arc<GuardExecutor>>>>,
}

impl std::fmt::Debug for GuardExecutorRegistry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let executors = self.executors.read().expect("registry lock poisoned");
		f.debug_struct("GuardExecutorRegistry")
			.field("backend_count", &executors.len())
			.field("backends", &executors.keys().collect::<Vec<_>>())
			.finish()
	}
}

impl GuardExecutorRegistry {
	/// Create a new empty registry
	pub fn new() -> Self {
		Self {
			executors: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Get or create a GuardExecutor for a backend.
	/// If the executor already exists, returns the existing one.
	/// If not, creates a new one from the provided config.
	pub fn get_or_create(
		&self,
		backend_name: &str,
		configs: Vec<McpSecurityGuard>,
	) -> Result<Arc<GuardExecutor>, GuardError> {
		// First try read lock to check if exists
		{
			let executors = self.executors.read().expect("registry lock poisoned");
			if let Some(executor) = executors.get(backend_name) {
				return Ok(executor.clone());
			}
		}

		// Need to create - acquire write lock
		let mut executors = self.executors.write().expect("registry lock poisoned");

		// Double-check in case another thread created it
		if let Some(executor) = executors.get(backend_name) {
			return Ok(executor.clone());
		}

		// Create new executor
		let executor = Arc::new(GuardExecutor::new(configs)?);
		executors.insert(backend_name.to_string(), executor.clone());
		tracing::info!(backend = %backend_name, "Created new GuardExecutor in registry");
		Ok(executor)
	}

	/// Update guards for a specific backend.
	/// If the executor exists, updates it in place (affecting all existing sessions).
	/// If not, creates a new one.
	pub fn update_backend(
		&self,
		backend_name: &str,
		configs: Vec<McpSecurityGuard>,
	) -> Result<(), GuardError> {
		let executors = self.executors.read().expect("registry lock poisoned");

		if let Some(executor) = executors.get(backend_name) {
			// Update existing executor - this propagates to all sessions using it
			executor.update(configs)?;
			tracing::info!(backend = %backend_name, "Updated GuardExecutor via hot-reload");
		} else {
			// No existing executor - create one on next request
			drop(executors);
			let mut executors = self.executors.write().expect("registry lock poisoned");
			let executor = Arc::new(GuardExecutor::new(configs)?);
			executors.insert(backend_name.to_string(), executor);
			tracing::info!(backend = %backend_name, "Created new GuardExecutor during hot-reload");
		}
		Ok(())
	}

	/// Remove a backend's executor from the registry.
	/// Called when a backend is removed from config.
	pub fn remove_backend(&self, backend_name: &str) {
		let mut executors = self.executors.write().expect("registry lock poisoned");
		if executors.remove(backend_name).is_some() {
			tracing::info!(backend = %backend_name, "Removed GuardExecutor from registry");
		}
	}

	/// Get a list of all backend names with registered executors
	pub fn backend_names(&self) -> Vec<String> {
		let executors = self.executors.read().expect("registry lock poisoned");
		executors.keys().cloned().collect()
	}

	/// Collect schemas from all WASM guards across all backends.
	/// Returns a map of guard_id -> (settings_schema_json, default_config_json).
	pub fn collect_wasm_schemas(&self) -> HashMap<String, WasmGuardSchema> {
		let executors = self.executors.read().expect("registry lock poisoned");
		let mut schemas = HashMap::new();

		for (_backend_name, executor) in executors.iter() {
			for entry in executor.collect_guard_schemas() {
				schemas.insert(entry.0, entry.1);
			}
		}

		schemas
	}
}

/// Schema information returned by a WASM guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmGuardSchema {
	/// JSON Schema describing guard's configurable parameters
	pub settings_schema: serde_json::Value,
	/// Default configuration values
	pub default_config: serde_json::Value,
}

/// Guard executor that manages and executes security guards in priority order
#[derive(Clone)]
pub struct GuardExecutor {
	/// Guards are stored behind RwLock to support hot-reload of config
	guards: Arc<RwLock<Vec<InitializedGuard>>>,
}

struct InitializedGuard {
	config: McpSecurityGuard,
	guard: Arc<dyn native::NativeGuard>,
}

/// Initialize guards from config (shared logic for new() and update())
fn initialize_guards(configs: Vec<McpSecurityGuard>) -> Result<Vec<InitializedGuard>, GuardError> {
	tracing::info!(
		config_count = configs.len(),
		"Initializing guards from config"
	);
	let mut guards = Vec::new();

	for config in configs {
		tracing::info!(
			guard_id = %config.id,
			guard_type = ?std::mem::discriminant(&config.kind),
			enabled = config.enabled,
			runs_on = ?config.runs_on,
			"Processing guard config"
		);
		if !config.enabled {
			tracing::info!(guard_id = %config.id, "Guard disabled, skipping");
			continue;
		}

		let guard: Arc<dyn native::NativeGuard> = match &config.kind {
			McpGuardKind::ToolPoisoning(cfg) => {
				Arc::new(native::ToolPoisoningDetector::new(cfg.clone())?)
			},
			McpGuardKind::RugPull(cfg) => Arc::new(native::RugPullDetector::new(cfg.clone())),
			McpGuardKind::ToolShadowing(cfg) => Arc::new(native::ToolShadowingDetector::new(cfg.clone())),
			McpGuardKind::ServerWhitelist(cfg) => {
				Arc::new(native::ServerWhitelistChecker::new(cfg.clone()))
			},
			McpGuardKind::Pii(cfg) => Arc::new(native::PiiGuard::new(cfg.clone())),
			#[cfg(feature = "wasm-guards")]
			McpGuardKind::Wasm(cfg) => Arc::new(wasm::WasmGuard::new(config.id.clone(), cfg.clone())?),
		};

		guards.push(InitializedGuard {
			config: config.clone(),
			guard,
		});
	}

	// Sort by priority (lower = higher priority)
	guards.sort_by_key(|g| g.config.priority);

	Ok(guards)
}

impl GuardExecutor {
	/// Create a new GuardExecutor from a list of guard configurations
	pub fn new(configs: Vec<McpSecurityGuard>) -> Result<Self, GuardError> {
		let guards = initialize_guards(configs)?;
		Ok(Self {
			guards: Arc::new(RwLock::new(guards)),
		})
	}

	/// Create an empty executor with no guards
	pub fn empty() -> Self {
		Self {
			guards: Arc::new(RwLock::new(Vec::new())),
		}
	}

	/// Returns true if any guards are configured
	pub fn has_guards(&self) -> bool {
		let guards = self.guards.read().expect("guards lock poisoned");
		!guards.is_empty()
	}

	/// Update guards with new configuration (hot-reload support)
	/// This replaces all guards atomically
	pub fn update(&self, configs: Vec<McpSecurityGuard>) -> Result<(), GuardError> {
		let new_guards = initialize_guards(configs)?;
		let mut guards = self.guards.write().expect("guards lock poisoned");
		*guards = new_guards;
		tracing::info!("Security guards updated via hot-reload");
		Ok(())
	}

	/// Execute guards before establishing connection to an MCP server
	/// Used for server whitelisting, typosquat detection, TLS validation
	pub fn evaluate_connection(
		&self,
		server_name: &str,
		server_url: Option<&str>,
		context: &GuardContext,
	) -> GuardResult {
		let guards = self.guards.read().expect("guards lock poisoned");
		tracing::info!(
			guard_count = guards.len(),
			server = %server_name,
			server_url = ?server_url,
			"GuardExecutor::evaluate_connection called"
		);
		for guard_entry in guards.iter() {
			// Only run guards configured for Connection phase
			if !guard_entry.config.runs_on.contains(&GuardPhase::Connection) {
				continue;
			}

			// Execute guard with timeout
			let result = self.execute_with_timeout(
				|| {
					guard_entry
						.guard
						.evaluate_connection(server_name, server_url, context)
				},
				Duration::from_millis(guard_entry.config.timeout_ms),
				&guard_entry.config,
			);

			// Handle result based on failure mode
			match result {
				Ok(GuardDecision::Allow) => continue,
				Ok(decision) => return Ok(decision),
				Err(e) => match guard_entry.config.failure_mode {
					FailureMode::FailClosed => {
						return Err(GuardError::ExecutionError(format!(
							"Guard {} failed: {}",
							guard_entry.config.id, e
						)));
					},
					FailureMode::FailOpen => {
						tracing::warn!(
							"Guard {} failed but continuing due to fail_open: {}",
							guard_entry.config.id,
							e
						);
						continue;
					},
				},
			}
		}

		Ok(GuardDecision::Allow)
	}

	/// Execute guards on a tools/list response
	pub fn evaluate_tools_list(
		&self,
		tools: &[rmcp::model::Tool],
		context: &GuardContext,
	) -> GuardResult {
		let guards = self.guards.read().expect("guards lock poisoned");
		tracing::info!(
			guard_count = guards.len(),
			tool_count = tools.len(),
			server = %context.server_name,
			"GuardExecutor::evaluate_tools_list called"
		);
		for guard_entry in guards.iter() {
			// Only run guards configured for ToolsList or Response phase
			if !guard_entry.config.runs_on.contains(&GuardPhase::ToolsList)
				&& !guard_entry.config.runs_on.contains(&GuardPhase::Response)
			{
				continue;
			}

			// Execute guard with timeout
			let result = self.execute_with_timeout(
				|| guard_entry.guard.evaluate_tools_list(tools, context),
				Duration::from_millis(guard_entry.config.timeout_ms),
				&guard_entry.config,
			);

			// Handle result based on failure mode
			match result {
				Ok(GuardDecision::Allow) => continue,
				Ok(decision) => return Ok(decision),
				Err(e) => match guard_entry.config.failure_mode {
					FailureMode::FailClosed => {
						return Err(GuardError::ExecutionError(format!(
							"Guard {} failed: {}",
							guard_entry.config.id, e
						)));
					},
					FailureMode::FailOpen => {
						tracing::warn!(
							"Guard {} failed but continuing due to fail_open: {}",
							guard_entry.config.id,
							e
						);
						continue;
					},
				},
			}
		}

		Ok(GuardDecision::Allow)
	}

	/// Execute guards on a tool invocation (tools/call)
	pub fn evaluate_tool_invoke(
		&self,
		tool_name: &str,
		arguments: &serde_json::Value,
		context: &GuardContext,
	) -> GuardResult {
		let guards = self.guards.read().expect("guards lock poisoned");
		tracing::info!(
			guard_count = guards.len(),
			tool = %tool_name,
			server = %context.server_name,
			arguments = %arguments,
			"GuardExecutor::evaluate_tool_invoke called"
		);
		for guard_entry in guards.iter() {
			tracing::info!(
				guard_id = %guard_entry.config.id,
				runs_on = ?guard_entry.config.runs_on,
				"Checking guard for tool_invoke"
			);
			// Only run guards configured for ToolInvoke or Request phase
			if !guard_entry.config.runs_on.contains(&GuardPhase::ToolInvoke)
				&& !guard_entry.config.runs_on.contains(&GuardPhase::Request)
			{
				tracing::info!(guard_id = %guard_entry.config.id, "Guard skipped - runs_on doesn't include tool_invoke/request");
				continue;
			}

			// Execute guard with timeout
			let result = self.execute_with_timeout(
				|| {
					guard_entry
						.guard
						.evaluate_tool_invoke(tool_name, arguments, context)
				},
				Duration::from_millis(guard_entry.config.timeout_ms),
				&guard_entry.config,
			);

			// Handle result based on failure mode
			match result {
				Ok(GuardDecision::Allow) => continue,
				Ok(decision) => return Ok(decision),
				Err(e) => match guard_entry.config.failure_mode {
					FailureMode::FailClosed => {
						return Err(GuardError::ExecutionError(format!(
							"Guard {} failed: {}",
							guard_entry.config.id, e
						)));
					},
					FailureMode::FailOpen => {
						tracing::warn!(
							"Guard {} failed but continuing due to fail_open: {}",
							guard_entry.config.id,
							e
						);
						continue;
					},
				},
			}
		}

		Ok(GuardDecision::Allow)
	}

	/// Execute guards on a response
	pub fn evaluate_response(
		&self,
		response: &serde_json::Value,
		context: &GuardContext,
	) -> GuardResult {
		let guards = self.guards.read().expect("guards lock poisoned");
		tracing::debug!(
			guard_count = guards.len(),
			server = %context.server_name,
			"GuardExecutor::evaluate_response called"
		);
		for guard_entry in guards.iter() {
			// Only run guards configured for Response phase
			if !guard_entry.config.runs_on.contains(&GuardPhase::Response) {
				continue;
			}

			// Execute guard with timeout
			let result = self.execute_with_timeout(
				|| guard_entry.guard.evaluate_response(response, context),
				Duration::from_millis(guard_entry.config.timeout_ms),
				&guard_entry.config,
			);

			// Handle result based on failure mode
			match result {
				Ok(GuardDecision::Allow) => continue,
				Ok(decision) => return Ok(decision),
				Err(e) => match guard_entry.config.failure_mode {
					FailureMode::FailClosed => {
						return Err(GuardError::ExecutionError(format!(
							"Guard {} failed: {}",
							guard_entry.config.id, e
						)));
					},
					FailureMode::FailOpen => {
						tracing::warn!(
							"Guard {} failed but continuing due to fail_open: {}",
							guard_entry.config.id,
							e
						);
						continue;
					},
				},
			}
		}

		Ok(GuardDecision::Allow)
	}

	fn execute_with_timeout<F>(
		&self,
		f: F,
		_timeout: Duration,
		_config: &McpSecurityGuard,
	) -> GuardResult
	where
		F: FnOnce() -> GuardResult,
	{
		// TODO: Implement actual timeout mechanism using tokio::time::timeout
		// For now, just execute synchronously
		f()
	}

	/// Collect schemas from guards that support dynamic schema export (WASM guards).
	/// Returns a list of (guard_id, WasmGuardSchema) pairs.
	pub fn collect_guard_schemas(&self) -> Vec<(String, WasmGuardSchema)> {
		let guards = self.guards.read().expect("guards lock poisoned");
		let mut schemas = Vec::new();

		for guard_entry in guards.iter() {
			if let Some(schema_json) = guard_entry.guard.get_settings_schema() {
				let settings_schema: serde_json::Value =
					serde_json::from_str(&schema_json).unwrap_or(serde_json::Value::Null);

				let default_config: serde_json::Value = guard_entry
					.guard
					.get_default_config()
					.and_then(|s| serde_json::from_str(&s).ok())
					.unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

				schemas.push((
					guard_entry.config.id.clone(),
					WasmGuardSchema {
						settings_schema,
						default_config,
					},
				));
			}
		}

		schemas
	}

	/// Reset state for a server (called on session re-initialization)
	/// This clears any per-server state like baselines in guards.
	pub fn reset_server(&self, server_name: &str) {
		let guards = self.guards.read().expect("guards lock poisoned");
		for guard_entry in guards.iter() {
			guard_entry.guard.reset_server(server_name);
		}
		tracing::debug!(
			server = %server_name,
			guard_count = guards.len(),
			"Reset server state across all guards"
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_guard_deserialization() {
		let yaml = r#"
id: test-guard
priority: 100
failure_mode: fail_closed
timeout_ms: 50
runs_on:
  - response
type: tool_poisoning
strict_mode: true
custom_patterns:
  - "(?i)SYSTEM:\\s*override"
"#;

		let guard: McpSecurityGuard = serde_yaml::from_str(yaml).unwrap();
		assert_eq!(guard.id, "test-guard");
		assert_eq!(guard.priority, 100);
		assert_eq!(guard.timeout_ms, 50);
		assert!(matches!(guard.kind, McpGuardKind::ToolPoisoning(_)));
	}

	#[test]
	fn test_pii_guard_deserialization() {
		let yaml = r#"
id: pii-guard
priority: 50
runs_on:
  - request
  - response
  - tool_invoke
type: pii
detect:
  - email
  - credit_card
action: reject
"#;

		let guard: McpSecurityGuard = serde_yaml::from_str(yaml).unwrap();
		assert_eq!(guard.id, "pii-guard");
		assert_eq!(guard.priority, 50);
		assert_eq!(guard.runs_on.len(), 3);
		assert!(guard.runs_on.contains(&GuardPhase::Request));
		assert!(guard.runs_on.contains(&GuardPhase::Response));
		assert!(guard.runs_on.contains(&GuardPhase::ToolInvoke));

		match guard.kind {
			McpGuardKind::Pii(config) => {
				assert_eq!(config.detect.len(), 2);
				assert!(config.detect.contains(&native::PiiType::Email));
				assert!(config.detect.contains(&native::PiiType::CreditCard));
				assert_eq!(config.action, native::PiiAction::Reject);
			},
			_ => panic!("Expected Pii guard kind"),
		}
	}
}
