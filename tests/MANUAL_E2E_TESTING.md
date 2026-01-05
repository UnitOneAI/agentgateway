# Manual E2E Testing Guide

Since the AgentGateway MCP routing requires specific protocol knowledge, here's a guide for manual end-to-end testing.

## Quick Connectivity Test

First, verify both services are accessible:

```bash
# Test 1: AgentGateway is running
curl -i https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/ui
# Expected: HTTP/2 401 (OAuth required) or 200 OK

# Test 2: Check AgentGateway logs for PII server target
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --tail 50 | grep -i "pii"

# Test 3: Verify PII server is running
az containerapp logs show \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --tail 20
# Expected: "Uvicorn running on http://0.0.0.0:8000"
```

## MCP Protocol Testing

The MCP protocol uses Server-Sent Events (SSE) for communication. Here's how to test:

### Step 1: Test Direct PII Server (Internal Network)

From within the Container App environment:

```bash
# This would only work from within the container app network
# For testing, check logs instead
az containerapp logs show \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --follow --tail 100
```

### Step 2: Test Through AgentGateway

The exact endpoint structure depends on AgentGateway's MCP routing implementation. Typical patterns:

```bash
# Option 1: Direct target access
curl -i https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp/pii-test-server

# Option 2: SSE endpoint
curl -i -H "Accept: text/event-stream" \
  https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp/pii-test-server

# Option 3: Check for MCP listing endpoint
curl -i https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp
```

## Verifying Cross-Component Integration

### Check AgentGateway Configuration

```bash
# View loaded configuration in logs
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --tail 100 | grep -E "(target|mcp|pii-test-server)"
```

### Monitor Real-Time Requests

```bash
# Terminal 1: Watch AgentGateway logs
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --follow

# Terminal 2: Watch PII Server logs
az containerapp logs show \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --follow

# Terminal 3: Make requests and observe routing
curl -i https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp/pii-test-server
```

## Testing Individual PII Tools (Manual)

Once you have the correct MCP endpoint working, test each tool:

### 1. generate_pii

Expected to generate random PII data of a specific type.

### 2. generate_bulk_pii

Expected to generate multiple PII records (1-100).

### 3. list_pii_types

Expected to list all available PII categories.

### 4. generate_full_record

Expected to generate complete PII record (personal + identity + financial).

### 5. generate_text_with_pii

Expected to generate lorem ipsum with embedded PII.

## Testing MCP Resources

Resources should be accessible via:
- `pii://fixtures/personal`
- `pii://fixtures/identity`
- `pii://fixtures/financial`
- `pii://fixtures/mixed`

## Troubleshooting

### Issue: Cannot connect to gateway

```bash
# Check gateway status
az containerapp show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.{ProvisioningState:provisioningState,RunningState:runningStatus}"
```

### Issue: PII server not responding

```bash
# Check PII server status
az containerapp show \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.{ProvisioningState:provisioningState,RunningState:runningStatus}"

# Restart if needed
az containerapp revision restart \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg
```

### Issue: 404 on MCP endpoints

Check AgentGateway configuration:

```bash
# Get the latest revision
az containerapp revision list \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --query "[?properties.active].{Name:name,Image:properties.template.containers[0].image}" \
  -o table

# Verify azure-config.yaml was baked into the image correctly
# This requires examining the Dockerfile build logs
```

## Next Steps

Once manual testing confirms the MCP routing is working:

1. Document the exact endpoint URLs that work
2. Update the automated E2E test script (e2e_mcp_pii_test.py) with correct URLs
3. Add authentication if required
4. Run automated tests: `python3 e2e_mcp_pii_test.py`

## Reference

- **AgentGateway FQDN**: https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io
- **PII Server Internal**: http://mcp-pii-test-server.internal.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io:8000
- **Config File**: /Users/surindersingh/source_code/agentgateway/azure-config.yaml (lines 33-36)
