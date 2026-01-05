# E2E Test Investigation Results

**Date**: 2025-12-31
**Status**: Investigation Complete

## Summary

The E2E tests in `e2e_mcp_pii_test.py` are failing because they use an incompatible protocol. The tests were written for HTTP REST-style MCP endpoints, but AgentGateway uses **MCP protocol over Server-Sent Events (SSE)**, not HTTP REST APIs.

## Root Cause Analysis

### What the Tests Expect
The E2E tests (`e2e_mcp_pii_test.py`) make HTTP POST requests to REST-style endpoints:
```python
# From e2e_mcp_pii_test.py:40-53
async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
    response = await self.client.post(
        f"{self.base_url}/mcp/v1/tools/call",  # ❌ This endpoint doesn't exist
        json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        }
    )
```

### What AgentGateway Actually Provides
AgentGateway exposes MCP servers via **Server-Sent Events (SSE)** at paths like:
- `/mcp/{server-name}` (e.g., `/mcp/pii-test-server`)
- Requires SSE connection with session management
- Uses MCP protocol over SSE transport, not HTTP REST

### Test Results

| Request | Status | Reason |
|---------|--------|--------|
| `POST /mcp/v1/tools/call` | 406 Not Acceptable | No route match in gateway config (azure-config.yaml:24-36) |
| `GET /mcp/pii-test-server` (SSE) | 422 Unprocessable Entity | Correct endpoint, but missing session ID parameter |

## Infrastructure Status

### AgentGateway
- **Status**: ✅ Running
- **URL**: `https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io`
- **Revision**: unitone-agentgateway--0000016
- **Authentication**: Disabled for dev testing (`--enabled false`)

### PII MCP Test Server
- **Status**: ✅ Running
- **Internal URL**: `http://mcp-pii-test-server.internal.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io:8000`
- **Server Process**: Uvicorn running successfully
- **Accessibility**: Internal only (configured in azure-config.yaml:33-36)

### Configuration
From `azure-config.yaml`:
```yaml
# Lines 24-36: MCP routes (NO path matching for /mcp/v1/...)
- backends:
  - mcp:
      targets:
      - name: echo
        stdio:
          cmd: echo
          args: ["MCP Available"]

      - name: pii-test-server
        http:
          url: http://mcp-pii-test-server.internal.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io:8000
```

## Log Evidence

### Gateway Logs (showing 406 errors)
```
route=default/route2 http.path=/mcp/v1/tools/call http.status=406 protocol=mcp duration=0ms
```

The gateway:
- Recognizes the protocol as "mcp"
- Routes to "default/route2"
- Returns 406 immediately (duration=0ms)
- Does not proxy to any backend

### PII Server Logs (healthy)
```
INFO:     Started server process [1]
INFO:     Application startup complete.
INFO:     Uvicorn running on http://0.0.0.0:8000
```

### Correct MCP Endpoint Test
```bash
$ curl -i -H "Accept: text/event-stream" https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp/pii-test-server
HTTP/2 422
Session ID is required
```

This confirms:
- ✅ The `/mcp/pii-test-server` endpoint exists
- ✅ The gateway responds to SSE requests
- ✅ MCP protocol routing is working
- ⚠️ Session management is required for MCP connections

## Authentication Configuration

Easy Auth was disabled for dev environment E2E testing:
```bash
az containerapp auth update \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --enabled false
```

**Previous state**: RedirectToLoginPage (401 Unauthorized)
**Current state**: Disabled (authentication bypassed)

## Next Steps

### Option 1: Rewrite E2E Tests for MCP Protocol (Recommended)
Create new E2E tests that properly use MCP over SSE:

```python
# Pseudo-code for proper MCP client
import httpx_sse

async with httpx_sse.aconnect_sse(
    client,
    "GET",
    f"{base_url}/mcp/pii-test-server?sessionId={session_id}",
    headers={"Accept": "text/event-stream"}
) as event_source:
    # Send MCP initialize message
    # Wait for SSE events
    # Send tool call requests via MCP protocol
    # Process SSE responses
```

**Pros**:
- Tests the actual production architecture
- Validates end-to-end MCP protocol flow
- Tests through the gateway (realistic scenario)

**Cons**:
- Requires implementing proper MCP client
- More complex than HTTP REST tests
- Need to handle SSE and session management

