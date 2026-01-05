#!/usr/bin/env python3
"""
E2E Test - Direct PII MCP Server via HTTP JSON-RPC
Tests the PII MCP server directly using HTTP transport
"""

import asyncio
import json
import sys
import uuid
from typing import Any, Dict
from datetime import datetime

try:
    import httpx
except ImportError:
    print("Error: httpx not installed")
    print("Install with: pip install httpx")
    sys.exit(1)


class HTTPMCPClient:
    """MCP client using HTTP JSON-RPC transport."""

    def __init__(self, base_url: str):
        self.base_url = base_url.rstrip('/')
        self.session_id = str(uuid.uuid4())
        self.client = httpx.AsyncClient(timeout=30.0)
        self.message_id = 0

    async def close(self):
        await self.client.aclose()

    def _next_message_id(self) -> int:
        """Get next message ID for JSON-RPC."""
        self.message_id += 1
        return self.message_id

    async def _call(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Make JSON-RPC call."""
        url = f"{self.base_url}?sessionId={self.session_id}"

        message = {
            "jsonrpc": "2.0",
            "id": self._next_message_id(),
            "method": method,
            "params": params
        }

        try:
            response = await self.client.post(
                url,
                json=message,
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json"
                }
            )

            if response.status_code != 200:
                return {
                    "success": False,
                    "error": f"HTTP {response.status_code}: {response.text}"
                }

            data = response.json()

            if "result" in data:
                return {
                    "success": True,
                    "result": data["result"]
                }
            elif "error" in data:
                return {
                    "success": False,
                    "error": data["error"]
                }
            else:
                return {
                    "success": False,
                    "error": "Unexpected response format"
                }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

    async def initialize(self) -> Dict[str, Any]:
        """Initialize MCP session."""
        return await self._call("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "pii-http-e2e-test",
                "version": "1.0.0"
            }
        })

    async def list_tools(self) -> Dict[str, Any]:
        """List available tools."""
        result = await self._call("tools/list", {})
        if result["success"]:
            result["tools"] = result.get("result", {}).get("tools", [])
        return result

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool."""
        return await self._call("tools/call", {
            "name": tool_name,
            "arguments": arguments
        })


async def run_tests():
    """Run all E2E tests."""
    pii_server_url = "https://mcp-pii-test-server.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp"

    print("=" * 60)
    print("Direct PII MCP Server - HTTP JSON-RPC E2E Tests")
    print("=" * 60)
    print(f"\nTarget: {pii_server_url}")
    print(f"Started: {datetime.now().isoformat()}\n")

    passed = 0
    failed = 0
    errors = []

    client = HTTPMCPClient(pii_server_url)

    try:
        # Test 1: Initialize
        print("[Test 1: MCP Initialize]")
        init_result = await client.initialize()
        if init_result["success"]:
            server_info = init_result.get("result", {}).get("serverInfo", {})
            print(f"✓ Initialize successful")
            print(f"  Server: {server_info.get('name')} v{server_info.get('version')}")
            passed += 1
        else:
            print(f"✗ Initialize failed: {init_result.get('error')}")
            errors.append(f"Initialize: {init_result.get('error')}")
            failed += 1
            return {"passed": passed, "failed": failed}

        # Test 2: List Tools
        print("\n[Test 2: List Tools]")
        tools_result = await client.list_tools()
        if tools_result["success"]:
            tools = tools_result.get("tools", [])
            print(f"✓ List tools successful")
            print(f"  Found {len(tools)} tools:")
            for tool in tools:
                print(f"    - {tool.get('name')}: {tool.get('description', 'No description')[:60]}")
            passed += 1
        else:
            print(f"✗ List tools failed: {tools_result.get('error')}")
            errors.append(f"List tools: {tools_result.get('error')}")
            failed += 1

        # Test 3: Generate Email PII
        print("\n[Test 3: Generate Email PII]")
        email_result = await client.call_tool("generate_pii", {"pii_type": "email"})
        if email_result["success"]:
            content = email_result.get("result", {}).get("content", [{}])[0].get("text", "")
            print(f"✓ Generate email successful")
            print(f"  Generated: {content[:80]}")
            passed += 1
        else:
            print(f"✗ Generate email failed: {email_result.get('error')}")
            errors.append(f"Generate email: {email_result.get('error')}")
            failed += 1

        # Test 4: Generate SSN
        print("\n[Test 4: Generate SSN]")
        ssn_result = await client.call_tool("generate_pii", {"pii_type": "ssn"})
        if ssn_result["success"]:
            content = ssn_result.get("result", {}).get("content", [{}])[0].get("text", "")
            print(f"✓ Generate SSN successful")
            print(f"  Generated: {content[:80]}")
            passed += 1
        else:
            print(f"✗ Generate SSN failed: {ssn_result.get('error')}")
            errors.append(f"Generate SSN: {ssn_result.get('error')}")
            failed += 1

        # Test 5: Bulk Generation
        print("\n[Test 5: Bulk PII Generation]")
        bulk_result = await client.call_tool("generate_bulk_pii", {"pii_type": "name", "count": 5})
        if bulk_result["success"]:
            content = bulk_result.get("result", {}).get("content", [{}])[0].get("text", "")
            try:
                data = json.loads(content)
                count = len(data) if isinstance(data, list) else 1
                print(f"✓ Bulk generation successful")
                print(f"  Generated {count} records")
                passed += 1
            except:
                print(f"✓ Bulk generation successful (non-JSON response)")
                passed += 1
        else:
            print(f"✗ Bulk generation failed: {bulk_result.get('error')}")
            errors.append(f"Bulk generation: {bulk_result.get('error')}")
            failed += 1

        # Test 6: Text with PII
        print("\n[Test 6: Generate Text with PII]")
        text_result = await client.call_tool("generate_text_with_pii", {"pii_type": "phone"})
        if text_result["success"]:
            content = text_result.get("result", {}).get("content", [{}])[0].get("text", "")
            print(f"✓ Generate text with PII successful")
            print(f"  Generated: {content[:100]}")
            passed += 1
        else:
            print(f"✗ Generate text with PII failed: {text_result.get('error')}")
            errors.append(f"Generate text with PII: {text_result.get('error')}")
            failed += 1

    except Exception as e:
        print(f"\n✗ Unexpected error: {str(e)}")
        errors.append(f"Test execution: {str(e)}")
        failed += 1
        import traceback
        print(f"\nTraceback:\n{traceback.format_exc()}")

    finally:
        await client.close()

    # Summary
    print(f"\n{'=' * 60}")
    print(f"Results: {passed} passed, {failed} failed")

    if errors:
        print(f"\nFailed Tests:")
        for error in errors:
            print(f"  - {error}")

    print("=" * 60)
    print(f"Completed: {datetime.now().isoformat()}")

    return {"passed": passed, "failed": failed}


if __name__ == "__main__":
    results = asyncio.run(run_tests())
    sys.exit(0 if results["failed"] == 0 else 1)
