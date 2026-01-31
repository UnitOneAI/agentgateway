// WASM Guard Loader
//
// Loads and executes security guards compiled to WebAssembly using the
// WebAssembly Component Model. This allows runtime loading of custom guards
// without recompiling the gateway.
//
// Guards must implement the WIT interface defined in:
// examples/wasm-guards/simple-pattern-guard/wit/guard.wit

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::{DenyReason, GuardContext, GuardDecision, GuardError, GuardResult};

#[cfg(feature = "wasm-guards")]
use super::native::NativeGuard;

/// Configuration for WASM-based guards
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct WasmGuardConfig {
    /// Path to WASM component module file
    pub module_path: String,

    /// Maximum memory for WASM instance (bytes)
    #[serde(default = "default_max_memory")]
    pub max_memory_bytes: usize,

    /// Maximum execution fuel (instruction count limit)
    #[serde(default = "default_max_fuel")]
    pub max_fuel: u64,

    /// Configuration values passed to WASM module via get_config()
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

fn default_max_memory() -> usize {
    10 * 1024 * 1024 // 10 MB
}

fn default_max_fuel() -> u64 {
    1_000_000 // 1 million instructions
}

// ============================================================================
// WASM Runtime Implementation (behind feature flag)
// ============================================================================

#[cfg(feature = "wasm-guards")]
mod runtime {
    use super::*;
    use wasmtime::component::{bindgen, Component, Linker, ResourceTable};
    use wasmtime::{Config, Engine, Store};
    use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

    // Generate Rust bindings from WIT interface
    bindgen!({
        world: "security-guard",
        path: "../../examples/wasm-guards/simple-pattern-guard/wit",
        async: false,
    });

    /// Host state passed to WASM runtime
    pub struct WasmHostState {
        /// Guard identifier for logging
        pub(crate) guard_id: String,

        /// Configuration values accessible via get_config()
        pub(crate) config: HashMap<String, serde_json::Value>,

        /// WASI context for basic system access
        pub(crate) wasi_ctx: WasiCtx,

        /// Resource table for WASI
        pub(crate) resource_table: ResourceTable,
    }

    impl WasiView for WasmHostState {
        fn ctx(&mut self) -> &mut WasiCtx {
            &mut self.wasi_ctx
        }

        fn table(&mut self) -> &mut ResourceTable {
            &mut self.resource_table
        }
    }

    // Implement the host interface from WIT
    impl mcp::security_guard::host::Host for WasmHostState {
        /// Log a message to the host logging system
        fn log(&mut self, level: u8, message: String) {
            match level {
                0 => tracing::trace!(guard = %self.guard_id, "[WASM] {}", message),
                1 => tracing::debug!(guard = %self.guard_id, "[WASM] {}", message),
                2 => tracing::info!(guard = %self.guard_id, "[WASM] {}", message),
                3 => tracing::warn!(guard = %self.guard_id, "[WASM] {}", message),
                _ => tracing::error!(guard = %self.guard_id, "[WASM] {}", message),
            }
        }

        /// Get current Unix timestamp in milliseconds
        fn get_time(&mut self) -> u64 {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        }

        /// Get configuration value by key
        fn get_config(&mut self, key: String) -> String {
            self.config
                .get(&key)
                .map(|v| {
                    // Return JSON string for complex types, raw string for simple values
                    match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => serde_json::to_string(other).unwrap_or_default(),
                    }
                })
                .unwrap_or_default()
        }
    }

    /// Cache for compiled WASM modules (thread-safe)
    pub struct WasmModuleCache {
        engine: Engine,
        modules: RwLock<HashMap<PathBuf, (Arc<Component>, SystemTime)>>,
    }

    impl WasmModuleCache {
        /// Create new cache with configured engine
        pub fn new() -> Result<Self, GuardError> {
            let mut config = Config::new();

            // Performance optimizations
            config.cranelift_opt_level(wasmtime::OptLevel::Speed);
            config.async_support(false); // Sync for now
            config.consume_fuel(true); // Enable fuel metering

            // Try to enable caching of compiled modules (non-fatal if fails)
            if let Err(e) = config.cache_config_load_default() {
                tracing::warn!("Failed to load wasmtime cache config: {}", e);
            }

            let engine = Engine::new(&config)
                .map_err(|e| GuardError::WasmError(format!("Failed to create engine: {}", e)))?;

            Ok(Self {
                engine,
                modules: RwLock::new(HashMap::new()),
            })
        }

        pub fn engine(&self) -> &Engine {
            &self.engine
        }

        pub fn get_or_compile(&self, path: &Path) -> Result<Arc<Component>, GuardError> {
            let canonical = path.canonicalize().map_err(|e| {
                GuardError::ConfigError(format!("Invalid path {}: {}", path.display(), e))
            })?;

            let file_mtime = std::fs::metadata(&canonical)
                .and_then(|m| m.modified())
                .map_err(|e| GuardError::ConfigError(format!("Cannot read file: {}", e)))?;

            // Check cache with read lock
            {
                let cache = self.modules.read().expect("cache lock poisoned");
                if let Some((component, cached_mtime)) = cache.get(&canonical) {
                    if *cached_mtime >= file_mtime {
                        tracing::debug!(path = %canonical.display(), "Using cached WASM module");
                        return Ok(component.clone());
                    }
                }
            }

            // Compile and cache with write lock
            let mut cache = self.modules.write().expect("cache lock poisoned");

            // Double-check after acquiring write lock
            if let Some((component, cached_mtime)) = cache.get(&canonical) {
                if *cached_mtime >= file_mtime {
                    return Ok(component.clone());
                }
            }

            tracing::info!(path = %canonical.display(), "Compiling WASM module");

            let bytes = std::fs::read(&canonical)
                .map_err(|e| GuardError::ConfigError(format!("Failed to read module: {}", e)))?;

            // Compile on a thread with larger stack (cranelift needs ~8MB for large modules)
            let engine = self.engine.clone();
            let component = std::thread::Builder::new()
                .name("wasm-compile".into())
                .stack_size(16 * 1024 * 1024) // 16MB stack
                .spawn(move || Component::new(&engine, &bytes))
                .map_err(|e| GuardError::WasmError(format!("Failed to spawn compile thread: {}", e)))?
                .join()
                .map_err(|_| GuardError::WasmError("Compile thread panicked".to_string()))?
                .map_err(|e| GuardError::WasmError(format!("Failed to compile: {}", e)))?;

            let component = Arc::new(component);
            cache.insert(canonical, (component.clone(), file_mtime));

            Ok(component)
        }

        /// Invalidate a specific module from cache
        #[allow(dead_code)]
        pub fn invalidate(&self, path: &Path) {
            if let Ok(canonical) = path.canonicalize() {
                let mut cache = self.modules.write().expect("cache lock poisoned");
                cache.remove(&canonical);
            }
        }
    }

    /// A compiled and ready-to-execute WASM guard
    pub struct WasmProbe {
        /// Guard identifier
        id: String,

        /// Compiled component (from cache)
        component: Arc<Component>,

        /// Engine reference
        engine: Engine,

        /// Configuration for new instances
        config: WasmGuardConfig,
    }

    impl WasmProbe {
        /// Create a new WASM probe from configuration
        pub fn new(
            id: String,
            config: WasmGuardConfig,
            cache: &WasmModuleCache,
        ) -> Result<Self, GuardError> {
            // Validate configuration
            if config.module_path.is_empty() {
                return Err(GuardError::ConfigError(
                    "module_path cannot be empty".to_string(),
                ));
            }

            let path = Path::new(&config.module_path);
            if !path.exists() {
                return Err(GuardError::ConfigError(format!(
                    "WASM module not found: {}",
                    config.module_path
                )));
            }

            // Get or compile the component
            let component = cache.get_or_compile(path)?;

            Ok(Self {
                id,
                component,
                engine: cache.engine().clone(),
                config,
            })
        }

        /// Convert WASM decision type to native GuardDecision
        fn convert_decision(
            wasm_decision: Result<exports::mcp::security_guard::guard::Decision, String>,
        ) -> GuardResult {
            use exports::mcp::security_guard::guard::Decision;
            match wasm_decision {
                Ok(decision) => match decision {
                    Decision::Allow => Ok(GuardDecision::Allow),
                    Decision::Deny(reason) => {
                        Ok(GuardDecision::Deny(DenyReason {
                            code: reason.code,
                            message: reason.message,
                            details: reason.details.and_then(|s| serde_json::from_str(&s).ok()),
                        }))
                    }
                    Decision::Modify(json_str) => {
                        // For now, Modify is not fully supported - treat as Allow with warning
                        tracing::warn!(
                            modify_json = %json_str,
                            "WASM guard returned Modify action - not fully supported yet"
                        );
                        Ok(GuardDecision::Allow)
                    }
                },
                Err(e) => Err(GuardError::WasmError(format!("WASM guard error: {}", e))),
            }
        }

        /// Convert native tools to WASM tool format
        fn convert_tools(
            tools: &[rmcp::model::Tool],
        ) -> Vec<exports::mcp::security_guard::guard::Tool> {
            use exports::mcp::security_guard::guard::Tool;
            tools
                .iter()
                .map(|t| Tool {
                    name: t.name.to_string(),
                    description: t.description.as_ref().map(|d| d.to_string()),
                    input_schema: serde_json::to_string(&t.input_schema).unwrap_or_default(),
                })
                .collect()
        }

        /// Convert native context to WASM context format
        fn convert_context(
            context: &GuardContext,
        ) -> exports::mcp::security_guard::guard::GuardContext {
            exports::mcp::security_guard::guard::GuardContext {
                server_name: context.server_name.clone(),
                identity: context.identity.clone(),
                metadata: serde_json::to_string(&context.metadata).unwrap_or_default(),
            }
        }
    }

    impl NativeGuard for WasmProbe {
        fn evaluate_tools_list(
            &self,
            tools: &[rmcp::model::Tool],
            context: &GuardContext,
        ) -> GuardResult {
            tracing::debug!(
                guard_id = %self.id,
                tool_count = tools.len(),
                server = %context.server_name,
                "Evaluating tools list with WASM guard"
            );

            // Create WASI context
            let wasi_ctx = WasiCtxBuilder::new().build();

            // Create store with host state
            let mut store = Store::new(
                &self.engine,
                WasmHostState {
                    guard_id: self.id.clone(),
                    config: self.config.config.clone(),
                    wasi_ctx,
                    resource_table: ResourceTable::new(),
                },
            );

            // Set fuel limit
            if let Err(e) = store.set_fuel(self.config.max_fuel) {
                tracing::warn!("Failed to set fuel limit: {}", e);
            }

            // Create linker and add WASI + host functions
            let mut linker = Linker::new(&self.engine);

            // Add WASI to linker
            if let Err(e) = wasmtime_wasi::add_to_linker_sync(&mut linker) {
                return Err(GuardError::WasmError(format!(
                    "Failed to add WASI to linker: {}",
                    e
                )));
            }

            // Add our host functions to linker
            if let Err(e) = SecurityGuard::add_to_linker(&mut linker, |state| state) {
                return Err(GuardError::WasmError(format!(
                    "Failed to add host to linker: {}",
                    e
                )));
            }

            // Instantiate component
            let instance = SecurityGuard::instantiate(&mut store, &self.component, &linker)
                .map_err(|e| GuardError::WasmError(format!("Failed to instantiate: {}", e)))?;

            // Convert inputs
            let wasm_tools = Self::convert_tools(tools);
            let wasm_context = Self::convert_context(context);

            // Call the WASM guard function
            let result = instance
                .mcp_security_guard_guard()
                .call_evaluate_tools_list(&mut store, &wasm_tools, &wasm_context)
                .map_err(|e| GuardError::WasmError(format!("Execution failed: {}", e)))?;

            // Convert and return result
            Self::convert_decision(result)
        }

        fn evaluate_tool_invoke(
            &self,
            _tool_name: &str,
            _arguments: &serde_json::Value,
            _context: &GuardContext,
        ) -> GuardResult {
            // For now, WASM guards only support tools_list evaluation
            // Future: Add evaluate_tool_invoke to WIT interface
            Ok(GuardDecision::Allow)
        }

        fn evaluate_response(
            &self,
            _response: &serde_json::Value,
            _context: &GuardContext,
        ) -> GuardResult {
            // For now, WASM guards only support tools_list evaluation
            // Future: Add evaluate_response to WIT interface
            Ok(GuardDecision::Allow)
        }
    }
}

