# MCP Security Guards: Multi-Tier Architecture

## Executive Summary

This document analyzes the three-tier security guard architecture for MCP protocol protection:

1. **Native Guards** (inline, compiled) - Core security, < 1ms latency
2. **WASM Guards** (runtime-loaded) - Extensible, sandboxed, ~5-10ms latency
3. **HTTP Hooks** (external services) - Enterprise integration, 50-500ms latency

**Key Finding**: All three tiers serve distinct purposes and are NOT overengineering when the goal is to build an enterprise-grade, extensible security platform. However, HTTP hooks should be **optional** and can be deprioritized for initial release.

---

## 1. Why Multi-Tier Architecture?

### The Extensibility Spectrum

```
┌─────────────────────────────────────────────────────────────┐
│                   Security Guard Continuum                   │
├──────────────┬──────────────────┬──────────────────────────┤
│   Native     │      WASM        │      HTTP Hooks          │
│  (Built-in)  │  (Pluggable)     │    (External)            │
├──────────────┼──────────────────┼──────────────────────────┤
│ < 1ms        │  5-10ms          │    50-500ms              │
│ Type-safe    │  Sandboxed       │    Language-agnostic     │
│ No updates   │  Hot-reload      │    Infinite flexibility  │
│ Zero trust   │  Medium trust    │    Full trust required   │
└──────────────┴──────────────────┴──────────────────────────┘
```

### Design Principles

1. **Performance Isolation**: Critical checks run native, heavy analysis runs external
2. **Security Isolation**: Untrusted code runs in WASM sandbox
3. **Operational Isolation**: Complex logic lives outside the gateway binary
4. **Vendor Neutrality**: Third parties can ship guards without forking

---

## 2. Tier 1: Native Guards (Inline/Compiled)

### What Are They?

Rust code compiled directly into the agentgateway binary. Implement the `NativeGuard` trait.

### Why Do We Need Them?

| Requirement | Why Native? |
|------------|-------------|
| **Performance** | Tool poisoning must check every tool with < 1ms overhead |
| **Reliability** | Core security CANNOT depend on external services |
| **Zero Dependencies** | Must work in air-gapped environments |
| **Type Safety** | Compile-time guarantees for critical paths |

### Scenarios

#### ✅ Use Native When:

1. **Performance-critical checks**
   - Tool poisoning detection (regex on every tool description)
   - Input validation (schema compliance)
   - Rate limiting (high-frequency decisions)

2. **Core security controls**
   - Authentication bypass attempts
   - Protocol compliance validation
   - Sandbox escape detection

3. **Simple, stable logic**
   - Pattern matching against known threats
   - Allowlist/denylist checks
   - Basic anomaly detection (e.g., tool count spike)

#### ❌ Don't Use Native When:

- Logic changes frequently (requires redeployment)
- Need customer-specific rules (multi-tenant)
- Requires external data (threat intel feeds)

### Implementation

**Location**: `crates/agentgateway/src/mcp/security/native/`

**Example**: Tool Poisoning Detector

```rust
// crates/agentgateway/src/mcp/security/native/tool_poisoning.rs
pub struct ToolPoisoningDetector {
    patterns: Vec<Regex>,
    strict_mode: bool,
}

impl NativeGuard for ToolPoisoningDetector {
    fn evaluate_tools_list(
        &self,
        tools: &[Tool],
        _context: &GuardContext,
    ) -> GuardResult {
        for tool in tools {
            if let Some(desc) = &tool.description {
                for pattern in &self.patterns {
                    if pattern.is_match(desc) {
                        return Ok(GuardDecision::Deny(DenyReason {
                            code: "tool_poisoning_detected".to_string(),
                            message: format!("Malicious pattern in tool '{}'", tool.name),
                            details: None,
                        }));
                    }
                }
            }
        }
        Ok(GuardDecision::Allow)
    }
}
```

**Performance**: Typical execution: 0.5ms for 50 tools with 10 regex patterns

---

## 3. Tier 2: WASM Guards (Runtime-Loaded)

### What Are They?

WebAssembly modules (`.wasm` files) loaded at runtime. Sandboxed execution with defined host interface.

### Why Do We Need Them?

