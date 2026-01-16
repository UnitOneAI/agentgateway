# MCP Security Guards: Interface Contract Specification

## Overview

This document defines the formal interface contract for MCP Security Guard plugins/probes. Security guards intercept MCP operations at various lifecycle phases to enforce security policies, detect threats, and modify requests/responses as needed.

## Guard Tiers and Their Contracts

### 1. Native Guards (Compiled, Type-Safe)

#### 1.1 Rust Trait Contract

```rust
/// Core trait that all native security guards must implement
pub trait NativeGuard: Send + Sync {
    /// Evaluate a list of tools before returning to client
    /// Called during: tools/list response
    fn evaluate_tools_list(
        &self,
        tools: &[rmcp::model::Tool],
        context: &GuardContext,
    ) -> GuardResult {
        Ok(GuardDecision::Allow)
    }

    /// Evaluate a single tool invocation request
    /// Called during: tools/call request
    fn evaluate_tool_call(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        context: &GuardContext,
    ) -> GuardResult {
        Ok(GuardDecision::Allow)
    }

    /// Evaluate a tool invocation response
    /// Called during: tools/call response
    fn evaluate_tool_response(
        &self,
        tool_name: &str,
        result: &serde_json::Value,
        context: &GuardContext,
    ) -> GuardResult {
        Ok(GuardDecision::Allow)
    }

    /// Evaluate a prompt request
    /// Called during: prompts/get request
    fn evaluate_prompt_request(
        &self,
        prompt_name: &str,
        arguments: &serde_json::Value,
        context: &GuardContext,
    ) -> GuardResult {
        Ok(GuardDecision::Allow)
    }

    /// Evaluate a resource read request
    /// Called during: resources/read request
    fn evaluate_resource_request(
        &self,
        uri: &str,
        context: &GuardContext,
    ) -> GuardResult {
        Ok(GuardDecision::Allow)
    }
}
```

#### 1.2 Core Data Structures

```rust
/// Context provided to every guard invocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardContext {
    /// Name of the upstream MCP server
    pub server_name: String,

    /// Authenticated identity (if available)
    pub identity: Option<Identity>,

    /// Additional metadata (request headers, trace context, etc.)
    pub metadata: serde_json::Value,
}

/// Identity information from authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Subject identifier (e.g., user ID)
    pub sub: String,

    /// Email address (if available)
    pub email: Option<String>,

    /// Groups/roles (if available)
    pub groups: Vec<String>,

    /// Additional claims
    pub claims: HashMap<String, serde_json::Value>,
}

/// Result type for guard evaluations
pub type GuardResult = Result<GuardDecision, GuardError>;

/// Decision returned by a guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardDecision {
    /// Allow the operation to proceed
    Allow,

    /// Deny the operation with reason
    Deny(DenyReason),

    /// Modify the operation (request/response transformation)
    Modify(ModifyAction),
}

/// Reason for denying an operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenyReason {
    /// Machine-readable error code
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Additional structured details
    pub details: Option<serde_json::Value>,
}

/// Action to modify request or response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModifyAction {
    /// Replace tool list with filtered/modified tools
    ReplaceTools(Vec<rmcp::model::Tool>),

    /// Redact sensitive fields in arguments/results
    RedactFields {
        /// JSONPath expressions to redact
        paths: Vec<String>,
        /// Replacement value
        replacement: String,
    },

    /// Add warning annotations
    AddWarning {
        message: String,
    },
}

/// Guard execution errors
#[derive(Debug, thiserror::Error)]
pub enum GuardError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Execution timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}
```

#### 1.3 Configuration Schema

```yaml
# Example native guard configuration
- kind: tool_poisoning
  enabled: true
  priority: 10
  timeout_ms: 100
  failure_mode: fail_closed
  runs_on:
    - tools_list
    - response
  config:
    strict_mode: true
    custom_patterns:
      - "(?i)malicious_pattern"
    scan_fields:
      - name
      - description
      - input_schema
    alert_threshold: 1
```

**Configuration Fields:**

- `kind`: Guard type identifier (enum: `tool_poisoning`, `rug_pull`, `tool_shadowing`, `server_whitelist`, etc.)
- `enabled`: Boolean flag to enable/disable guard
- `priority`: Integer (0-100, lower = higher priority)
- `timeout_ms`: Maximum execution time in milliseconds
- `failure_mode`: `fail_closed` (block on error) or `fail_open` (allow on error)
- `runs_on`: Array of execution phases (see Phase Contract below)
- `config`: Guard-specific configuration (varies by guard type)

---