#[cfg(feature = "wasm-guards")]
pub use runtime::{WasmModuleCache, WasmProbe};

// ============================================================================
// Stub implementation when wasm-guards feature is disabled
// ============================================================================

#[cfg(not(feature = "wasm-guards"))]
pub struct WasmProbe {
    #[allow(dead_code)]
    config: WasmGuardConfig,
}

#[cfg(not(feature = "wasm-guards"))]
impl WasmProbe {
    pub fn new(config: WasmGuardConfig) -> Result<Self, GuardError> {
        if config.module_path.is_empty() {
            return Err(GuardError::ConfigError(
                "module_path cannot be empty".to_string(),
            ));
        }
        Ok(Self { config })
    }

    pub fn evaluate(
        &self,
        _payload: &serde_json::Value,
        _context: &GuardContext,
    ) -> GuardResult {
        // Stub: always allow when feature is disabled
        Ok(GuardDecision::Allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "wasm-guards"))]
    #[test]
    fn test_wasm_config_validation() {
        let invalid_config = WasmGuardConfig {
            module_path: String::new(),
            max_memory_bytes: 1024 * 1024,
            max_fuel: 1_000_000,
            config: HashMap::new(),
        };

        assert!(WasmProbe::new(invalid_config).is_err());

        let valid_config = WasmGuardConfig {
            module_path: "/path/to/probe.wasm".to_string(),
            max_memory_bytes: 10 * 1024 * 1024,
            max_fuel: 1_000_000,
            config: HashMap::new(),
        };

        // This will succeed because stub doesn't check file exists
        assert!(WasmProbe::new(valid_config).is_ok());
    }

    #[cfg(feature = "wasm-guards")]
    mod wasm_tests {
        use super::*;

        #[test]
        fn test_wasm_module_cache_creation() {
            let cache = WasmModuleCache::new();
            assert!(cache.is_ok());
        }

        #[test]
        fn test_host_functions() {
            use runtime::WasmHostState;
            use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};
            use super::runtime::mcp::security_guard::host::Host;

            let mut state = WasmHostState {
                guard_id: "test".to_string(),
                config: {
                    let mut map = HashMap::new();
                    map.insert("key1".to_string(), serde_json::json!("value1"));
                    map.insert("key2".to_string(), serde_json::json!({"nested": true}));
                    map
                },
                wasi_ctx: WasiCtxBuilder::new().build(),
                resource_table: ResourceTable::new(),
            };

            // Test get_config
            assert_eq!(state.get_config("key1".to_string()), "value1");
            assert_eq!(state.get_config("key2".to_string()), r#"{"nested":true}"#);
            assert_eq!(state.get_config("missing".to_string()), "");

            // Test get_time returns reasonable value
            let time = state.get_time();
            assert!(time > 1700000000000); // After 2023
        }
    }
}