| Requirement | Why WASM? |
|------------|-----------|
| **Hot Reload** | Update security rules without gateway restart |
| **Sandboxing** | Run untrusted code safely (third-party vendors) |
| **Multi-Tenancy** | Different rules per customer/deployment |
| **A/B Testing** | Test new detection logic on subset of traffic |
| **Portability** | Write guards in any language (Rust, Go, C++) |

### Scenarios

#### ✅ Use WASM When:

1. **Customer-specific rules**
   - Organization has custom compliance requirements
   - Need per-tenant security policies
   - Want to test new rules before baking into binary

2. **Third-party security modules**
   - Security vendor ships WASM module for their product
   - Community-contributed guards
   - Proprietary detection logic you can't open-source

3. **Frequent updates**
   - Threat patterns change weekly/daily
   - Experimental detection algorithms
   - Seasonal/temporary rules (e.g., Black Friday fraud)

4. **Language diversity**
   - Team prefers Go/TinyGo for guard logic
   - Existing security code in C/C++
   - Want to use AssemblyScript for simpler syntax

#### ❌ Don't Use WASM When:

- Latency budget < 5ms (use native)
- Need complex I/O (database queries, use HTTP hooks)
- Logic is stable and performance-critical (use native)

### Implementation

**Location**: `crates/agentgateway/src/mcp/security/wasm/`

**Architecture**:

```
┌─────────────────────────────────────────────────┐
│           AgentGateway (Rust/Host)              │
│  ┌───────────────────────────────────────────┐  │
│  │  WASM Runtime (wasmtime)                  │  │
│  │  ┌─────────────────────────────────────┐  │  │
│  │  │  Guard Module (guest.wasm)          │  │  │
│  │  │  - evaluate_tools_list()            │  │  │
│  │  │  - evaluate_tool_invoke()           │  │  │
│  │  └─────────────────────────────────────┘  │  │
│  │           ▲              │                 │  │
│  │           │ Host         │ Guest           │  │
│  │           │ Functions    │ Exports         │  │
│  │  ┌────────┴──────────────▼──────────────┐  │  │
│  │  │  Host Interface                      │  │  │
│  │  │  - log(level, message)               │  │  │
│  │  │  - get_metadata(key) -> value        │  │  │
│  │  └──────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

**Interface Definition** (WIT format):

```wit
// crates/agentgateway/src/mcp/security/wasm/guard.wit
package mcp:security-guard;

interface guard {
    record tool {
        name: string,
        description: option<string>,
        input-schema: string,  // JSON
    }

    record guard-context {
        server-name: string,
        identity: option<string>,
        metadata: string,  // JSON
    }

    variant decision {
        allow,
        deny(deny-reason),
        modify(string),  // JSON
    }

    record deny-reason {
        code: string,
        message: string,
        details: option<string>,
    }

    // Main guard function
    evaluate-tools-list: func(tools: list<tool>, context: guard-context) -> result<decision, string>;
}

// Host functions available to guest
interface host {
    log: func(level: u8, message: string);
    get-time: func() -> u64;
}

