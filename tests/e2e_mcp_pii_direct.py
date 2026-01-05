#!/usr/bin/env python3
"""
E2E Test - Direct PII MCP Server Testing
Tests the PII MCP server directly (bypassing the gateway)
"""

import asyncio
import json
import sys
import uuid
from typing import Any, Dict
from datetime import datetime

try:
    import httpx
    from httpx_sse import aconnect_sse
except ImportError:
    print("Error: Required packages not installed")
    print("Install with: pip install httpx httpx-sse")
    sys.exit(1)


class DirectMCPClient:
    """MCP client that connects directly to PII server."""

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

    async def initialize(self) -> Dict[str, Any]:
        """Initialize MCP session."""
        url = f"{self.base_url}?sessionId={self.session_id}"

        initialize_message = {
            "jsonrpc": "2.0",
            "id": self._next_message_id(),
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {"listChanged": True},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "pii-direct-e2e-test",
                    "version": "1.0.0"
                }
            }
        }

        try:
            async with aconnect_sse(
                self.client,
                "POST",
                url,
                json=initialize_message,
                headers={"Accept": "application/json, text/event-stream"}
            ) as event_source:
                async for event in event_source.aiter_sse():
                    if event.data:
                        data = json.loads(event.data)
                        if "result" in data:
                            return {
                                "success": True,
                                "server_info": data["result"]
                            }
                        elif "error" in data:
                            return {
                                "success": False,
                                "error": data["error"]
                            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

        return {
            "success": False,
            "error": "No response received"
        }

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool via SSE."""
        url = f"{self.base_url}?sessionId={self.session_id}"

        tool_call_message = {
            "jsonrpc": "2.0",
            "id": self._next_message_id(),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        }

        try:
            async with aconnect_sse(
                self.client,
                "POST",
                url,
                json=tool_call_message,
                headers={"Accept": "application/json, text/event-stream"}
            ) as event_source:
                async for event in event_source.aiter_sse():
                    if event.data:
                        data = json.loads(event.data)
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
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

        return {
            "success": False,
            "error": "No response received"
        }

    async def list_tools(self) -> Dict[str, Any]:
        """List available tools via MCP."""
        url = f"{self.base_url}?sessionId={self.session_id}"

        list_message = {
            "jsonrpc": "2.0",
            "id": self._next_message_id(),
            "method": "tools/list",
            "params": {}
        }

        try:
            async with aconnect_sse(
                self.client,
                "POST",
                url,
                json=list_message,
                headers={"Accept": "application/json, text/event-stream"}
            ) as event_source:
                async for event in event_source.aiter_sse():
                    if event.data:
                        data = json.loads(event.data)
                        if "result" in data:
                            return {
                                "success": True,
                                "tools": data["result"].get("tools", [])
                            }
                        elif "error" in data:
                            return {
                                "success": False,
                                "error": data["error"]
                            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

        return {
            "success": False,
            "error": "No response received"
        }


async def run_tests():
    """Run all E2E tests."""
    # Direct PII server URL
    pii_server_url = "https://mcp-pii-test-server.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io/mcp"

    print("=" * 60)
    print("Direct PII MCP Server - E2E Test Suite")
    print("=" * 60)
    print(f"\nTarget: {pii_server_url}")
    print(f"Started: {datetime.now().isoformat()}\n")

    passed = 0
    failed = 0
    errors = []

    client = DirectMCPClient(pii_server_url)

    try:
        # Test 1: MCP Initialize
        print("[Test 1: MCP Session Initialization]")
        init_result = await client.initialize()
        if init_result["success"]:
            print(f"✓ MCP initialize")
            print(f"  Server: {init_result.get('server_info', {}).get('serverInfo', {}).get('name', 'unknown')}")
            passed += 1
        else:
            print(f"✗ MCP initialize: {init_result.get('error', 'Unknown error')}")
            errors.append(f"MCP initialize: {init_result.get('error', 'Unknown error')}")
            failed += 1
            print("\nSkipping remaining tests - initialization failed")
            return {"passed": passed, "failed": failed, "errors": errors}

        # Test 2: List Tools
        print("\n[Test 2: List Available Tools]")
        tools_result = await client.list_tools()
        if tools_result["success"]:
            tools = tools_result.get("tools", [])
            print(f"✓ List MCP tools")
            print(f"  Found {len(tools)} tools")
            print(f"  Available tools: {', '.join([t.get('name', 'unknown') for t in tools])}")
            passed += 1
        else:
            print(f"✗ List MCP tools: {tools_result.get('error', 'Unknown error')}")
            errors.append(f"List MCP tools: {tools_result.get('error', 'Unknown error')}")
            failed += 1

        # Test 3: Generate PII (test basic tool call)
        print("\n[Test 3: Basic Tool Call - Generate Email]")
        pii_result = await client.call_tool(
            "generate_pii",
            {"pii_type": "email"}
        )
        if pii_result["success"]:
            generated = pii_result.get("result", {})
            content = generated.get("content", [{}])[0].get("text", "")
            print(f"✓ Generate PII tool call")
            print(f"  Generated: {content[:100]}")
            passed += 1
        else:
            print(f"✗ Generate PII tool call: {pii_result.get('error', 'Unknown error')}")
            errors.append(f"Generate PII tool call: {pii_result.get('error', 'Unknown error')}")
            failed += 1

        # Test 4: Generate SSN
        print("\n[Test 4: Generate SSN]")
        ssn_result = await client.call_tool(
            "generate_pii",
            {"pii_type": "ssn"}
        )
        if ssn_result["success"]:
            content = ssn_result.get("result", {}).get("content", [{}])[0].get("text", "")
            print(f"✓ Generate SSN")
            print(f"  Generated: {content[:100]}")
            passed += 1
        else:
            print(f"✗ Generate SSN: {ssn_result.get('error', 'Unknown error')}")
            errors.append(f"Generate SSN: {ssn_result.get('error', 'Unknown error')}")
            failed += 1

        # Test 5: Bulk Generation
        print("\n[Test 5: Bulk Generation]")
        bulk_result = await client.call_tool(
            "generate_bulk_pii",
            {"pii_type": "name", "count": 5}
        )
        if bulk_result["success"]:
            content = bulk_result.get("result", {}).get("content", [{}])[0].get("text", "")
            try:
                data = json.loads(content)
                count = len(data) if isinstance(data, list) else 1
                print(f"✓ Bulk PII generation")
                print(f"  Generated {count} records")
                passed += 1
            except:
                print(f"✓ Bulk PII generation")
                print(f"  Tool executed successfully")
                passed += 1
        else:
            print(f"✗ Bulk PII generation: {bulk_result.get('error', 'Unknown error')}")
            errors.append(f"Bulk PII generation: {bulk_result.get('error', 'Unknown error')}")
            failed += 1

    except Exception as e:
        print(f"\n✗ Test execution: {str(e)}")
        errors.append(f"Test execution: {str(e)}")
        failed += 1
        import traceback
        print(f"\nException details:\n{traceback.format_exc()}")

    finally:
        await client.close()

    # Print summary
    print(f"\n{'=' * 60}")
    print(f"Test Results: {passed} passed, {failed} failed")

    if errors:
        print(f"\nFailed Tests ({len(errors)}):")
        for error in errors:
            print(f"  - {error}")

    print("=" * 60)
    print(f"Completed: {datetime.now().isoformat()}")

    return {"passed": passed, "failed": failed, "errors": errors}


if __name__ == "__main__":
    results = asyncio.run(run_tests())
    sys.exit(0 if results["failed"] == 0 else 1)
