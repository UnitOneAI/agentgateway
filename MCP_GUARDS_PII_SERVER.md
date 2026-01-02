# MCP Security Guards & PII Test Server

**Branch:** `feature/mcp-security-guards`
**Status:** Development
**Last Updated:** 2026-01-02

## Overview

This document covers the MCP (Model Context Protocol) security guards implementation and the PII MCP Test Server integration in AgentGateway. This branch extends AgentGateway's MCP support with enhanced security features, proper SSE (Server-Sent Events) handling, and comprehensive testing infrastructure.

## Table of Contents

1. [Architecture](#architecture)
2. [MCP Backend Configuration](#mcp-backend-configuration)
3. [PII MCP Test Server](#pii-mcp-test-server)
4. [E2E Testing](#e2e-testing)
5. [Deployment](#deployment)
6. [Known Issues](#known-issues)
7. [Troubleshooting](#troubleshooting)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     AgentGateway                             │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐   │
│  │  UI (8080)  │  │ Admin API   │  │  MCP Routes      │   │
│  │  /ui        │  │  /config    │  │  /{mcp-name}     │   │
│  └─────────────┘  └─────────────┘  └──────────────────┘   │
│         │               │                    │              │
│         └───────────────┴────────────────────┘              │
│                         │                                    │
│                 ┌───────▼────────┐                          │
│                 │  MCP Routing   │                          │
│                 │  Engine        │                          │
│                 └───────┬────────┘                          │
│                         │                                    │
│          ┌──────────────┼──────────────┐                   │
│          │              │              │                    │
│     ┌────▼────┐   ┌────▼────┐   ┌────▼────┐              │
│     │  HTTP   │   │  STDIO  │   │ Stateful│              │
│     │ Targets │   │ Targets │   │  Mode   │              │
│     └────┬────┘   └─────────┘   └─────────┘              │
└──────────┼─────────────────────────────────────────────────┘
           │
      ┌────▼─────────────────────────────────────────┐
      │  External MCP Servers                        │
      │  - PII Test Server (HTTPS/SSE)               │
      │  - GitHub MCP Server                         │
      │  - Custom MCP Servers                        │
      └──────────────────────────────────────────────┘
```

## MCP Backend Configuration

### Configuration Structure

AgentGateway uses a YAML configuration file (`azure-config.yaml` for Azure Container Apps deployments) that defines MCP backends. The configuration follows a strict schema based on the `LocalMcpTargetSpec` Rust enum.

### LocalMcpTargetSpec Schema

The MCP backend configuration uses **mutually exclusive** variants:

```rust
enum LocalMcpTargetSpec {
    targets(Vec<McpTarget>),      // For HTTP/HTTPS MCP servers
    statefulMode(StatefulConfig),  // For stateful MCP servers
    prefixMode(PrefixConfig),      // For prefix-based routing
}
```

#### HTTP/HTTPS MCP Targets (Correct Structure)

```yaml
binds:
- port: 8080
  listeners:
  - routes:
    - name: mcp-route
      backends:
      - mcp:
          targets:                # ✅ REQUIRED: targets array
          - http:                 # ✅ CORRECT: http nested inside targets
              host: https://example.com/mcp
```

#### Common Configuration Error

```yaml
# ❌ INCORRECT - This will fail with parsing error
backends:
- mcp:
    http:                        # ❌ ERROR: http directly under mcp
      host: https://example.com/mcp
```

**Error Message:**
```
Error: binds[0].listeners[0].routes[N].backends[0]: unknown field `http`,
expected one of `targets`, `statefulMode`, `prefixMode` at line X column Y
```

### Complete Configuration Example

```yaml
# Azure Container App Configuration
# Exposes both UI and MCP on port 8080
binds:
- port: 8080
  listeners:
  - routes:
    # Admin UI route (must come before MCP routes)
    - name: ui-route
      matches:
      - path:
          pathPrefix: /ui
      backends:
      - host: 127.0.0.1:15000

    # Admin API route (for config management)
    - name: admin-api-route
      matches:
      - path:
          pathPrefix: /config
      backends:
      - host: 127.0.0.1:15000

    # MCP routes
    - name: pii-test-server-route
      backends:
      - mcp:
          targets:
          - http:
              host: https://mcp-pii-test-server.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp
```

### Key Configuration Requirements

1. **Targets Array**: For HTTP/HTTPS MCP servers, the `http:` configuration MUST be inside a `targets:` array
2. **Route Ordering**: UI and Admin routes must come before MCP routes to prevent path conflicts
3. **Host Format**: Use full URL including protocol (`https://` or `http://`) for HTTP targets
4. **Port Binding**: All services (UI, Admin API, MCP) share the same port (8080 in Azure Container Apps)

---

## PII MCP Test Server

The PII MCP Test Server is a FastMCP-based server designed for testing MCP integrations with sensitive data handling.

### Features

- **Random PII Generation**: Generate realistic test PII data on demand
- **Multiple PII Categories**:
  - Personal: name, email, phone, dob, address
  - Identity: SSN, driver's license, passport
  - Financial: credit card, bank account, tax ID
- **Bulk Generation**: Generate multiple records in a single request
- **Text Embedding**: Generate lorem ipsum text with embedded PII
- **Fixtures**: Pre-defined PII datasets for consistent testing
- **SSE Support**: Full Server-Sent Events support for MCP protocol

### Repository Structure

```
PiiMcpTest/
├── src/
│   └── mcp_test_server/
│       ├── fastmcp_server.py      # Main MCP server implementation
│       ├── generators/             # PII generators
│       │   ├── __init__.py
│       │   ├── personal.py        # Name, email, phone, etc.
│       │   ├── identity.py        # SSN, DL, passport
│       │   └── financial.py       # Credit cards, bank accounts
│       └── fixtures/               # Pre-defined test datasets
│           ├── __init__.py
│           ├── datasets.py
│           └── personal.py
├── Dockerfile                      # Container build configuration
├── pyproject.toml                  # Python package configuration
└── README.md
```

### Available MCP Tools

#### 1. `generate_pii`
Generate random PII of a specific type.

**Parameters:**
- `pii_type`: Type of PII (name, email, phone, dob, address, personal, ssn, drivers_license, passport, identity, credit_card, bank_account, tax_id, financial)

**Example:**
```json
{
  "tool": "generate_pii",
  "arguments": {
    "pii_type": "email"
  }
}
```

#### 2. `generate_bulk_pii`
Generate multiple PII records.

**Parameters:**
- `pii_type`: Type of PII to generate
- `count`: Number of records (1-100, default 5)

#### 3. `list_pii_types`
List all available PII types organized by category.

#### 4. `generate_full_record`
Generate a complete PII record with personal, identity, and financial data.

#### 5. `generate_text_with_pii`
Generate lorem ipsum text with embedded PII.

**Parameters:**
- `pii_type`: Type of PII to embed

### MCP Resources

- `pii://fixtures/personal` - Predefined personal PII test data
- `pii://fixtures/identity` - Predefined identity PII test data
- `pii://fixtures/financial` - Predefined financial PII test data
- `pii://fixtures/mixed` - Complete PII records

### FastMCP Configuration

The server is configured with SSE support enabled:

```python
# src/mcp_test_server/fastmcp_server.py:18
mcp = FastMCP("pii-test-server", host="0.0.0.0")
```

**Critical Note**: Do NOT add `json_response=True` parameter, as it disables SSE support required for MCP.

### Deployment

The PII MCP Test Server is deployed as an Azure Container App:

- **URL**: `https://mcp-pii-test-server.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp`
- **Port**: 8080
- **Protocol**: HTTP with SSE (Server-Sent Events)
- **Container Registry**: `agwimages.azurecr.io`
- **Image Tags**: `latest`, `sse-enabled`, `v1.0.0`

#### Building the Image

```bash
cd /path/to/PiiMcpTest
az acr build \
  --registry agwimages \
  --image mcp-pii-test-server:latest \
  --image mcp-pii-test-server:sse-enabled \
  --file Dockerfile \
  --platform linux/amd64 \
  .
```

#### Deploying to Azure

```bash
az containerapp update \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --image agwimages.azurecr.io/mcp-pii-test-server:sse-enabled
```

---

## E2E Testing

### Test Infrastructure

#### Direct HTTP Testing

Test file: `tests/e2e_pii_http.py`

Tests the PII MCP server directly using HTTP JSON-RPC transport without going through AgentGateway.

**Key Tests:**
1. MCP Initialize
2. List Tools
3. Generate Email PII
4. Generate SSN
5. Bulk PII Generation
6. Text with PII

#### Gateway SSE Testing

Test file: `tests/e2e_mcp_sse_test.py`

Tests MCP communication through AgentGateway using SSE transport.

**Key Tests:**
1. MCP Endpoint Connectivity
2. MCP Session Initialization
3. List Available Tools
4. Generate PII Data
5. Bulk Generation
6. Text Embedding

### Running E2E Tests

```bash
cd /path/to/agentgateway/tests

# Install dependencies
python3 -m venv test_venv
source test_venv/bin/activate  # On Windows: test_venv\Scripts\activate
pip install httpx

# Run direct HTTP test
python3 e2e_pii_http.py

# Run gateway SSE test
./test_venv/bin/python3 e2e_mcp_sse_test.py
```

### Expected Output (Successful)

```
============================================================
AgentGateway MCP over SSE - E2E Test Suite
============================================================

Target: https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io
Started: 2026-01-02T09:27:54.641116

[Test 1: MCP Endpoint Connectivity]
✓ MCP endpoint accessible
  Status: 200

[Test 2: MCP Session Initialization]
✓ MCP initialize successful
  Server: pii-test-server v1.0.0

[Test 3: List Available Tools]
✓ List tools successful
  Found 6 tools:
    - generate_pii
    - generate_bulk_pii
    - list_pii_types
    - generate_full_record
    - generate_text_with_pii
    - (resource endpoints)

[Test 4: Generate PII Data]
✓ Generate email successful
  Generated: {"email": "john.doe@example.com"}

[Test 5: Bulk PII Generation]
✓ Bulk generation successful
  Generated 5 records

[Test 6: Generate Text with PII]
✓ Generate text with PII successful
  Generated: Lorem ipsum... Call me at 555-1234...

============================================================
Results: 6 passed, 0 failed
============================================================
```

---

## Deployment

### Prerequisites

- Azure CLI installed and configured
- Access to Azure Container Registry: `agwimages.azurecr.io`
- Resource group: `mcp-gateway-dev-rg`
- Container Apps environment: `unitone-agw-env`

### Build AgentGateway Image

```bash
cd /path/to/agentgateway

# Commit configuration changes
git add azure-config.yaml
git commit -m "Update MCP configuration with PII test server"

# Build image (az acr build uses git archive, so changes must be committed)
az acr build \
  --registry agwimages \
  --image unitone-agentgateway:latest \
  --image unitone-agentgateway:mcp-backend-targets-fix \
  --file Dockerfile.acr \
  --platform linux/amd64 \
  .
```

### Deploy to Azure Container Apps

```bash
# Update container app with new image
az containerapp update \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --image agwimages.azurecr.io/unitone-agentgateway:latest

# Wait for deployment to stabilize (60-90 seconds)
sleep 60

# Verify deployment
az containerapp revision list \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --query "[?properties.active==\`true\`].{Name:name,Traffic:properties.trafficWeight,Health:properties.healthState,Created:properties.createdTime}" \
  --output table
```

### Automated Deployment Script

File: `tests/wait_and_deploy.sh`

```bash
#!/bin/bash
# Wait for build to complete, then deploy and test

cd /path/to/agentgateway/tests

echo "Waiting for build to complete..."
while true; do
  STATUS=$(az acr task logs --registry agwimages --run-id <run-id> 2>&1 | grep "Run Status" | tail -1)

  if echo "$STATUS" | grep -q "Succeeded"; then
    echo "✓ Build completed successfully!"
    break
  elif echo "$STATUS" | grep -q "Failed"; then
    echo "✗ Build failed!"
    exit 1
  fi

  sleep 15
done

echo "Deploying image..."
az containerapp update \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --image agwimages.azurecr.io/unitone-agentgateway:latest

echo "✓ Deployment complete!"

echo "Waiting 30 seconds for deployment to stabilize..."
sleep 30

echo "Running E2E tests..."
./test_venv/bin/python3 e2e_mcp_sse_test.py

echo "✓ All tasks completed!"
```

---

## Known Issues

### Issue 1: SSE Initialization Failure (422 Error)

**Status:** Under Investigation
**Severity:** High
**Last Observed:** 2026-01-02 17:27 UTC

#### Symptoms

```
[Test 1: MCP Endpoint Connectivity]
✓ MCP endpoint accessible
  Status: 422

[Test 2: MCP Session Initialization]
✗ MCP initialize: Expected response header Content-Type to contain 'text/event-stream', got ''
⚠ Skipping remaining tests - initialization failed
```

#### Details

- AgentGateway responds with HTTP 422 (Unprocessable Entity)
- Response is missing `Content-Type: text/event-stream` header
- Direct PII server tests pass (HTTP JSON-RPC works)
- Gateway SSE proxying appears to have runtime issues

#### Configuration Status

- ✅ Configuration syntax correct (`http:` properly nested in `targets:` array)
- ✅ Gateway starts successfully (no parsing errors)
- ✅ PII server is healthy and responding
- ❌ SSE proxying not working correctly

#### Attempted Fixes

1. **Fix 1:** Corrected configuration syntax (commit 48e30aa)
   - Changed from `mcp: → http:` to `mcp: → targets: → - http:`
   - Result: Fixed parsing error, but SSE issue persists

2. **Fix 2:** Verified PII server SSE support
   - Confirmed FastMCP configured without `json_response=True`
   - Direct HTTP tests successful
   - Result: Server is correct, issue is in gateway

3. **Fix 3:** Multiple deployments and verifications
   - Builds: ch15-ch19
   - Revisions: 0000023-0000027
   - Result: Issue persists across deployments

#### Current Hypothesis

The gateway may have an issue with:
- SSE header forwarding/transformation
- HTTP → SSE protocol upgrade handling
- MCP session initialization over SSE
- Content negotiation between client and MCP server

#### Workaround

Use direct HTTP JSON-RPC transport for testing:
```bash
python3 tests/e2e_pii_http.py
```

### Issue 2: Git Archive Behavior in Azure ACR Builds

**Status:** Resolved
**Severity:** Medium

#### Problem

`az acr build` uses `git archive` which only includes committed files, not uncommitted working directory changes.

#### Solution

Always commit configuration changes before building:
```bash
git add azure-config.yaml
git commit -m "Update configuration"
az acr build ...
```

---

## Troubleshooting

### Checking Gateway Logs

```bash
# View recent logs
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --tail 100

# Follow logs in real-time
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --tail 50 \
  --follow true

# Filter for specific patterns
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --tail 100 2>&1 | grep -E "(error|Error|pii-test-server|MCP)" -i
```

### Checking Build Status

```bash
# List recent builds
az acr task logs --registry agwimages 2>&1 | grep -E "Run ID|Status" | head -20

# Check specific build
az acr task logs --registry agwimages --run-id ch19 2>&1 | tail -50
```

### Checking Container App Health

```bash
# List active revisions
az containerapp revision list \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --query "[?properties.active==\`true\`].{Name:name,Health:properties.healthState,Provisioning:properties.provisioningState,Traffic:properties.trafficWeight}" \
  --output table

# Get detailed revision info
az containerapp show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --query "{LatestRevision:properties.latestRevisionName,LatestReady:properties.latestReadyRevisionName,Status:properties.runningStatus}"
```

### Testing MCP Endpoint Directly

```bash
# Test MCP initialize via curl
curl -i -X POST \
  "https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp/pii-test-server?sessionId=test123" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{
    "jsonrpc":"2.0",
    "id":1,
    "method":"initialize",
    "params":{
      "protocolVersion":"2024-11-05",
      "capabilities":{},
      "clientInfo":{"name":"test","version":"1.0.0"}
    }
  }'
```

### Common Configuration Errors

#### Error: Unknown field `http`

```
Error: binds[0].listeners[0].routes[2].backends[0]: unknown field `http`,
expected one of `targets`, `statefulMode`, `prefixMode`
```

**Solution:** Nest `http:` inside `targets:` array:
```yaml
backends:
- mcp:
    targets:      # Add this
    - http:       # Indent this
        host: ...
```

#### Error: 422 Unprocessable Entity

**Check:**
1. Is the MCP server URL correct and accessible?
2. Does the MCP server support SSE?
3. Are gateway logs showing any errors?

#### Error: Connection Refused

**Check:**
1. Is the backend MCP server running?
2. Is the URL/hostname correct?
3. Are there network/firewall issues?

---

## References

### MCP Protocol

- [MCP Specification](https://modelcontextprotocol.io/specification)
- [MCP Introduction](https://modelcontextprotocol.io/introduction)
- [FastMCP Documentation](https://github.com/jlowin/fastmcp)

### AgentGateway Documentation

- [Main Repository](https://github.com/agentgateway/agentgateway)
- [MCP Examples](https://github.com/agentgateway/agentgateway/tree/main/examples/mcp-authentication)
- [Architecture Overview](https://github.com/agentgateway/agentgateway/blob/main/architecture/README.md)

### Azure Resources

- Container Apps: `unitone-agentgateway`, `mcp-pii-test-server`
- Resource Group: `mcp-gateway-dev-rg`
- Container Registry: `agwimages.azurecr.io`
- Environment: `unitone-agw-env`

---

## Contributing

This is a development branch in a private fork. When making changes:

1. Test locally with direct PII server tests first
2. Commit all configuration changes before building
3. Build and tag images appropriately
4. Deploy to dev environment
5. Run E2E tests to verify
6. Document any issues or workarounds

## License

Same as AgentGateway: Apache 2.0

---

**Document Version:** 1.0
**Branch:** feature/mcp-security-guards
**Last Updated:** 2026-01-02
**Maintained By:** Unitone.ai Fork
