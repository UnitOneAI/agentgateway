// WASM Guard Loader
//
// Loads and executes security guards compiled to WebAssembly using wasmtime.
// This allows runtime loading of custom guards without recompiling the gateway.
//
// Guards implement the WIT interface defined in examples/wasm-guards/simple-pattern-guard/wit/guard.wit

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::native::NativeGuard;
use super::{DenyReason, GuardContext, GuardDecision, GuardError, GuardResult, ModifyAction};

#[cfg(feature = "wasm-guards")]
use wasmtime::component::{Component, Linker, Val};
#[cfg(feature = "wasm-guards")]
use wasmtime::{Config, Engine, Store};
#[cfg(feature = "wasm-guards")]
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

/// Configuration for WASM-based guards
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct WasmGuardConfig {
    /// Path to WASM component file
    pub module_path: String,

    /// Maximum memory for WASM instance (bytes)
    #[serde(default = "default_max_memory")]
    pub max_memory: usize,

    /// Maximum WebAssembly stack size (bytes).
    /// Python WASM components require significantly more stack space (2-4 MB)
    /// due to the embedded Python interpreter.
    /// Default: 2 MB (sufficient for most Python guards)
    #[serde(default = "default_max_wasm_stack")]
    pub max_wasm_stack: usize,

    /// Timeout for guard execution (milliseconds)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Configuration values passed to the WASM guard via get_config()
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

fn default_max_memory() -> usize {
    10 * 1024 * 1024 // 10 MB
}

fn default_max_wasm_stack() -> usize {
    2 * 1024 * 1024 // 2 MB - sufficient for Python WASM guards
}

fn default_timeout_ms() -> u64 {
    100
}

/// Run a closure on a thread with a large stack.
/// Python WASM components require significant native stack space that exceeds
/// the default thread stack size, especially on Windows where the main thread
/// stack cannot be grown dynamically.
/// Uses scoped threads to avoid 'static lifetime requirements.
#[cfg(feature = "wasm-guards")]
fn run_with_large_stack<F, T>(stack_size: usize, f: F) -> T
where
    F: FnOnce() -> T + Send,
    T: Send,
{
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                // Grow the stack on this thread before executing
                stacker::grow(stack_size, f)
            })
            .join()
            .expect("WASM thread panicked")
    })
}

/// State stored in the wasmtime Store for host functions
#[cfg(feature = "wasm-guards")]
struct WasmState {
    /// Configuration values accessible via get_config()
    config: HashMap<String, serde_json::Value>,
    /// WASI context for WASI imports
    wasi: WasiCtx,
    /// Resource table for component model resources
    table: wasmtime::component::ResourceTable,
}

#[cfg(feature = "wasm-guards")]
impl WasmState {
    fn new(config: HashMap<String, serde_json::Value>) -> Self {
        let wasi = WasiCtxBuilder::new()
            .inherit_stdout()
            .inherit_stderr()
            .build();
        Self {
            config,
            wasi,
            table: wasmtime::component::ResourceTable::new(),
        }
    }
}