### 2. WASM Guards (Runtime-Loaded, Sandboxed)

#### 2.1 WebAssembly Interface Type (WIT) Contract

```wit
// Package declaration
package mcp:security-guard@0.1.0;

// Core guard interface
interface guard {
    // Data structures matching Rust types

    record guard-context {
        server-name: string,
        identity: option<identity>,
        metadata: string,  // JSON-encoded
    }

    record identity {
        sub: string,
        email: option<string>,
        groups: list<string>,
        claims: string,  // JSON-encoded map
    }

    record tool {
        name: string,
        description: option<string>,
        input-schema: string,  // JSON-encoded
    }

    variant decision {
        allow,
        deny(deny-reason),
        modify(string),  // JSON-encoded ModifyAction
    }

    record deny-reason {
        code: string,
        message: string,
        details: option<string>,  // JSON-encoded
    }

    // Guard evaluation functions

    /// Evaluate a list of tools
    evaluate-tools-list: func(
        tools: list<tool>,
        context: guard-context
    ) -> result<decision, string>;

    /// Evaluate a tool invocation request
    evaluate-tool-call: func(
        tool-name: string,
        arguments: string,  // JSON-encoded
        context: guard-context
    ) -> result<decision, string>;

    /// Evaluate a tool response
    evaluate-tool-response: func(
        tool-name: string,
        result: string,  // JSON-encoded
        context: guard-context
    ) -> result<decision, string>;

    /// Evaluate a prompt request
    evaluate-prompt-request: func(
        prompt-name: string,
        arguments: string,  // JSON-encoded
        context: guard-context
    ) -> result<decision, string>;

    /// Evaluate a resource read request
    evaluate-resource-request: func(
        uri: string,
        context: guard-context
    ) -> result<decision, string>;
}

// Host functions available to WASM guards
interface host {
    /// Log a message to host logging system
    log: func(level: log-level, message: string);

    /// Get configuration value
    get-config: func(key: string) -> option<string>;

    /// Record a metric
    record-metric: func(name: string, value: f64, labels: list<tuple<string, string>>);

    enum log-level {
        trace,
        debug,
        info,
        warn,
        error,
    }
}

// World combining guest and host interfaces
world security-guard {
    export guard;
    import host;
}
```

#### 2.2 WASM Guard Implementation Example

```rust
// Rust implementation using wit-bindgen
use bindings::*;

struct MyGuard;

impl Guest for MyGuard {
    fn evaluate_tools_list(
        tools: Vec<Tool>,
        context: GuardContext,
    ) -> Result<Decision, String> {
        // Log to host
        host::log(LogLevel::Info, "Evaluating tools list");

        // Get configuration
        let threshold = host::get_config("threshold")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1);

        // Scan for violations
        let violations: Vec<_> = tools
            .iter()
            .filter(|tool| is_suspicious(&tool.name))
            .collect();

        if violations.len() >= threshold {
            Ok(Decision::Deny(DenyReason {
                code: "suspicious_tools".to_string(),
                message: format!("Found {} suspicious tools", violations.len()),
                details: Some(serde_json::to_string(&violations).unwrap()),
            }))
        } else {
            Ok(Decision::Allow)
        }
    }

    // ... implement other methods
}

bindings::export!(MyGuard with_types_in bindings);
```

#### 2.3 WASM Configuration Schema

```yaml
- kind: wasm
  enabled: true
  priority: 20
  timeout_ms: 500
  failure_mode: fail_closed
  runs_on:
    - tools_list
  config:
    module_path: "/etc/agentgateway/guards/pattern-detector.wasm"
    # Configuration passed to WASM module via get-config()
    wasm_config:
      threshold: 2
      patterns:
        - "suspicious"
        - "malicious"
```

---

### 3. HTTP Hooks (External Services) - OPTIONAL

**Status:** Deferred. Included for completeness but deemed optional based on latency/complexity tradeoffs.

#### 3.1 HTTP API Contract

**Endpoint:** Configurable webhook URL

**Request Format:**

```json
POST /guard/evaluate
Content-Type: application/json

{
  "phase": "tools_list",
  "operation": {
    "type": "list_tools",
    "data": {
      "tools": [
        {
          "name": "file_reader",
          "description": "Reads files",
          "input_schema": {...}
        }
      ]
    }
  },
  "context": {
    "server_name": "github-mcp",
    "identity": {
      "sub": "user123",
      "email": "user@example.com",
      "groups": ["developers"]
    },
    "metadata": {
      "request_id": "req-abc123",
      "trace_id": "trace-xyz789"
    }
  }
}
```