world security-guard {
    export guard;
    import host;
}
```

**Performance**: Typical execution: 5-10ms (includes module instantiation + execution)

### Is WASM Overengineering?

**NO** - Here's why:

1. **Industry Standard**: Envoy, Istio, Cloudflare Workers all use WASM for extensibility
2. **Real Demand**: Enterprises WILL ask "can I customize security rules?"
3. **Competitive Advantage**: Enables ecosystem of third-party security vendors
4. **Future-Proof**: WASM support unlocks countless use cases beyond security

**However**: Can ship v1 without WASM and add later (interface is already designed).

---

## 4. Tier 3: HTTP Hooks (External Services)

### What Are They?

HTTP/gRPC webhooks to external services. Gateway makes synchronous call before allowing operation.

### Why Do We Need Them?

| Requirement | Why HTTP? |
|------------|-----------|
| **ML Models** | Run TensorFlow/PyTorch models (can't fit in WASM) |
| **External Data** | Query threat intelligence databases |
| **Existing Systems** | Integrate with SIEM, SOC, ticketing systems |
| **Heavy Computation** | Complex analysis (graph algorithms, NLP) |
| **Language Freedom** | Use Python, Java, or any language |

### Scenarios

#### ✅ Use HTTP Hooks When:

1. **Machine Learning Detection**
   ```
   Tool invocation → HTTP POST → ML service → Anomaly score → Allow/Deny
   ```
   - Behavioral analysis (is this tool usage normal for this user?)
   - Natural language analysis (is this prompt injection?)
   - Time-series anomaly detection

2. **Threat Intelligence Integration**
   ```
   New tool appears → HTTP POST → ThreatDB → Check reputation → Allow/Deny
   ```
   - Check tool against known malicious signatures
   - Verify server certificates against revocation lists
   - Cross-reference with CVE databases

3. **Enterprise Integration**
   ```
   High-risk operation → HTTP POST → SOC approval → Ticket created → Manual review
   ```
   - Create audit trails in existing systems
   - Require human approval for sensitive operations
   - Integrate with identity providers (LDAP, AD)

4. **Dynamic Policy Evaluation**
   ```
   Request → HTTP POST → Policy engine (OPA, Casbin) → Decision
   ```
   - Centralized policy management across services
   - Complex RBAC with external attribute sources
   - Compliance-driven access control

#### ❌ Don't Use HTTP Hooks When:

- Latency budget < 50ms (use native or WASM)
- Network reliability is concern (failover complexity)
- Logic is simple enough for native/WASM
- Want to avoid operational complexity

### Implementation

**Location**: `crates/agentgateway/src/mcp/security/http/`

**Protocol** (inspired by Envoy ext_authz):

```rust
// Request to external service
POST /v1/security/evaluate HTTP/1.1
Content-Type: application/json

{
  "operation": "tools_list",
  "tools": [
    {
      "name": "github_search",
      "description": "Search GitHub repositories",
      "input_schema": {...}
    }
  ],
  "context": {
    "server_name": "github",
    "identity": "user@example.com",
    "metadata": {
      "request_id": "req-123",
      "timestamp": 1734567890
    }
  }
}

// Response from external service
HTTP/1.1 200 OK
Content-Type: application/json