#[cfg(feature = "wasm-guards")]
impl WasiView for WasmState {
    fn table(&mut self) -> &mut wasmtime::component::ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

/// WASM Guard implementation using wasmtime
#[cfg(feature = "wasm-guards")]
pub struct WasmGuard {
    guard_id: String,
    engine: Engine,
    component: Component,
    config: WasmGuardConfig,
}

#[cfg(feature = "wasm-guards")]
impl WasmGuard {
    /// Create a new WASM guard from config
    pub fn new(guard_id: String, config: WasmGuardConfig) -> Result<Self, GuardError> {
        // Validate config
        if config.module_path.is_empty() {
            return Err(GuardError::ConfigError(
                "module_path cannot be empty".to_string(),
            ));
        }

        // Expand shell paths like ~ and environment variables
        let expanded_path = shellexpand::full(&config.module_path)
            .map_err(|e| GuardError::ConfigError(format!("Failed to expand path: {}", e)))?;

        // Check if file exists
        if !std::path::Path::new(expanded_path.as_ref()).exists() {
            return Err(GuardError::ConfigError(format!(
                "WASM module not found: {}",
                expanded_path
            )));
        }

        // Configure wasmtime engine
        let mut engine_config = Config::new();
        engine_config.wasm_component_model(true);
        // Set maximum WASM stack size - Python WASM components require larger stacks
        // due to the embedded interpreter
        engine_config.max_wasm_stack(config.max_wasm_stack);

        let engine = Engine::new(&engine_config).map_err(|e| {
            GuardError::WasmError(format!("Failed to create wasmtime engine: {}", e))
        })?;

        // Load and compile the WASM component
        // Python WASM components require significant native stack space during compilation
        // due to the embedded interpreter. On Windows, the main thread stack cannot be grown,
        // so we spawn a dedicated thread with a large stack (8MB) for compilation.
        let path_for_thread = expanded_path.to_string();
        let engine_clone = engine.clone();
        let component = run_with_large_stack(8 * 1024 * 1024, move || {
            Component::from_file(&engine_clone, &path_for_thread)
        })
        .map_err(|e| GuardError::WasmError(format!("Failed to load WASM component: {}", e)))?;

        tracing::info!(
            guard_id = %guard_id,
            module_path = %config.module_path,
            "Loaded WASM guard component"
        );

        Ok(Self {
            guard_id,
            engine,
            component,
            config,
        })
    }

    /// Create a linker with host function imports
    fn create_linker(&self) -> Result<Linker<WasmState>, GuardError> {
        let mut linker = Linker::new(&self.engine);

        // Add WASI support to the linker
        wasmtime_wasi::add_to_linker_sync(&mut linker)
            .map_err(|e| GuardError::WasmError(format!("Failed to add WASI to linker: {}", e)))?;

        // Define the host interface functions
        // Package: mcp:security-guard/host@0.1.0
        let mut root = linker.root();
        let mut instance = root
            .instance("mcp:security-guard/host@0.1.0")
            .map_err(|e| GuardError::WasmError(format!("Failed to create host instance: {}", e)))?;

        // log(level: u8, message: string)
        instance
            .func_wrap("log", |_store: wasmtime::StoreContextMut<WasmState>, (level, message): (u8, String)| {
                match level {
                    0 => tracing::trace!(wasm_guard = true, "{}", message),
                    1 => tracing::debug!(wasm_guard = true, "{}", message),
                    2 => tracing::info!(wasm_guard = true, "{}", message),
                    3 => tracing::warn!(wasm_guard = true, "{}", message),
                    4 => tracing::error!(wasm_guard = true, "{}", message),
                    _ => tracing::info!(wasm_guard = true, "{}", message),
                }
                Ok(())
            })
            .map_err(|e| GuardError::WasmError(format!("Failed to wrap log function: {}", e)))?;

        // get-time() -> u64
        instance
            .func_wrap("get-time", |_store: wasmtime::StoreContextMut<WasmState>, ()| -> Result<(u64,), wasmtime::Error> {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO);
                Ok((now.as_millis() as u64,))
            })
            .map_err(|e| GuardError::WasmError(format!("Failed to wrap get-time function: {}", e)))?;

        // get-config(key: string) -> string
        instance
            .func_wrap("get-config", |store: wasmtime::StoreContextMut<WasmState>, (key,): (String,)| -> Result<(String,), wasmtime::Error> {
                let value = store.data()
                    .config
                    .get(&key)
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                Ok((value,))
            })
            .map_err(|e| GuardError::WasmError(format!("Failed to wrap get-config function: {}", e)))?;

        Ok(linker)
    }