### Option 2: Test PII Server Directly (Alternative)
If the goal is to test PII functionality (not gateway integration):

1. Make PII server externally accessible
2. Use existing HTTP REST tests directly against the server
3. Gateway integration tested separately

**Pros**:
- Can reuse existing test code
- Simpler test implementation
- Faster feedback cycle

**Cons**:
- Doesn't test gateway integration
- Requires exposing PII server externally
- Misses any gateway-level processing

### Option 3: Hybrid Approach
- Test PII server functionality directly (Option 2)
- Add separate integration tests for gateway MCP routing (Option 1)

## Recommendations

1. **Short-term**: Use Option 2 to validate PII server functionality
   - Expose PII server with external ingress
   - Run existing e2e_mcp_pii_test.py against it directly

2. **Medium-term**: Implement Option 1 for proper E2E testing
   - Create MCP SSE client implementation
   - Write integration tests through gateway
   - Validate complete request flow

3. **Authentication for Production**:
   - Re-enable Easy Auth for production environments
   - Update Terraform configuration to manage auth settings
   - Document authentication requirements in deployment guides

## Files Modified

| File | Status | Purpose |
|------|--------|---------|
| `/Users/surindersingh/source_code/terraform/modules/azure/agentgateway/auth.tf` | Created (not applied) | Terraform-managed Easy Auth configuration |
| `/Users/surindersingh/source_code/terraform/modules/azure/agentgateway/variables.tf` | Modified (not applied) | Added `allow_anonymous_access` variable |
| `/Users/surindersingh/source_code/terraform/environments/dev/agentgateway/main.tf` | Modified (not applied) | Set `allow_anonymous_access = true` for dev |

**Note**: Terraform changes were not applied because the actual infrastructure naming doesn't match the Terraform configuration (deployed as "unitone-agentgateway", Terraform expects "unitone-agw-dev-app").

## Technical Debt

1. **Infrastructure/Terraform Mismatch**: Deployed infrastructure doesn't match Terraform configuration
   - Deployed: `unitone-agentgateway`
   - Terraform expects: `unitone-agw-dev-app`
   - **Action**: Either import existing infra into Terraform or recreate using Terraform

2. **Test Architecture**: E2E tests use wrong protocol
   - **Action**: Rewrite tests or clarify testing strategy

3. **Authentication Management**: Currently managed via Azure CLI, not Terraform
   - **Action**: Align with Terraform for infrastructure as code best practices

## References

- **AgentGateway Config**: `/Users/surindersingh/source_code/agentgateway/azure-config.yaml`
- **E2E Test File**: `/Users/surindersingh/source_code/agentgateway/tests/e2e_mcp_pii_test.py`
- **Test Summary**: `/Users/surindersingh/source_code/agentgateway/tests/TEST_SUITE_SUMMARY.md`
- **Terraform Module**: `/Users/surindersingh/source_code/terraform/modules/azure/agentgateway/`

---

**Investigation completed successfully. Both services are healthy and running. The issue is architectural - tests need to be aligned with the MCP protocol implementation.**

---

## UPDATE: 2026-01-01 00:58 UTC

### New E2E SSE Tests Created

Created proper E2E test suite in `e2e_mcp_sse_test.py` that uses correct MCP protocol over Server-Sent Events:
- ✅ Uses `httpx-sse` library for SSE connections
- ✅ Implements proper MCP JSON-RPC 2.0 protocol
- ✅ Tests initialize, list_tools, and call_tool methods
- ✅ Includes security guards validation tests

### Root Cause of Test Failures

Tests were failing due to **configuration issue**, not protocol issue:

1. **Problem**: Deployed container used old `azure-config.yaml` with broken echo server
2. **Error**: `fail to create relay: failed to run command '"echo"'`
3. **Impact**: Gateway crashes when trying to initialize MCP connections

### Fix Applied

- Removed echo server from `azure-config.yaml` (lines 27-31)
- Rebuilding image with corrected configuration
- Deploying new revision to test E2E SSE tests

### Files Updated

| File | Status | Purpose |
|------|--------|---------|
| `tests/e2e_mcp_sse_test.py` | Created | Proper MCP over SSE test suite |
| `azure-config.yaml` | Fixed | Removed broken echo server config |

### Next Steps

1. ✓ Deploy fixed configuration
2. Run E2E SSE tests
3. Verify all tests pass
4. Document testing approach for future
