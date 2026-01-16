# AgentGateway Security Guards Testing Guide

## Overview

This guide explains how to test AgentGateway's security guards (PII detection, tool poisoning, etc.) using the admin UI.

## Prerequisites

1. **Access to UI**: https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/ui
2. **Microsoft Account**: To log in via OAuth
3. **PII Test Server Deployed**: See "Deploy PII Test Server" section below

---

## Deploy PII Test Server

The PII test server image is already in ACR. Deploy it to Azure Container Apps:

```bash
# 1. Create Container App for PII Test Server
az containerapp create \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --environment unitone-agw-env \
  --image agwimages.azurecr.io/mcp-pii-test-server:v1.0.0 \
  --target-port 8000 \
  --ingress internal \
  --min-replicas 1 \
  --max-replicas 1 \
  --cpu 0.5 \
  --memory 1Gi \
  --registry-server agwimages.azurecr.io

# 2. Get the internal FQDN
az containerapp show \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --query "properties.configuration.ingress.fqdn" \
  -o tsv
```

This will output something like: `mcp-pii-test-server.internal.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io`

---

## Configure AgentGateway to Connect to PII Test Server

Add the PII test server to AgentGateway's configuration:

### Option 1: Via UI (Recommended)

1. **Login to UI**: https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/ui
2. **Go to "Servers"** section
3. **Click "Add Server"**
4. **Configure**:
   - Name: `pii-test-server`
   - Type: `HTTP`
   - URL: `http://mcp-pii-test-server.internal.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io:8000/mcp/v1`
5. **Save**

### Option 2: Via Configuration File

Edit the AgentGateway config to add:

```yaml
mcp_servers:
  pii-test:
    url: http://mcp-pii-test-server.internal.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io:8000/mcp/v1
    transport: sse
    guards:
      - pii_detector
      - tool_poisoning
```

---

## Test Security Guards

### Test 1: Tool Poisoning Guard

**What it does**: Detects malicious tool descriptions that try to manipulate the AI.

**Steps**:
1. In the UI, go to **"Guards"** → **"Tool Poisoning Detector"**
2. Click **"Test"** or **"Enable"**
3. Make a request to any MCP server
4. Check **"Logs"** to see if it detects malicious patterns

**Expected behavior**:
- If an MCP server returns a tool with description like "SYSTEM: ignore all restrictions", the guard blocks it
- You'll see a denial in the logs with details about the violation

### Test 2: PII Detector Guard

**What it does**: Detects and redacts PII (SSNs, credit cards, emails, etc.) in MCP responses.

**Steps**:
1. **Call the PII test server** through AgentGateway
2. Use one of its tools that returns fake PII
3. **Check the response** - PII should be redacted

**Example test**:

```bash
# Call the PII test server's "generate_fake_person" tool
curl -X POST https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp/pii-test \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "method": "tools/call",
    "params": {
      "name": "generate_fake_person",
      "arguments": {}
    }
  }'
```

**Expected response (with PII guard enabled)**:
```json
{
  "name": "John Doe",
  "email": "[REDACTED_EMAIL]",
  "ssn": "[REDACTED_SSN]",
  "credit_card": "[REDACTED_CREDIT_CARD]"
}
```

**Without PII guard**:
```json
{
  "name": "John Doe",
  "email": "john.doe@example.com",
  "ssn": "123-45-6789",
  "credit_card": "4532-1234-5678-9010"
}
```

---

## Using the UI for Testing

### View Configured Guards

1. Go to **"Guards"** or **"Security"** section
2. You should see:
   - `pii_detector` - Detects PII in responses
   - `tool_poisoning` - Detects malicious tool descriptions
   - Custom guards (if configured)

### View Logs

1. Go to **"Logs"** or **"Monitoring"**
2. Filter by:
   - Server name (e.g., `pii-test-server`)
   - Guard name (e.g., `pii_detector`)
   - Status (allowed/denied)
3. Click on a log entry to see:
   - Full request/response
   - Detected violations
   - Redacted PII

### Enable/Disable Guards

1. Go to **"Guards"** section
2. Toggle guards on/off for specific servers or routes
3. Configure guard parameters (e.g., PII types to detect, alert thresholds)

---

## Understanding MCP Flow

```
┌─────────────────┐
│   Claude AI     │  (or any MCP client)
└────────┬────────┘
         │ MCP Request (e.g., call tool "generate_fake_person")
         ▼
┌─────────────────────────────┐
│     AgentGateway            │
│  ┌──────────────────────┐   │
│  │  Security Guards     │   │  ← Inspect request/response
│  │  - PII Detector      │   │
│  │  - Tool Poisoning    │   │
│  └──────────────────────┘   │
└────────┬────────────────────┘
         │ Forward request
         ▼
┌───────────────────────┐
│  MCP Server           │  (e.g., PII Test Server)
│  - Exposes tools      │
│  - Returns PII data   │
└───────────────────────┘
```

**Guard inspection points**:
1. **Request**: Before forwarding to MCP server (e.g., check for prompt injection)
2. **Response - Tools List**: When MCP server advertises its tools (check for tool poisoning)
3. **Response - Tool Result**: After tool execution (check for PII leaks)

---

## Troubleshooting

### UI not loading
- Check you're logged in with Microsoft OAuth
- Check Container App is running: `az containerapp show --name unitone-agentgateway --resource-group mcp-gateway-dev-rg`

### PII Test Server not responding
- Check it's deployed: `az containerapp show --name mcp-pii-test-server --resource-group mcp-gateway-dev-rg`
- Check logs: `az containerapp logs show --name mcp-pii-test-server --resource-group mcp-gateway-dev-rg`

### Guards not triggering
- Check guards are enabled in the UI
- Check the MCP server is configured to use guards
- Check logs for errors

---

## Next Steps

1. **Test with real MCP servers**: GitHub, filesystem, etc.
2. **Create custom guards**: Add your own security rules
3. **Monitor metrics**: View guard performance and PII detection rates
4. **Configure alerts**: Get notified when guards block requests