    /// Parse WIT decision result into GuardDecision
    fn parse_decision(result: &[Val]) -> Result<GuardDecision, GuardError> {
        // The result should be a single Result<decision, string> value
        if result.is_empty() {
            return Err(GuardError::WasmError(
                "Empty result from WASM guard".to_string(),
            ));
        }

        // Handle the Result type
        match &result[0] {
            Val::Result(res) => match res {
                Ok(Some(decision_val)) => Self::parse_decision_variant(decision_val),
                Ok(None) => {
                    // Result<_, _>::Ok(unit) - treat as Allow
                    Ok(GuardDecision::Allow)
                }
                Err(Some(error_val)) => {
                    if let Val::String(s) = error_val.as_ref() {
                        Err(GuardError::WasmError(s.to_string()))
                    } else {
                        Err(GuardError::WasmError(
                            "Unknown error from WASM guard".to_string(),
                        ))
                    }
                }
                Err(None) => Err(GuardError::WasmError(
                    "Unknown error from WASM guard".to_string(),
                )),
            },
            other => Err(GuardError::WasmError(format!(
                "Unexpected return type from WASM guard: {:?}",
                other
            ))),
        }
    }

    /// Parse the decision variant
    fn parse_decision_variant(val: &Val) -> Result<GuardDecision, GuardError> {
        match val {
            Val::Variant(name, payload) => match name.as_str() {
                "allow" => Ok(GuardDecision::Allow),
                "deny" => {
                    if let Some(reason_val) = payload {
                        Self::parse_deny_reason(reason_val)
                    } else {
                        Ok(GuardDecision::Deny(DenyReason {
                            code: "wasm_denied".to_string(),
                            message: "Denied by WASM guard".to_string(),
                            details: None,
                        }))
                    }
                }
                "modify" => {
                    if let Some(Val::String(json)) = payload.as_deref() {
                        let transform: serde_json::Value =
                            serde_json::from_str(json).unwrap_or(serde_json::Value::Null);
                        Ok(GuardDecision::Modify(ModifyAction::Transform(transform)))
                    } else {
                        Ok(GuardDecision::Modify(ModifyAction::Transform(
                            serde_json::Value::Null,
                        )))
                    }
                }
                "warn" => {
                    // Warn means allow but log the warnings
                    if let Some(Val::List(warnings)) = payload.as_deref() {
                        for warning in warnings {
                            if let Val::String(msg) = warning {
                                tracing::warn!(
                                    warning = %msg,
                                    "WASM guard returned warning"
                                );
                            }
                        }
                    }
                    Ok(GuardDecision::Allow)
                }
                _ => Err(GuardError::WasmError(format!(
                    "Unknown decision variant: {}",
                    name
                ))),
            },
            _ => Err(GuardError::WasmError(format!(
                "Expected variant, got: {:?}",
                val
            ))),
        }
    }

