# MCP Security Guards Framework - Design Document

## Executive Summary

This document defines the architecture and design principles for the MCP Security Guards framework in Agent Gateway. The framework provides extensible security capabilities for MCP protocol operations.

> **Note**: For the authoritative interface specification, see [mcp-security-guards-contract.md](./mcp-security-guards-contract.md). This document focuses on architectural concepts and design rationale.

## Architecture Overview

### Three-Tier Implementation Strategy

```
┌─────────────────────────────────────────────────────────────┐
│                    Tier 1: Native Guards                      │
│   (High-Performance Rust - MCP Protocol-Specific Threats)     │
│   - Tool Poisoning Detection                                  │
│   - Rug Pull Detection                                        │
│   - Tool Shadowing Prevention                                 │
│   - PII Detection & Masking                                   │
└───────────────────────────────────┬─────────────────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────┐
│                Tier 2: WASM Guards                            │
│   (Runtime-Loaded - Sandboxed Extensibility)                 │
│   - Custom Detection Logic                                   │
│   - Org-Specific Rules                                       │
│   - Third-Party Security Modules                             │
└───────────────────────────────────┬─────────────────────────┘
                                    │
┌───────────────────────────────────▼─────────────────────────┐
│             Tier 3: External Services (Future)                │
│   (Webhook/gRPC - Enterprise Integration)                    │
│   - ML-Based Anomaly Detection                               │
│   - External Policy Engines (OPA, etc.)                      │
│   - SIEM/Audit System Integration                            │
└─────────────────────────────────────────────────────────────┘
```

For detailed analysis of each tier, see [mcp-security-guards-architecture.md](./mcp-security-guards-architecture.md).

## Core Design Principles

1. **Non-Blocking by Default**: Security checks should not add significant latency
2. **Fail-Safe**: Configurable failure modes (fail-open, fail-closed)
3. **Composable**: Chain multiple security guards in priority order
4. **Observable**: All security decisions are logged with context
5. **Extensible**: Easy to add new security capabilities via traits
6. **MCP-Aware**: Deep inspection of MCP protocol messages (tools/list, tools/call, etc.)

---

## Interface Specification

The complete interface specification is defined in [mcp-security-guards-contract.md](./mcp-security-guards-contract.md).

### Key Interfaces

**`NativeGuard` trait** - Core trait for all native Rust guards:
- `evaluate_tools_list()` - Inspect tools/list responses
- `evaluate_tool_call()` - Inspect tools/call requests
- `evaluate_tool_response()` - Inspect tool invocation responses

**`GuardDecision` enum** - Possible outcomes:
- `Allow` - Operation proceeds
- `Deny(DenyReason)` - Operation blocked with reason
- `Modify(ModifyAction)` - Operation modified (e.g., mask PII)

**`GuardContext`** - Context passed to guards:
- `server_name` - MCP server being accessed
- `identity` - Authenticated user (if available)
- `metadata` - Additional request context

**`GuardExecutor`** - Orchestrates guard execution:
- Executes guards in priority order
- Handles timeouts and failure modes
- Supports hot-reload of guard configuration

See the contract document for complete type definitions and WASM interface specifications.

---

## Example Implementation: Tool Poisoning Detection Guard

The actual implementation lives in `crates/agentgateway/src/mcp/security/native/tool_poisoning.rs`. Here's a simplified example showing the key concepts:

```rust
use crate::mcp::security::{GuardContext, GuardDecision, GuardResult, DenyReason};
use crate::mcp::security::native::NativeGuard;
use regex::Regex;

pub struct ToolPoisoningDetector {
    patterns: Vec<Regex>,
    strict_mode: bool,
}

impl ToolPoisoningDetector {
    pub fn new(config: ToolPoisoningConfig) -> Result<Self, GuardError> {
        let mut patterns = Self::default_patterns();
        for pattern in &config.custom_patterns {
            patterns.push(Regex::new(pattern)?);
        }
        Ok(Self { patterns, strict_mode: config.strict_mode })
    }

    fn scan_text(&self, text: &str) -> Option<DenyReason> {
        for pattern in &self.patterns {
            if let Some(m) = pattern.find(text) {
                return Some(DenyReason {
                    code: "tool_poisoning_detected".to_string(),
                    message: format!("Malicious pattern detected: {}", m.as_str()),
                    details: Some(serde_json::json!({
                        "matched_pattern": pattern.as_str(),
                        "matched_text": m.as_str(),
                    })),
                });
            }
        }
        None
    }
}

impl NativeGuard for ToolPoisoningDetector {
    fn evaluate_tools_list(
        &self,
        tools: &[rmcp::model::Tool],
        _context: &GuardContext,
    ) -> GuardResult {
        for tool in tools {
            // Scan tool name
            if let Some(reason) = self.scan_text(&tool.name) {
                return Ok(GuardDecision::Deny(reason));
            }

            // Scan tool description
            if let Some(desc) = &tool.description {
                if let Some(reason) = self.scan_text(desc) {
                    return Ok(GuardDecision::Deny(reason));
                }
            }

            // Scan input schema
            let schema_str = serde_json::to_string(&tool.input_schema)?;
            if let Some(reason) = self.scan_text(&schema_str) {
                return Ok(GuardDecision::Deny(reason));
            }
        }
        Ok(GuardDecision::Allow)
    }
}
```

See the actual implementation for production-ready code with full pattern sets and configuration options.

---

## Integration Patterns

### Pattern 1: Native Rust Guard (Implemented)

**Use Case**: Tool Poisoning, Rug Pull, Tool Shadowing, PII Detection

**Pros**:
- < 1ms latency overhead
- Direct access to MCP protocol structs
- Type-safe, compile-time guarantees

**Cons**:
- Requires Rust knowledge
- Needs recompilation for updates

```yaml
# Configuration example
security_guards:
  - id: tool-poisoning-detector
    type: tool_poisoning
    priority: 100
    failure_mode: fail_closed
    runs_on: [tools_list, response]
    strict_mode: true
```

### Pattern 2: WASM Guard (In Development)

**Use Case**: Custom detection logic, org-specific rules, third-party modules

**Pros**:
- Near-native performance (~5-10ms)
- Sandboxed execution
- Dynamic loading without recompilation
- Write in any language that compiles to WASM

**Cons**:
- More complex to develop than native
- Memory constraints

```yaml
# Configuration example (future)
security_guards:
  - id: custom-detector
    type: wasm
    module_path: /etc/agentgateway/guards/custom.wasm
    priority: 200
    failure_mode: fail_closed
    runs_on: [tools_list]
```

### Pattern 3: External Service (Future)

**Use Case**: ML-based analysis, external policy engines, SIEM integration

**Pros**:
- Full language flexibility
- Can leverage existing infrastructure
- Suitable for heavy computation

**Cons**:
- Higher latency (50-500ms)
- Network reliability concerns
- Operational complexity

See [mcp-security-guards-architecture.md](./mcp-security-guards-architecture.md) for detailed analysis of when to use each pattern.

---

## Configuration Schema

Guards are configured per MCP backend in the gateway configuration:

```yaml
# agentgateway config.yaml
backends:
  - mcp:
      targets:
        - name: github-mcp
          sse:
            endpoint: https://mcp.github.com/sse

      # Security guards for this backend
      security_guards:
        # Tier 1: Native Guards
        - id: tool-poisoning-detector
          type: tool_poisoning
          enabled: true
          priority: 100
          failure_mode: fail_closed
          runs_on: [tools_list, response]
          strict_mode: true
          custom_patterns:
            - "(?i)ignore\\s+all\\s+previous"

        - id: rug-pull-detector
          type: rug_pull
          enabled: true
          priority: 101
          failure_mode: fail_closed
          runs_on: [tools_list]
          mode: detect  # or "remove" to filter malicious tools

        - id: pii-guard
          type: pii
          enabled: true
          priority: 50
          runs_on: [request, response, tool_invoke]
          detect: [email, credit_card, ssn, phone]
          action: mask  # or "reject"
```

