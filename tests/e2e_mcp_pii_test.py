#!/usr/bin/env python3
"""
E2E Test Suite for AgentGateway with PII MCP Test Server

Tests all MCP tools and resources through the AgentGateway.
"""

import asyncio
import json
import sys
from typing import Any, Dict, List
import httpx
from datetime import datetime


class MCPClient:
    """Simple MCP client for testing HTTP-based MCP servers."""

    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
        self.client = httpx.AsyncClient(timeout=30.0)

    async def close(self):
        await self.client.aclose()

    async def list_tools(self) -> List[Dict[str, Any]]:
        """List all available tools from the MCP server."""
        response = await self.client.post(
            f"{self.base_url}/mcp/v1/tools/list",
            json={
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            }
        )
        response.raise_for_status()
        result = response.json()
        return result.get("result", {}).get("tools", [])

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Any:
        """Call an MCP tool with the given arguments."""
        response = await self.client.post(
            f"{self.base_url}/mcp/v1/tools/call",
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
        response.raise_for_status()
        result = response.json()
        return result.get("result")

    async def list_resources(self) -> List[Dict[str, Any]]:
        """List all available resources from the MCP server."""
        response = await self.client.post(
            f"{self.base_url}/mcp/v1/resources/list",
            json={
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/list",
                "params": {}
            }
        )
        response.raise_for_status()
        result = response.json()
        return result.get("result", {}).get("resources", [])

    async def read_resource(self, uri: str) -> Any:
        """Read an MCP resource by its URI."""
        response = await self.client.post(
            f"{self.base_url}/mcp/v1/resources/read",
            json={
                "jsonrpc": "2.0",
                "id": 1,
                "method": "resources/read",
                "params": {
                    "uri": uri
                }
            }
        )
        response.raise_for_status()
        result = response.json()
        return result.get("result")


class TestResults:
    """Track test results."""

    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors: List[str] = []

    def add_pass(self, test_name: str):
        self.passed += 1
        print(f"✓ {test_name}")

    def add_fail(self, test_name: str, error: str):
        self.failed += 1
        self.errors.append(f"{test_name}: {error}")
        print(f"✗ {test_name}: {error}")

    def summary(self):
        total = self.passed + self.failed
        print(f"\n{'='*60}")
        print(f"Test Summary: {self.passed}/{total} passed")
        if self.failed > 0:
            print(f"\nFailed tests:")
            for error in self.errors:
                print(f"  - {error}")
        print(f"{'='*60}")
        return self.failed == 0


async def test_generate_pii(client: MCPClient, results: TestResults):
    """Test 1: generate_pii tool with various PII types."""
    print("\n[Test 1: generate_pii]")

    pii_types = ["name", "email", "phone", "ssn", "credit_card", "personal", "identity", "financial"]

    for pii_type in pii_types:
        try:
            result = await client.call_tool("generate_pii", {"pii_type": pii_type})
            if result and "content" in result:
                data = json.loads(result["content"][0]["text"])
                if data:
                    results.add_pass(f"generate_pii(pii_type={pii_type})")
                else:
                    results.add_fail(f"generate_pii(pii_type={pii_type})", "Empty result")
            else:
                results.add_fail(f"generate_pii(pii_type={pii_type})", "Invalid response format")
        except Exception as e:
            results.add_fail(f"generate_pii(pii_type={pii_type})", str(e))


async def test_generate_bulk_pii(client: MCPClient, results: TestResults):
    """Test 2: generate_bulk_pii with various counts."""
    print("\n[Test 2: generate_bulk_pii]")

    test_cases = [
        ("email", 5),
        ("ssn", 10),
        ("credit_card", 1),
        ("name", 50),
    ]

    for pii_type, count in test_cases:
        try:
            result = await client.call_tool("generate_bulk_pii", {
                "pii_type": pii_type,
                "count": count
            })
            if result and "content" in result:
                data = json.loads(result["content"][0]["text"])
                if isinstance(data, list) and len(data) == count:
                    results.add_pass(f"generate_bulk_pii(pii_type={pii_type}, count={count})")
                else:
                    results.add_fail(
                        f"generate_bulk_pii(pii_type={pii_type}, count={count})",
                        f"Expected {count} records, got {len(data) if isinstance(data, list) else 'non-list'}"
                    )
            else:
                results.add_fail(f"generate_bulk_pii(pii_type={pii_type}, count={count})", "Invalid response format")
        except Exception as e:
            results.add_fail(f"generate_bulk_pii(pii_type={pii_type}, count={count})", str(e))


async def test_list_pii_types(client: MCPClient, results: TestResults):
    """Test 3: list_pii_types tool."""
    print("\n[Test 3: list_pii_types]")

    try:
        result = await client.call_tool("list_pii_types", {})
        if result and "content" in result:
            data = json.loads(result["content"][0]["text"])
            if "categories" in data and isinstance(data["categories"], dict):
                results.add_pass("list_pii_types")
            else:
                results.add_fail("list_pii_types", "Invalid response structure")
        else:
            results.add_fail("list_pii_types", "Invalid response format")
    except Exception as e:
        results.add_fail("list_pii_types", str(e))


async def test_generate_full_record(client: MCPClient, results: TestResults):
    """Test 4: generate_full_record tool."""
    print("\n[Test 4: generate_full_record]")

    try:
        result = await client.call_tool("generate_full_record", {})
        if result and "content" in result:
            data = json.loads(result["content"][0]["text"])
            if "personal" in data and "identity" in data and "financial" in data:
                results.add_pass("generate_full_record")
            else:
                results.add_fail("generate_full_record", "Missing required fields (personal/identity/financial)")
        else:
            results.add_fail("generate_full_record", "Invalid response format")
    except Exception as e:
        results.add_fail("generate_full_record", str(e))


async def test_generate_text_with_pii(client: MCPClient, results: TestResults):
    """Test 5: generate_text_with_pii tool."""
    print("\n[Test 5: generate_text_with_pii]")

    pii_types = ["email", "ssn", "credit_card", "phone"]

    for pii_type in pii_types:
        try:
            result = await client.call_tool("generate_text_with_pii", {"pii_type": pii_type})
            if result and "content" in result:
                text = result["content"][0]["text"]
                if isinstance(text, str) and len(text) > 0:
                    results.add_pass(f"generate_text_with_pii(pii_type={pii_type})")
                else:
                    results.add_fail(f"generate_text_with_pii(pii_type={pii_type})", "Empty or invalid text")
            else:
                results.add_fail(f"generate_text_with_pii(pii_type={pii_type})", "Invalid response format")
        except Exception as e:
            results.add_fail(f"generate_text_with_pii(pii_type={pii_type})", str(e))


async def test_resources(client: MCPClient, results: TestResults):
    """Test 6: MCP resource access."""
    print("\n[Test 6: MCP Resources]")

    resource_uris = [
        "pii://fixtures/personal",
        "pii://fixtures/identity",
        "pii://fixtures/financial",
        "pii://fixtures/mixed",
    ]

    for uri in resource_uris:
        try:
            result = await client.read_resource(uri)
            if result and "contents" in result:
                data = json.loads(result["contents"][0]["text"])
                if data:
                    results.add_pass(f"read_resource({uri})")
                else:
                    results.add_fail(f"read_resource({uri})", "Empty fixture data")
            else:
                results.add_fail(f"read_resource({uri})", "Invalid response format")
        except Exception as e:
            results.add_fail(f"read_resource({uri})", str(e))


async def test_performance(client: MCPClient, results: TestResults):
    """Test 7: Performance test with bulk generation."""
    print("\n[Test 7: Performance Testing]")

    test_cases = [
        ("Bulk 10 records", "email", 10),
        ("Bulk 50 records", "name", 50),
        ("Bulk 100 records", "phone", 100),
    ]

    for test_name, pii_type, count in test_cases:
        try:
            start = datetime.now()
            result = await client.call_tool("generate_bulk_pii", {
                "pii_type": pii_type,
                "count": count
            })
            duration = (datetime.now() - start).total_seconds()

            if result and "content" in result:
                data = json.loads(result["content"][0]["text"])
                if isinstance(data, list) and len(data) == count:
                    results.add_pass(f"{test_name} ({duration:.2f}s)")
                else:
                    results.add_fail(test_name, f"Count mismatch: expected {count}, got {len(data)}")
            else:
                results.add_fail(test_name, "Invalid response format")
        except Exception as e:
            results.add_fail(test_name, str(e))


async def main():
    """Run all E2E tests."""
    print("="*60)
    print("AgentGateway + PII MCP Server - E2E Test Suite")
    print("="*60)

    # Configuration
    GATEWAY_URL = "https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io"

    print(f"\nTarget: {GATEWAY_URL}")
    print(f"Started: {datetime.now().isoformat()}\n")

    results = TestResults()
    client = MCPClient(GATEWAY_URL)

    try:
        # Run all test suites
        await test_generate_pii(client, results)
        await test_generate_bulk_pii(client, results)
        await test_list_pii_types(client, results)
        await test_generate_full_record(client, results)
        await test_generate_text_with_pii(client, results)
        await test_resources(client, results)
        await test_performance(client, results)

    finally:
        await client.close()

    # Print summary and exit
    success = results.summary()
    print(f"\nCompleted: {datetime.now().isoformat()}")
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())