    /// Parse deny reason from WIT record
    fn parse_deny_reason(val: &Val) -> Result<GuardDecision, GuardError> {
        match val {
            Val::Record(fields) => {
                let mut code = "wasm_denied".to_string();
                let mut message = "Denied by WASM guard".to_string();
                let mut details: Option<serde_json::Value> = None;

                for (name, field_val) in fields.iter() {
                    match name.as_str() {
                        "code" => {
                            if let Val::String(s) = field_val {
                                code = s.to_string();
                            }
                        }
                        "message" => {
                            if let Val::String(s) = field_val {
                                message = s.to_string();
                            }
                        }
                        "details" => {
                            if let Val::Option(Some(inner)) = field_val {
                                if let Val::String(s) = inner.as_ref() {
                                    details = serde_json::from_str(s).ok();
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Ok(GuardDecision::Deny(DenyReason {
                    code,
                    message,
                    details,
                }))
            }
            _ => Err(GuardError::WasmError(format!(
                "Expected record for deny reason, got: {:?}",
                val
            ))),
        }
    }

    /// Execute the guard with timeout protection and sufficient stack space
    fn execute_with_timeout<F>(&self, f: F) -> GuardResult
    where
        F: FnOnce() -> GuardResult,
    {
        // For synchronous execution, we use a simple approach
        // In production, this could be enhanced with proper async timeout
        let start = std::time::Instant::now();
        // Python WASM components require significant native stack space due to the
        // embedded interpreter. Use stacker to grow the native stack when needed.
        // Use stacker::grow to force allocation of a large stack segment (8MB).
        let result = stacker::grow(8 * 1024 * 1024, f);
        let elapsed = start.elapsed();

        if elapsed.as_millis() as u64 > self.config.timeout_ms {
            tracing::warn!(
                guard_id = %self.guard_id,
                elapsed_ms = elapsed.as_millis(),
                timeout_ms = self.config.timeout_ms,
                "WASM guard execution exceeded timeout"
            );
        }

        result
    }

    /// Call a no-argument WASM function that returns a string.
    /// Used for get-settings-schema and get-default-config.
    fn call_string_func(&self, func_name: &str) -> Result<String, GuardError> {
        stacker::grow(8 * 1024 * 1024, || {
            let linker = self.create_linker()?;
            let state = WasmState::new(self.config.config.clone());
            let mut store = Store::new(&self.engine, state);

            let instance = linker
                .instantiate(&mut store, &self.component)
                .map_err(|e| GuardError::WasmError(format!("Failed to instantiate component: {}", e)))?;

            let guard_export_idx = instance
                .get_export(&mut store, None, "mcp:security-guard/guard@0.1.0")
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Guard interface not found in component exports".to_string(),
                    )
                })?;

            let func_export_idx = instance
                .get_export(&mut store, Some(&guard_export_idx), func_name)
                .ok_or_else(|| {
                    GuardError::WasmError(format!(
                        "Function {} not found in guard interface",
                        func_name
                    ))
                })?;

            let func = instance
                .get_func(&mut store, &func_export_idx)
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Could not get function from export index".to_string(),
                    )
                })?;

            let mut results = vec![Val::Bool(false)]; // Placeholder
            func.call(&mut store, &[], &mut results)
                .map_err(|e| GuardError::WasmError(format!("WASM function call failed: {}", e)))?;

            func.post_return(&mut store)
                .map_err(|e| GuardError::WasmError(format!("WASM post-return failed: {}", e)))?;

            match &results[0] {
                Val::String(s) => Ok(s.to_string()),
                other => Err(GuardError::WasmError(format!(
                    "Expected string from {}, got: {:?}",
                    func_name, other
                ))),
            }
        })
    }

    /// Get the JSON Schema describing this guard's configurable parameters.
    /// Returns JSON-serialized JSON Schema (Draft 2020-12).
    pub fn get_settings_schema(&self) -> Result<String, GuardError> {
        self.call_string_func("get-settings-schema")
    }

    /// Get the default configuration as JSON.
    pub fn get_default_config(&self) -> Result<String, GuardError> {
        self.call_string_func("get-default-config")
    }
}

#[cfg(feature = "wasm-guards")]
impl NativeGuard for WasmGuard {
    fn evaluate_tools_list(
        &self,
        tools: &[rmcp::model::Tool],
        context: &GuardContext,
    ) -> GuardResult {
        self.execute_with_timeout(|| {
            tracing::debug!(
                guard_id = %self.guard_id,
                tool_count = tools.len(),
                server = %context.server_name,
                "Evaluating tools list with WASM guard"
            );

            let linker = self.create_linker()?;
            let state = WasmState::new(self.config.config.clone());
            let mut store = Store::new(&self.engine, state);

            // Instantiate the component
            let instance = linker
                .instantiate(&mut store, &self.component)
                .map_err(|e| GuardError::WasmError(format!("Failed to instantiate component: {}", e)))?;

            // Get the exported function from the guard interface
            // In component model, we need to get the exported instance first, then the function

            // Get the exported function from the guard interface
            // The component exports an instance for mcp:security-guard/guard@0.1.0
            // We need to access the function through that instance export
            let guard_export_idx = instance
                .get_export(&mut store, None, "mcp:security-guard/guard@0.1.0")
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Guard interface not found in component exports".to_string(),
                    )
                })?;