**Response Format:**

```json
HTTP/1.1 200 OK
Content-Type: application/json

{
  "decision": "deny",
  "reason": {
    "code": "policy_violation",
    "message": "Tool 'file_reader' not allowed for this user",
    "details": {
      "required_group": "admins",
      "user_groups": ["developers"]
    }
  }
}
```

**Response Variants:**

```json
// Allow
{"decision": "allow"}

// Deny
{
  "decision": "deny",
  "reason": {
    "code": "string",
    "message": "string",
    "details": {}  // optional
  }
}

// Modify
{
  "decision": "modify",
  "action": {
    "type": "replace_tools",
    "tools": [...]
  }
}
```

---

## Guard Execution Phases

Guards can be configured to run at specific lifecycle phases:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardPhase {
    /// Before processing any request
    Request,

    /// After receiving response, before returning to client
    Response,

    /// When returning tools/list
    ToolsList,

    /// When processing tools/call request
    ToolInvoke,

    /// When returning tools/call response
    ToolResult,

    /// When processing prompts/get request
    PromptRequest,

    /// When processing resources/read request
    ResourceRequest,
}
```

**Phase Mapping:**

| Phase | Native Trait Method | WASM Function |
|-------|-------------------|--------------|
| `tools_list` | `evaluate_tools_list()` | `evaluate-tools-list()` |
| `tool_invoke` | `evaluate_tool_call()` | `evaluate-tool-call()` |
| `tool_result` | `evaluate_tool_response()` | `evaluate-tool-response()` |
| `prompt_request` | `evaluate_prompt_request()` | `evaluate-prompt-request()` |
| `resource_request` | `evaluate_resource_request()` | `evaluate-resource-request()` |

---

## Guard Configuration Specification

### Top-Level Backend Configuration

```yaml
backends:
  - mcp:
      targets:
        - name: github
          stdio:
            cmd: npx
            args: ["-y", "@modelcontextprotocol/server-github"]

      # Security guards configuration
      security_guards:
        - kind: tool_poisoning
          enabled: true
          priority: 10
          timeout_ms: 100
          failure_mode: fail_closed
          runs_on: [tools_list, response]
          config:
            strict_mode: true
            custom_patterns: ["(?i)execute.*as.*root"]

        - kind: server_whitelist
          enabled: true
          priority: 5
          timeout_ms: 50
          failure_mode: fail_closed
          runs_on: [request]
          config:
            allowed_servers: ["github", "gitlab"]

        - kind: wasm
          enabled: true
          priority: 20
          timeout_ms: 500
          failure_mode: fail_open
          runs_on: [tools_list]
          config:
            module_path: "/etc/guards/custom-detector.wasm"
            wasm_config:
              sensitivity: "high"
