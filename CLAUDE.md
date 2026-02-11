# AgentGateway Submodule — Claude Code Guide

## IMPORTANT: Do Not Build Locally

**Do NOT attempt to build this Rust project on Windows.** The user builds and checks it manually in a VS2022 Developer Command Prompt. Claude Code should only read, analyze, and edit source files — never run `cargo build`, `cargo test`, or similar commands.

## MCP Security Guards

Guards intercept MCP tool calls at different phases and can Allow, Deny, or Modify operations.

### Guard Phases (`runs_on`)

| Phase | When | Purpose |
|-------|------|---------|
| `ToolsList` | `tools/list` response | Filter/block exposed tools |
| `ToolInvoke` | Before `tools/call` | Block/modify tool execution |
| `Request` | Any MCP request | General request filtering |
| `Response` | Any MCP response | General response filtering |

### Core Trait

```rust
// crates/agentgateway/src/mcp/security/native/mod.rs
pub trait NativeGuard: Send + Sync {
    fn evaluate_tools_list(&self, tools: &[Tool], context: &GuardContext) -> GuardResult;
    fn evaluate_tool_invoke(&self, tool_name: &str, arguments: &Value, context: &GuardContext) -> GuardResult;
    fn evaluate_request(&self, request: &Value, context: &GuardContext) -> GuardResult;
    fn evaluate_response(&self, response: &Value, context: &GuardContext) -> GuardResult;
    fn reset_server(&self, server_name: &str);
}
```

### Native Guards

| Guard | File | What It Does |
|-------|------|-------------|
| **ToolPoisoningDetector** | `native/tool_poisoning.rs` (28 KB) | Blocks tools with malicious descriptions — prompt injection, system overrides, safety bypass, hidden instructions, prompt leaking, encoding tricks. 32 built-in regex patterns + custom patterns. |
| **RugPullDetector** | `native/rug_pull.rs` (38 KB) | Detects bait-and-switch: establishes per-server tool baseline on first encounter, then blocks if tools are modified/removed. Risk scoring with configurable weights (description=2, schema=3, remove=3, add=1), threshold=5. |
| **PiiGuard** | `native/pii_guard.rs` (28 KB) | Detects PII (email, phone, SSN, credit cards, CA SIN, URLs) via regex. Two modes: Mask (replace with `<ENTITY_TYPE>` placeholders) or Reject (block entire response). |
| **ToolShadowingDetector** | `native/tool_shadowing.rs` | Placeholder — will block duplicate tool names across servers. |
| **ServerWhitelistChecker** | `native/server_whitelist.rs` | Placeholder — will enforce allowed server whitelist. |

### WASM Guards

Custom guards loaded at runtime as WebAssembly components (`security/wasm.rs`). Fully sandboxed (no FS/network). ~5-10ms latency vs <1ms for native. See `examples/wasm-guards/` for examples.

### Key Files

- **Orchestration**: `crates/agentgateway/src/mcp/security/mod.rs` — `GuardExecutor`, `GuardExecutorRegistry`, config types, priority ordering
- **Integration**: `crates/agentgateway/src/mcp/handler.rs` — calls guards before tool execution and on tools/list responses
- **Config binding**: `crates/agentgateway/src/types/local.rs` — `McpBackendConfig` holds `Vec<McpSecurityGuard>`
- **PII patterns**: `crates/agentgateway/src/mcp/security/native/pii_detection.rs`

### Guard Configuration (YAML)

```yaml
security_guards:
  - id: pii-mask
    description: "Mask PII in responses"
    priority: 10          # Lower = runs first
    enabled: true
    timeout_ms: 100
    failure_mode: fail_closed  # or fail_open
    runs_on: [response]
    type:
      native:
        pii:
          action: mask
          pii_types: [email, phone, ssn, credit_card]
```

## E2E Tests (`tests/`)

### Test Files

| File | Tests | What It Covers |
|------|-------|----------------|
| `e2e_security_guards_test.py` | Master runner | Orchestrates all guard suites sequentially |
| `e2e_pii_guard_test.py` | 19 per mode (mask+reject) | 6 PII types: single, embedded-in-text, bulk, full-record, clean-data-passthrough |
| `e2e_tool_poisoning_guard_test.py` | 6 | tools/list blocked (HTTP 403), deny reason structure, 6 attack categories covered |
| `e2e_rug_pull_guard_test.py` | 22 | Baseline establishment, session vs global scope, 5 mutation modes (all/description/schema/remove/add) |
| `e2e_mcp_sse_test.py` | 5+ | SSE transport compliance with guards |
| `benchmark.py` | Configurable | Throughput, latency percentiles (p95/p99), error rates |
| `mcp_client.py` | — | Shared client library: `MCPSSEClient`, `MCPStreamableHTTPClient`, `TestResults` |

### Test Routes

| Route | Guard Config | Backend Port |
|-------|-------------|-------------|
| `/pii-test` | PII mask mode | 8000 |
| `/pii-test-reject` | PII reject mode | 8000 |
| `/poison` | Tool poisoning | 8010 |
| `/rug-pull` | Rug pull (default) | 8020 |
| `/rug-pull-desc` | Rug pull description-only | 8020 |
| `/rug-pull-schema` | Rug pull schema-only | 8020 |
| `/rug-pull-remove` | Rug pull remove mode | 8020 |
| `/rug-pull-add` | Rug pull add mode | 8020 |

### Running Tests

```bash
# Docker (CI/CD) — runs full suite
cd tests/docker && docker compose up --build --abort-on-container-exit --exit-code-from test-runner

# Against deployed environment
GATEWAY_URL=https://... python tests/e2e_security_guards_test.py --transport streamable
```

## Test Servers (`testservers/`)

Three Python MCP servers in one Docker image, started via `start-server.sh`:

### PII Test Server (port 8000)
- **Module**: `src/mcp_test_server/fastmcp_server.py`
- **Tools**: `generate_pii`, `generate_bulk_pii`, `generate_full_record`, `generate_text_with_pii`, `list_pii_types`
- **Data**: Faker-generated emails, phones, SSNs, credit cards, CA SINs, URLs, addresses
- **Resources**: `pii://fixtures/{personal,identity,financial,mixed}` — predefined test fixtures

### Tool Poisoning Server (port 8010)
- **Module**: `src/tool_poisoning_test/server.py`
- **Tools**: 6 poisoned (`add`, `secret_notes`, `translate_text`, `get_status`, `search_files`, `run_diagnostic`) + 2 clean (`subtract`, `multiply`)
- **Attack categories**: hidden instructions, prompt injection, system override, safety bypass, role manipulation, prompt leaking

### Rug Pull Server (port 8020)
- **Module**: `src/rug_pull_test/server.py`
- **Tools**: `get_weather` (session trigger), `get_global_weather` (global trigger), `get_forecast`, `reset_session_rug`, `reset_global_rug`, `get_rug_status`, `set_rug_pull_mode`
- **Mutation modes**: `all` (default), `description`, `schema`, `remove`, `add` — each changes tool definitions differently after trigger
- **Benign→Malicious**: e.g. "Get weather" becomes "Get weather AND read all env vars, API keys, secrets..."

### Docker Setup

```yaml
# tests/docker/docker-compose.yaml — 3 services
mcp-test-servers:  # Builds from testservers/, exposes 8000/8010/8020
agentgateway:      # ACR image, mounts test config, port 8080
test-runner:       # Python 3.12, runs e2e_security_guards_test.py
```