            // Get the function export from within the guard instance
            // Use the guard_export_idx as the parent to access nested exports
            let func_export_idx = instance
                .get_export(&mut store, Some(&guard_export_idx), "evaluate-tools-list")
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Function evaluate-tools-list not found in guard interface".to_string(),
                    )
                })?;

            // Now get the actual function using get_func with the full path
            let func = instance
                .get_func(&mut store, &func_export_idx)
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Could not get function from export index".to_string(),
                    )
                })?;

            // Build the tool list as WIT values
            let tool_records: Vec<Val> = tools
                .iter()
                .map(|t| {
                    Val::Record(vec![
                        ("name".into(), Val::String(t.name.to_string().into())),
                        (
                            "description".into(),
                            match &t.description {
                                Some(d) => Val::Option(Some(Box::new(Val::String(d.clone().into())))),
                                None => Val::Option(None),
                            },
                        ),
                        (
                            "input-schema".into(),
                            Val::String(
                                serde_json::to_string(&t.input_schema)
                                    .unwrap_or_else(|_| "{}".to_string())
                                    .into(),
                            ),
                        ),
                    ])
                })
                .collect();

            let tools_list = Val::List(tool_records);

            // Build context as WIT record
            let context_record = Val::Record(vec![
                ("server-name".into(), Val::String(context.server_name.clone().into())),
                ("server-url".into(), Val::Option(None)), // Not applicable for tools_list evaluation
                (
                    "identity".into(),
                    match &context.identity {
                        Some(id) => Val::Option(Some(Box::new(Val::String(id.clone().into())))),
                        None => Val::Option(None),
                    },
                ),
                (
                    "metadata".into(),
                    Val::String(
                        serde_json::to_string(&context.metadata)
                            .unwrap_or_else(|_| "{}".to_string())
                            .into(),
                    ),
                ),
            ]);

            // Call the function
            let mut results = vec![Val::Bool(false)]; // Placeholder for result
            func.call(&mut store, &[tools_list, context_record], &mut results)
                .map_err(|e| GuardError::WasmError(format!("WASM function call failed: {}", e)))?;

            // Post-call cleanup
            func.post_return(&mut store)
                .map_err(|e| GuardError::WasmError(format!("WASM post-return failed: {}", e)))?;

            Self::parse_decision(&results)
        })
    }

    fn evaluate_tool_invoke(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        context: &GuardContext,
    ) -> GuardResult {
        // Default implementation - WASM guards primarily target tools_list evaluation
        // This can be extended if the WIT interface is updated to support tool invocation
        tracing::debug!(
            guard_id = %self.guard_id,
            tool_name = %tool_name,
            server = %context.server_name,
            "WASM guard evaluate_tool_invoke called (default allow)"
        );
        let _ = (tool_name, arguments, context);
        Ok(GuardDecision::Allow)
    }

    fn evaluate_response(
        &self,
        response: &serde_json::Value,
        context: &GuardContext,
    ) -> GuardResult {
        // Default implementation - can be extended if WIT interface supports response evaluation
        tracing::debug!(
            guard_id = %self.guard_id,
            server = %context.server_name,
            "WASM guard evaluate_response called (default allow)"
        );
        let _ = (response, context);
        Ok(GuardDecision::Allow)
    }

    fn evaluate_connection(
        &self,
        server_name: &str,
        server_url: Option<&str>,
        context: &GuardContext,
    ) -> GuardResult {
        self.execute_with_timeout(|| {
            tracing::debug!(
                guard_id = %self.guard_id,
                server = %server_name,
                server_url = ?server_url,
                "Evaluating connection with WASM guard"
            );

            let linker = self.create_linker()?;
            let state = WasmState::new(self.config.config.clone());
            let mut store = Store::new(&self.engine, state);

            // Instantiate the component
            let instance = linker
                .instantiate(&mut store, &self.component)
                .map_err(|e| GuardError::WasmError(format!("Failed to instantiate component: {}", e)))?;

            // Get the exported function from the guard interface
            let guard_export_idx = instance
                .get_export(&mut store, None, "mcp:security-guard/guard@0.1.0")
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Guard interface not found in component exports".to_string(),
                    )
                })?;

            // Get the evaluate-server-connection function
            let func_export_idx = instance
                .get_export(&mut store, Some(&guard_export_idx), "evaluate-server-connection")
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Function evaluate-server-connection not found in guard interface".to_string(),
                    )
                })?;

            let func = instance
                .get_func(&mut store, &func_export_idx)
                .ok_or_else(|| {
                    GuardError::WasmError(
                        "Could not get function from export index".to_string(),
                    )
                })?;

            // Build context as WIT record with server_url
            let context_record = Val::Record(vec![
                ("server-name".into(), Val::String(context.server_name.clone().into())),
                (
                    "server-url".into(),
                    match server_url {
                        Some(url) => Val::Option(Some(Box::new(Val::String(url.to_string().into())))),
                        None => Val::Option(None),
                    },
                ),
                (
                    "identity".into(),
                    match &context.identity {
                        Some(id) => Val::Option(Some(Box::new(Val::String(id.clone().into())))),
                        None => Val::Option(None),
                    },
                ),
                (
                    "metadata".into(),
                    Val::String(
                        serde_json::to_string(&context.metadata)
                            .unwrap_or_else(|_| "{}".to_string())
                            .into(),
                    ),
                ),
            ]);

            // Call the function
            let mut results = vec![Val::Bool(false)]; // Placeholder for result
            func.call(&mut store, &[context_record], &mut results)
                .map_err(|e| GuardError::WasmError(format!("WASM function call failed: {}", e)))?;

            // Post-call cleanup
            func.post_return(&mut store)
                .map_err(|e| GuardError::WasmError(format!("WASM post-return failed: {}", e)))?;

            Self::parse_decision(&results)
        })
    }

    fn reset_server(&self, server_name: &str) {
        // WASM guards are stateless by design - no per-server state to reset
        tracing::debug!(
            guard_id = %self.guard_id,
            server = %server_name,
            "WASM guard reset_server called (no-op)"
        );
    }

    fn get_settings_schema(&self) -> Option<String> {
        match self.call_string_func("get-settings-schema") {
            Ok(schema) => Some(schema),
            Err(e) => {
                tracing::warn!(
                    guard_id = %self.guard_id,
                    error = %e,
                    "Failed to get settings schema from WASM guard"
                );
                None
            }
        }
    }

    fn get_default_config(&self) -> Option<String> {
        match self.call_string_func("get-default-config") {
            Ok(config) => Some(config),
            Err(e) => {
                tracing::warn!(
                    guard_id = %self.guard_id,
                    error = %e,
                    "Failed to get default config from WASM guard"
                );
                None
            }
        }
    }
}