```

### Configuration Field Constraints

| Field | Type | Required | Default | Constraints |
|-------|------|----------|---------|-------------|
| `kind` | Enum | Yes | - | One of: `tool_poisoning`, `rug_pull`, `tool_shadowing`, `server_whitelist`, `wasm` |
| `enabled` | Boolean | No | `true` | - |
| `priority` | Integer | No | `50` | 0-100, lower = higher priority |
| `timeout_ms` | Integer | No | `1000` | 10-10000 |
| `failure_mode` | Enum | No | `fail_closed` | One of: `fail_closed`, `fail_open` |
| `runs_on` | Array | Yes | - | Non-empty array of GuardPhase values |
| `config` | Object | No | `{}` | Guard-specific configuration |

---

## Execution Guarantees

### Priority and Ordering

1. Guards are sorted by `priority` (ascending: 0 = highest priority)
2. Guards with same priority execute in configuration order
3. If any guard returns `Deny`, execution stops (short-circuit)
4. If any guard returns `Modify`, subsequent guards see modified data

### Timeout Behavior

- Each guard has independent timeout via `timeout_ms`
- On timeout:
  - `fail_closed`: Treat as `Deny` with timeout error
  - `fail_open`: Treat as `Allow` (log warning)

### Error Handling

- Guards must be resilient and handle errors gracefully
- Native guards return `Result<GuardDecision, GuardError>`
- WASM guards return `result<decision, string>`
- HTTP hooks return standard HTTP status codes

**Error to Decision Mapping:**

| Failure Mode | Error Outcome |
|--------------|---------------|
| `fail_closed` | Deny with error reason |
| `fail_open` | Allow (log error at WARN level) |

---

## Performance Characteristics

| Tier | Latency | Isolation | Use Case |
|------|---------|-----------|----------|
| Native | <1ms | Process-level | Built-in policies, high-frequency checks |
| WASM | 5-10ms | Sandboxed | Custom policies, org-specific rules |
| HTTP | 50-500ms | Full isolation | External policy engines, audit systems |

---

## Versioning and Compatibility

### Semantic Versioning

- Contract version: `0.1.0`
- Breaking changes increment MINOR version (pre-1.0)
- Additive changes (new phases, optional fields) increment PATCH

### Backward Compatibility

- New guard phases are optional (guards can ignore them)
- New configuration fields must have sensible defaults
- Deprecated phases must be supported for 2 major versions

### WASM ABI Stability

- WIT interface is versioned independently: `mcp:security-guard@0.1.0`
- Component Model ensures ABI compatibility within same MINOR version
- Guards declare minimum required version in manifest

---

## Security Considerations

### Guard Isolation

1. **Native Guards:** Trust boundary = process
   - Can access full process memory
   - Should validate all inputs
   - Must not panic on malformed data

2. **WASM Guards:** Trust boundary = sandbox
   - Cannot access host memory directly
   - Limited to capabilities exposed via WIT
   - Resource limits enforced by runtime

3. **HTTP Hooks:** Trust boundary = network
   - Must use TLS for sensitive data
   - Authenticate webhook requests
   - Rate limit external calls

### Data Sensitivity

Guards may see sensitive data:
- User credentials in tool arguments
- PII in tool responses
- Internal infrastructure details

**Mitigation:**
- Guards must not log sensitive data
- WASM guards cannot exfiltrate data (sandboxed)
- HTTP hooks require explicit data sharing policy
- Consider PII redaction before guard execution

### DoS Protection

Guards could be used for DoS attacks:
- Infinite loops
- Excessive memory allocation
- Regex catastrophic backtracking

**Mitigation:**
- Timeout enforcement (`timeout_ms`)
- WASM fuel limits (instruction counting)
- Memory limits on WASM modules
- Regex complexity analysis for native guards

---

## Testing Contract Compliance

### Native Guard Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_contract_allow() {
        let guard = MyGuard::new(config).unwrap();
        let context = create_test_context();
        let tools = vec![create_benign_tool()];

        let result = guard.evaluate_tools_list(&tools, &context);
        assert!(matches!(result, Ok(GuardDecision::Allow)));
    }

    #[test]
    fn test_guard_contract_deny() {
        let guard = MyGuard::new(config).unwrap();
        let context = create_test_context();
        let tools = vec![create_malicious_tool()];

        let result = guard.evaluate_tools_list(&tools, &context);
        match result {
            Ok(GuardDecision::Deny(reason)) => {
                assert!(!reason.code.is_empty());
                assert!(!reason.message.is_empty());
            },
            _ => panic!("Expected Deny decision"),
        }
    }

    #[test]
    fn test_guard_contract_timeout() {
        let guard = SlowGuard::new();
        let context = create_test_context();

        // Should timeout and respect failure_mode
        let result = execute_with_timeout(&guard, context, Duration::from_millis(10));
        // Assert based on failure_mode
    }
}
```

### WASM Guard Testing

```bash
# Build WASM module
cargo component build --release

# Test with wasmtime
wasmtime run target/wasm32-wasip2/release/my_guard.wasm

# Integration test with host
cargo test --test wasm_integration
```

---

## Migration Guide

### Adding a New Native Guard

1. Implement `NativeGuard` trait
2. Add variant to `McpGuardKind` enum
3. Update `GuardExecutor::new()` initialization
4. Add configuration schema
5. Write tests
6. Document in architecture.md

### Creating a WASM Guard

1. Create new Cargo project with `cargo component new`
2. Add `wit/guard.wit` interface definition
3. Implement guard logic in `src/lib.rs`
4. Build: `cargo component build --release`
5. Test locally with wasmtime
6. Deploy `.wasm` file to `/etc/agentgateway/guards/`
7. Configure in YAML

### Deprecating a Guard Phase

1. Mark phase as deprecated in code comments
2. Add deprecation warning in logs when used
3. Document migration path
4. Support for 2 major versions
5. Remove in major version bump

---

## Examples

See:
- `examples/guards/native/tool-poisoning/` - Native guard implementation
- `examples/guards/wasm/simple-pattern-guard/` - WASM guard implementation
- `examples/configs/security-guards.yaml` - Complete configuration example

---

## Changelog

### v0.1.0 (2025-12-30)

- Initial contract specification
- Native guard trait definition
- WASM WIT interface
- HTTP hook specification (optional)
- Guard execution phases
- Configuration schema
- Performance characteristics
- Security considerations
