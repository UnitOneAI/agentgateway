# E2E Testing Guide - AgentGateway with PII MCP Server

This guide explains how to run comprehensive end-to-end tests for the AgentGateway + PII MCP Server deployment.

## Overview

The E2E test suite validates the complete integration between:
- AgentGateway (external-facing gateway)
- PII MCP Test Server (internal MCP server)
- All MCP tools and resources

## Quick Start

### 1. Install Dependencies

```bash
cd /Users/surindersingh/source_code/agentgateway/tests

# Create virtual environment (optional but recommended)
python3 -m venv venv
source venv/bin/activate

# Install test dependencies
pip install -r requirements-e2e.txt
```

### 2. Run the Tests

```bash
# Make the test script executable
chmod +x e2e_mcp_pii_test.py

# Run the tests
python3 e2e_mcp_pii_test.py
```

Or use pytest:

```bash
pytest e2e_mcp_pii_test.py -v
```

## What Gets Tested

### Test Suite 1: generate_pii
Tests generation of individual PII types:
- Personal data: name, email, phone, address
- Identity data: SSN, driver's license, passport
- Financial data: credit card, bank account, tax ID
- Aggregate types: personal, identity, financial

**Total tests**: 8

### Test Suite 2: generate_bulk_pii
Tests bulk generation with various counts:
- Small batch (5 records)
- Medium batch (10 records)
- Single record (1 record)
- Large batch (50 records)

**Total tests**: 4

### Test Suite 3: list_pii_types
Tests the tool that lists all available PII categories and types.

**Total tests**: 1

### Test Suite 4: generate_full_record
Tests generation of complete PII records containing personal, identity, and financial data.

**Total tests**: 1

### Test Suite 5: generate_text_with_pii
Tests generation of lorem ipsum text with embedded PII:
- Text with email
- Text with SSN
- Text with credit card
- Text with phone number

**Total tests**: 4

### Test Suite 6: MCP Resources
Tests access to predefined fixture data:
- `pii://fixtures/personal`
- `pii://fixtures/identity`
- `pii://fixtures/financial`
- `pii://fixtures/mixed`

**Total tests**: 4

### Test Suite 7: Performance Testing
Tests bulk generation performance:
- 10 records
- 50 records
- 100 records (maximum allowed)

**Total tests**: 3

**Grand Total**: 25 tests

## Expected Output

```
============================================================
AgentGateway + PII MCP Server - E2E Test Suite
============================================================

Target: https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io
Started: 2025-12-31T19:30:00.123456

[Test 1: generate_pii]
✓ generate_pii(pii_type=name)
✓ generate_pii(pii_type=email)
✓ generate_pii(pii_type=phone)
✓ generate_pii(pii_type=ssn)
✓ generate_pii(pii_type=credit_card)
✓ generate_pii(pii_type=personal)
✓ generate_pii(pii_type=identity)
✓ generate_pii(pii_type=financial)

[Test 2: generate_bulk_pii]
✓ generate_bulk_pii(pii_type=email, count=5)
✓ generate_bulk_pii(pii_type=ssn, count=10)
✓ generate_bulk_pii(pii_type=credit_card, count=1)
✓ generate_bulk_pii(pii_type=name, count=50)

[Test 3: list_pii_types]
✓ list_pii_types

[Test 4: generate_full_record]
✓ generate_full_record

[Test 5: generate_text_with_pii]
✓ generate_text_with_pii(pii_type=email)
✓ generate_text_with_pii(pii_type=ssn)
✓ generate_text_with_pii(pii_type=credit_card)
✓ generate_text_with_pii(pii_type=phone)

[Test 6: MCP Resources]
✓ read_resource(pii://fixtures/personal)
✓ read_resource(pii://fixtures/identity)
✓ read_resource(pii://fixtures/financial)
✓ read_resource(pii://fixtures/mixed)

[Test 7: Performance Testing]
✓ Bulk 10 records (0.45s)
✓ Bulk 50 records (1.23s)
✓ Bulk 100 records (2.15s)

============================================================
Test Summary: 25/25 passed
============================================================

Completed: 2025-12-31T19:30:15.654321
```

## Customizing Tests

### Change Target URL

Edit the `GATEWAY_URL` in `e2e_mcp_pii_test.py`:

```python
GATEWAY_URL = "https://your-gateway.azurecontainerapps.io"
```

### Add Custom Tests

Add new test functions following this pattern:

```python
async def test_custom_feature(client: MCPClient, results: TestResults):
    """Test custom MCP feature."""
    print("\n[Test X: Custom Feature]")

    try:
        result = await client.call_tool("tool_name", {"param": "value"})
        if result:
            results.add_pass("Custom test")
        else:
            results.add_fail("Custom test", "Unexpected result")
    except Exception as e:
        results.add_fail("Custom test", str(e))
```

Then call it in `main()`:

```python
await test_custom_feature(client, results)
```

## Troubleshooting

### Connection Errors

If you get connection errors:

1. Verify the gateway is running:
```bash
curl -i https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/ui
```

2. Check Container App logs:
```bash
az containerapp logs show \
  --name unitone-agentgateway \
  --resource-group mcp-gateway-dev-rg \
  --tail 50
```

### Timeout Errors

If tests timeout:

1. Increase timeout in the MCPClient constructor:
```python
self.client = httpx.AsyncClient(timeout=60.0)  # Increase from 30
```

2. Check PII server is running:
```bash
az containerapp logs show \
  --name mcp-pii-test-server \
  --resource-group mcp-gateway-dev-rg \
  --tail 50
```

### Invalid Response Format

If you get "Invalid response format" errors:

1. Check the MCP protocol version compatibility
2. Verify the gateway is routing to the correct MCP target
3. Check server logs for errors

## CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'

      - name: Install dependencies
        run: |
          pip install -r tests/requirements-e2e.txt

      - name: Run E2E tests
        run: |
          python3 tests/e2e_mcp_pii_test.py
```

## Testing Against Different Environments

### Dev Environment (Default)
```bash
# Already configured in the script
python3 e2e_mcp_pii_test.py
```

### Prod Environment
```python
# Edit GATEWAY_URL in e2e_mcp_pii_test.py
GATEWAY_URL = "https://your-prod-gateway.azurecontainerapps.io"
```

### Local Testing
If running AgentGateway locally:
```python
GATEWAY_URL = "http://localhost:8080"
```

## Next Steps

After all tests pass:

1. **Add Security Guards Testing**: Test PII detection and filtering
2. **Add Load Testing**: Use `locust` or similar for load testing
3. **Add Integration Tests**: Test with real MCP clients
4. **Monitor in Production**: Set up Application Insights alerts

## Related Documentation

- **PII Server Code**: `/Users/surindersingh/source_code/PiiMcpTest/src/mcp_test_server/fastmcp_server.py`
- **AgentGateway Config**: `/Users/surindersingh/source_code/agentgateway/azure-config.yaml`
- **Deployment Guide**: `/Users/surindersingh/source_code/terraform/environments/dev/agentgateway/README.md`