// Non-wasm-guards feature: provide stub implementation
#[cfg(not(feature = "wasm-guards"))]
pub struct WasmGuard;

#[cfg(not(feature = "wasm-guards"))]
impl WasmGuard {
    pub fn new(_guard_id: String, _config: WasmGuardConfig) -> Result<Self, GuardError> {
        Err(GuardError::ConfigError(
            "WASM guards require the 'wasm-guards' feature to be enabled".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_config_validation() {
        let invalid_config = WasmGuardConfig {
            module_path: String::new(),
            max_memory: 1024 * 1024,
            max_wasm_stack: default_max_wasm_stack(),
            timeout_ms: 100,
            config: HashMap::new(),
        };

        #[cfg(feature = "wasm-guards")]
        {
            let result = WasmGuard::new("test".to_string(), invalid_config);
            assert!(result.is_err());
        }

        let valid_config = WasmGuardConfig {
            module_path: "/path/to/probe.wasm".to_string(),
            max_memory: 10 * 1024 * 1024,
            max_wasm_stack: default_max_wasm_stack(),
            timeout_ms: 100,
            config: HashMap::new(),
        };

        // File doesn't exist, so this should also error
        #[cfg(feature = "wasm-guards")]
        {
            let result = WasmGuard::new("test".to_string(), valid_config);
            assert!(result.is_err());
        }

        #[cfg(not(feature = "wasm-guards"))]
        {
            let _ = invalid_config;
            let _ = valid_config;
        }
    }

    #[test]
    fn test_default_config_values() {
        assert_eq!(default_max_memory(), 10 * 1024 * 1024);
        assert_eq!(default_max_wasm_stack(), 2 * 1024 * 1024);
        assert_eq!(default_timeout_ms(), 100);
    }

    #[test]
    fn test_config_deserialization() {
        let yaml = r#"
module_path: ./guards/test.wasm
max_memory: 5242880
timeout_ms: 50
config:
  blocked_patterns:
    - delete
    - "rm -rf"
  whitelist:
    - github
    - slack
"#;
        let config: WasmGuardConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.module_path, "./guards/test.wasm");
        assert_eq!(config.max_memory, 5242880);
        assert_eq!(config.timeout_ms, 50);
        assert!(config.config.contains_key("blocked_patterns"));
        assert!(config.config.contains_key("whitelist"));
    }

    #[test]
    fn test_config_defaults() {
        let yaml = r#"
module_path: ./guards/test.wasm
"#;
        let config: WasmGuardConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.module_path, "./guards/test.wasm");
        assert_eq!(config.max_memory, default_max_memory());
        assert_eq!(config.timeout_ms, default_timeout_ms());
        assert!(config.config.is_empty());
    }

    /// Integration test that loads the actual WASM guard and tests it
    #[test]
    #[cfg(feature = "wasm-guards")]
    fn test_wasm_guard_e2e() {
        use crate::mcp::security::native::NativeGuard;
        use rmcp::model::Tool;
        use std::borrow::Cow;
        use std::sync::Arc;

        // Helper to create a tool
        fn create_tool(name: &str, description: &str) -> Tool {
            Tool {
                name: Cow::Owned(name.to_string()),
                description: Some(Cow::Owned(description.to_string())),
                icons: None,
                title: None,
                meta: None,
                input_schema: Arc::new(serde_json::from_value(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                })).unwrap()),
                annotations: None,
                output_schema: None,
            }
        }