{
  "decision": "deny",
  "reason": {
    "code": "ml_anomaly_detected",
    "message": "Unusual tool pattern detected (confidence: 0.92)",
    "details": {
      "anomaly_score": 0.92,
      "baseline_deviation": 3.5,
      "model_version": "v2.1.0"
    }
  }
}
```

**Performance**: 50-500ms depending on network + service latency

### Is HTTP Hooks Overengineering?

**MAYBE** - Critical analysis:

#### Arguments FOR:

1. **Enterprise Requirement**: Large orgs NEED to integrate with existing security infrastructure
2. **ML Use Case**: Can't run real ML models in WASM yet (model sizes are GB)
3. **Proven Pattern**: Envoy ext_authz is widely adopted

#### Arguments AGAINST:

1. **Complexity**: Adds network reliability concerns, retry logic, circuit breakers
2. **Performance**: 50-500ms is SLOW for synchronous security checks
3. **Operational Burden**: Now you're running two services instead of one
4. **Smaller Market**: Only 5-10% of users will actually use this

#### Recommendation:

**Make HTTP hooks OPTIONAL and deprioritize for v1**:

- Ship with Native + WASM support initially
- Add HTTP hooks later when customers explicitly request it
- Design the interface now, implement when needed

**Alternative**: Use WASM with async HTTP capabilities (WASI-HTTP) - guards can call external APIs from within WASM sandbox.

---

## 5. Decision Matrix: Which Tier to Use?

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Use Case                        │ Native │ WASM │ HTTP  │ Rationale   │
├──────────────────────────────────┼────────┼──────┼───────┼─────────────┤
│ Regex pattern matching           │   ✓    │      │       │ Performance │
│ Customer-specific rules          │        │  ✓   │       │ Multi-tenant│
│ ML-based anomaly detection       │        │      │   ✓   │ Model size  │
│ Tool poisoning (built-in)        │   ✓    │      │       │ Core        │
│ Tool poisoning (custom patterns) │        │  ✓   │       │ Flexibility │
│ Query threat intel database      │        │      │   ✓   │ External I/O│
│ Rate limiting                    │   ✓    │      │       │ Performance │
│ Compliance policy check          │        │  ✓   │   ✓   │ Depends     │
│ Third-party security vendor      │        │  ✓   │   ✓   │ Sandboxing  │
│ Prompt injection detection (NLP) │        │      │   ✓   │ Model size  │
│ A/B testing new detection logic  │        │  ✓   │       │ Hot reload  │
│ Create SIEM tickets              │        │      │   ✓   │ Integration │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Implementation Roadmap

### Phase 1: Core (v0.1) ✅ COMPLETED

- [x] Native guard framework
- [x] Tool Poisoning detector (native)
- [x] YAML configuration parsing
- [x] Guard executor with priority ordering
- [x] Failure mode handling (fail-open/fail-closed)

### Phase 2: Extensibility (v0.2) 📍 CURRENT

- [ ] WASM runtime integration (wasmtime)
- [ ] WIT interface definition
- [ ] Example WASM guard in Rust
- [ ] WASM loader with timeout/sandbox
- [ ] Documentation + tutorials

### Phase 3: Enterprise (v0.3) 🔮 FUTURE

- [ ] HTTP hook framework (optional)
- [ ] Circuit breaker for external services
- [ ] Metrics + observability
- [ ] Multi-hook composition
- [ ] Async WASM with WASI-HTTP (alternative to HTTP hooks)

---

## 7. Comparison with Industry Standards

### Envoy Filters

| Envoy | AgentGateway | Notes |
|-------|--------------|-------|
| C++ filters | Native guards | Compiled into binary |
| WASM filters | WASM guards | Runtime-loaded modules |
| ext_authz (HTTP) | HTTP hooks | External authorization |

**Takeaway**: Our architecture mirrors Envoy's proven extensibility model.

### Kong Plugins

| Kong | AgentGateway | Notes |
|------|--------------|-------|
| Lua plugins | WASM guards | Scripting layer |
| Go plugins | Native guards | Compiled plugins |
| Serverless functions | HTTP hooks | External execution |

**Difference**: Kong uses Lua (not sandboxed), we use WASM (sandboxed).

---

## 8. Critical Evaluation: What to Cut?

### Keep: Native Guards ✅

**Essential**: Core functionality, performance-critical, zero dependencies.

### Keep: WASM Guards ✅

**High Value**: Enables extensibility, industry standard, competitive advantage.

**Can defer**: Ship v1 with just native, add WASM in v0.2.

### Consider Cutting: HTTP Hooks ⚠️

**Reasons to cut**:
1. High complexity (network, retries, circuit breakers)
2. Performance concerns (latency)
3. Smaller market (< 10% of users)
4. Can be replaced by WASM + WASI-HTTP

**Reasons to keep**:
1. ML use case is compelling
2. Enterprise requirement for integration
3. Already designed, just needs implementation

**Recommendation**: Design the interface now, implement in v0.3+ when demand is proven.

---

## 9. Example: End-to-End WASM Guard

WASM guard examples will be added once the WASM runtime integration is complete. For now, refer to:
- [mcp-security-guards-contract.md](./mcp-security-guards-contract.md) for the WIT interface specification
- `crates/agentgateway/src/mcp/security/wasm.rs` for the runtime integration (in development)

---

## 10. Conclusion

### Summary

| Tier | Status | Priority | Complexity | Value |
|------|--------|----------|------------|-------|
| Native | ✅ Implemented | P0 | Low | Critical |
| WASM | 📍 In Progress | P1 | Medium | High |
| HTTP Hooks | 🔮 Future | P2 | High | Medium |

### Final Recommendations

1. **Ship v1 with Native guards only** - Get security features out quickly
2. **Add WASM in v0.2** - Enable extensibility before market demand
3. **Defer HTTP hooks to v0.3+** - Wait for explicit customer requests
4. **Design all interfaces now** - Keep options open for future

### Not Overengineering

The three-tier architecture is **well-justified** when building an enterprise-grade, extensible security platform. Each tier serves distinct use cases that cannot be effectively served by the others.

**However**, they should be rolled out **incrementally**:
- v0.1: Native (DONE)
- v0.2: Native + WASM
- v0.3: Native + WASM + HTTP (if needed)

This mirrors how successful infrastructure projects (Envoy, Istio, Cloudflare) evolved their extensibility models over time.