See [security-guards-config-example.yaml](./security-guards-config-example.yaml) for a comprehensive configuration reference.

---

## Directory Structure

```
crates/agentgateway/src/mcp/security/
├── mod.rs                        # Core types: GuardDecision, GuardContext, GuardExecutor
├── native/                       # Native Rust guard implementations
│   ├── mod.rs                    # NativeGuard trait definition
│   ├── tool_poisoning.rs         # Tool poisoning detection
│   ├── rug_pull.rs               # Rug pull detection
│   ├── tool_shadowing.rs         # Tool shadowing prevention
│   ├── server_whitelist.rs       # Server whitelisting
│   ├── pii_guard.rs              # PII detection and masking
│   └── pii_detection.rs          # PII pattern definitions
└── wasm.rs                       # WASM guard support (in development)
```

---

## Implementation Status

### Completed (Tier 1 Native Guards)
- [x] Core framework: `GuardDecision`, `GuardContext`, `GuardExecutor`
- [x] Tool Poisoning Detection (`tool_poisoning.rs`)
- [x] Rug Pull Detection (`rug_pull.rs`)
- [x] Tool Shadowing Prevention (`tool_shadowing.rs`)
- [x] Server Whitelisting (`server_whitelist.rs`)
- [x] PII Detection & Masking (`pii_guard.rs`)
- [x] YAML configuration support
- [x] Hot-reload via `GuardExecutorRegistry`

### In Progress
- [ ] WASM guard runtime integration
- [ ] WIT interface finalization

### Future
- [ ] External gRPC guard support
- [ ] Webhook guard support
- [ ] Guard metrics and observability

---

## Performance Considerations

### Latency Budget

| Hook Type | Target Latency | P99 Latency | Mitigation |
|-----------|---------------|-------------|------------|
| Native Rust | < 1ms | < 5ms | Optimize regex, use lazy statics |
| External gRPC | < 20ms | < 50ms | Connection pooling, circuit breaker |
| Webhook | < 50ms | < 200ms | Async execution, caching |

### Scaling Considerations

1. **Horizontal Scaling**: Hooks should be stateless or use external stores
2. **Caching**: Cache tool baselines, RBAC decisions, etc.
3. **Circuit Breaker**: Automatic failover when external services are down
4. **Rate Limiting**: Protect external services from overload
5. **Async Execution**: Non-critical hooks (audit, analytics) run async

---

## Security & Privacy

1. **Secrets Management**: Hook configs never log secrets
2. **PII Protection**: Automatically mask PII in audit logs
3. **Least Privilege**: Hooks only see data they need
4. **Audit Trail**: All security decisions are logged with correlation IDs
5. **Defense in Depth**: Multiple hooks can detect same threat

---

## Monitoring & Observability

### Metrics

```rust
// Prometheus metrics
security_hooks_total{hook_id, decision}
security_hooks_duration_seconds{hook_id}
security_hooks_errors_total{hook_id, error_type}
security_violations_total{threat_type, severity}
```

### Alerts

1. High rate of security violations
2. Hook execution failures
3. Unusual hook latency
4. External service unavailable

---

## Design Decisions

1. **Failure Mode Default**: `fail_closed` (block on error) - security over availability
2. **Tool Baselines**: In-memory per `GuardExecutor` instance (suitable for single-instance deployments)
3. **Guard Loading**: Native guards compiled-in; WASM guards loaded at runtime from filesystem
4. **Hot Reload**: Supported via `GuardExecutorRegistry` - config changes apply to existing sessions
5. **Logging**: All guard decisions logged via `tracing` at INFO level

## Open Questions

1. **Distributed Baselines**: For multi-instance deployments, should rug pull baselines be stored externally (Redis)?
2. **WASM Capabilities**: What host functions should be available to WASM guards?
3. **Metrics**: Should guard execution metrics be exposed via Prometheus endpoint?