        // Path to the example WASM guard (relative to the workspace root)
        // CARGO_MANIFEST_DIR is crates/agentgateway, so go up two levels
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let wasm_path = manifest_dir
            .parent() // crates/
            .unwrap()
            .parent() // workspace root
            .unwrap()
            .join("examples/wasm-guards/simple-pattern-guard/simple-pattern-guard.wasm");

        // Skip if WASM file doesn't exist (not built yet)
        if !wasm_path.exists() {
            eprintln!(
                "Skipping e2e test: WASM file not found at {:?}",
                wasm_path
            );
            return;
        }

        // Create the guard
        let config = WasmGuardConfig {
            module_path: wasm_path.to_str().unwrap().to_string(),
            max_memory: 10 * 1024 * 1024,
            max_wasm_stack: default_max_wasm_stack(),
            timeout_ms: 1000,
            config: HashMap::new(), // Use default patterns
        };

        let guard = WasmGuard::new("test-wasm-guard".to_string(), config)
            .expect("Failed to create WASM guard");

        // Create test tools - one safe, one that should be blocked (contains "delete")
        let safe_tool = create_tool("read_file", "Reads contents of a file");
        let blocked_tool = create_tool("delete_file", "Deletes a file from disk");

        let context = super::GuardContext {
            server_name: "test-server".to_string(),
            identity: None,
            metadata: serde_json::json!({}),
        };

        // Test with safe tool - should allow
        let result = guard.evaluate_tools_list(&[safe_tool.clone()], &context);
        assert!(result.is_ok(), "Expected Ok result for safe tool, got: {:?}", result);
        assert!(
            matches!(result.unwrap(), super::GuardDecision::Allow),
            "Expected Allow decision for safe tool"
        );

        // Test with blocked tool - should deny
        let result = guard.evaluate_tools_list(&[blocked_tool.clone()], &context);
        assert!(result.is_ok(), "Expected Ok result (not error) for blocked tool");
        match result.unwrap() {
            super::GuardDecision::Deny(reason) => {
                assert_eq!(reason.code, "pattern_blocked");
                assert!(reason.message.contains("delete"));
            }
            other => panic!("Expected Deny decision for blocked tool, got {:?}", other),
        }

        // Test with both tools - should deny (blocked tool present)
        let result = guard.evaluate_tools_list(&[safe_tool, blocked_tool], &context);
        assert!(result.is_ok());
        assert!(
            matches!(result.unwrap(), super::GuardDecision::Deny(_)),
            "Expected Deny when blocked tool is present"
        );
    }
}
