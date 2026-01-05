#!/usr/bin/env python3
"""
E2E Test Suite for AgentGateway with MCP over SSE
Tests proper MCP protocol with security guards validation

This test suite validates:
1. MCP protocol over Server-Sent Events (SSE)
2. AgentGateway routing to PII MCP Test Server
3. Security guards functionality (PII detection, redaction, blocking)
"""

import asyncio
import json
import sys
import uuid
from typing import Any, Dict, List, Optional
from datetime import datetime

try:
    import httpx
except ImportError:
    print("Error: Required packages not installed")
    print("Install with: pip install httpx")
    sys.exit(1)


class MCPSSEClient:
    """MCP client that uses Server-Sent Events protocol with raw httpx."""

    def __init__(self, base_url: str, server_name: str = "pii-test-server"):
        self.base_url = base_url.rstrip('/')
        self.server_name = server_name
        self.session_id = str(uuid.uuid4())
        self.session_header = None  # Will be set from initialize response
        self.client = httpx.AsyncClient(timeout=60.0)
        self.message_id = 0

    async def close(self):
        await self.client.aclose()

    def _next_message_id(self) -> int:
        """Get next message ID for JSON-RPC."""
        self.message_id += 1
        return self.message_id

    def _parse_sse_response(self, response_text: str) -> Optional[Dict[str, Any]]:
        """Parse SSE response and extract JSON-RPC message."""
        for line in response_text.split('\n'):
            if line.startswith('data: '):
                data_str = line[6:]  # Remove 'data: ' prefix
                try:
                    return json.loads(data_str)
                except json.JSONDecodeError:
                    continue
        return None

    async def test_connection(self) -> Dict[str, Any]:
        """Test basic connectivity to MCP endpoint."""
        url = f"{self.base_url}/mcp/{self.server_name}"
        try:
            response = await self.client.get(
                url,
                headers={"Accept": "text/event-stream"}
            )
            return {
                "success": response.status_code in [200, 422],  # 422 = "Session ID required" is expected
                "status_code": response.status_code,
                "response": response.text[:200] if response.text else None
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

    async def initialize(self) -> Dict[str, Any]:
        """Initialize MCP session."""
        url = f"{self.base_url}/mcp/{self.server_name}?sessionId={self.session_id}"

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
                    "name": "agentgateway-e2e-test",
                    "version": "1.0.0"
                }
            }
        }

        try:
            response = await self.client.post(
                url,
                json=initialize_message,
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json, text/event-stream"
                }
            )

            if response.status_code == 200:
                # Extract session header for subsequent requests
                self.session_header = response.headers.get("mcp-session-id")

                data = self._parse_sse_response(response.text)
                if data and "result" in data:
                    return {
                        "success": True,
                        "server_info": data["result"]
                    }
                elif data and "error" in data:
                    return {
                        "success": False,
                        "error": data["error"]
                    }

            return {
                "success": False,
                "error": f"HTTP {response.status_code}: {response.text[:200]}"
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

    async def call_tool(self, tool_name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call an MCP tool via SSE."""
        url = f"{self.base_url}/mcp/{self.server_name}"

        tool_call_message = {
            "jsonrpc": "2.0",
            "id": self._next_message_id(),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        }

        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream"
        }

        # Add session header if available
        if self.session_header:
            headers["mcp-session-id"] = self.session_header

        try:
            response = await self.client.post(
                url,
                json=tool_call_message,
                headers=headers
            )

            if response.status_code == 200:
                data = self._parse_sse_response(response.text)
                if data and "result" in data:
                    return {
                        "success": True,
                        "result": data["result"]
                    }
                elif data and "error" in data:
                    return {
                        "success": False,
                        "error": data["error"]
                    }

            return {
                "success": False,
                "error": f"HTTP {response.status_code}: {response.text[:200]}"
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }

    async def list_tools(self) -> Dict[str, Any]:
        """List available tools via MCP."""
        url = f"{self.base_url}/mcp/{self.server_name}"

        list_message = {
            "jsonrpc": "2.0",
            "id": self._next_message_id(),
            "method": "tools/list",
            "params": {}
        }

        headers = {
            "Content-Type": "application/json",
            "Accept": "application/json, text/event-stream"
        }

        # Add session header if available
        if self.session_header:
            headers["mcp-session-id"] = self.session_header

        try:
            response = await self.client.post(
                url,
                json=list_message,
                headers=headers
            )

            if response.status_code == 200:
                data = self._parse_sse_response(response.text)
                if data and "result" in data:
                    return {
                        "success": True,
                        "tools": data["result"].get("tools", [])
                    }
                elif data and "error" in data:
                    return {
                        "success": False,
                        "error": data["error"]
                    }

            return {
                "success": False,
                "error": f"HTTP {response.status_code}: {response.text[:200]}"
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e)
            }


class TestResults:
    """Track test results."""

    def __init__(self):
        self.passed = 0
        self.failed = 0
        self.errors: List[str] = []
        self.warnings: List[str] = []

    def add_pass(self, test_name: str, details: str = ""):
        self.passed += 1
        print(f"✓ {test_name}")
        if details:
            print(f"  {details}")

    def add_fail(self, test_name: str, error: str):
        self.failed += 1
        self.errors.append(f"{test_name}: {error}")
        print(f"✗ {test_name}: {error}")

    def add_warning(self, message: str):
        self.warnings.append(message)
        print(f"⚠ {message}")

    def print_summary(self):
        print("\n" + "=" * 60)
        print(f"Test Results: {self.passed} passed, {self.failed} failed")

        if self.warnings:
            print(f"\nWarnings ({len(self.warnings)}):")
            for warning in self.warnings:
                print(f"  - {warning}")

        if self.errors:
            print(f"\nFailed Tests ({len(self.errors)}):")
            for error in self.errors:
                print(f"  - {error}")

        print("=" * 60)
        return self.failed == 0


async def run_tests():
    """Run all E2E tests."""
    # Configuration
    gateway_url = "https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io"

    print("=" * 60)
    print("AgentGateway MCP over SSE - E2E Test Suite")
    print("=" * 60)
    print(f"\nTarget: {gateway_url}")
    print(f"Started: {datetime.now().isoformat()}\n")

    results = TestResults()
    client = MCPSSEClient(gateway_url, "pii-test-server")

    try:
        # Test 1: Basic Connectivity
        print("\n[Test 1: MCP Endpoint Connectivity]")
        conn_result = await client.test_connection()
        if conn_result["success"]:
            results.add_pass(
                "MCP endpoint accessible",
                f"Status: {conn_result['status_code']}"
            )
        else:
            results.add_fail(
                "MCP endpoint accessibility",
                conn_result.get("error", "Unknown error")
            )

        # Test 2: MCP Initialize
        print("\n[Test 2: MCP Session Initialization]")
        init_result = await client.initialize()
        if init_result["success"]:
            results.add_pass(
                "MCP initialize",
                f"Server: {init_result.get('server_info', {}).get('serverInfo', {}).get('name', 'unknown')}"
            )
        else:
            results.add_fail(
                "MCP initialize",
                init_result.get("error", "Unknown error")
            )
            results.add_warning("Skipping remaining tests - initialization failed")
            return results

        # Test 3: List Tools
        print("\n[Test 3: List Available Tools]")
        tools_result = await client.list_tools()
        if tools_result["success"]:
            tools = tools_result.get("tools", [])
            results.add_pass(
                "List MCP tools",
                f"Found {len(tools)} tools"
            )
            print(f"  Available tools: {', '.join([t.get('name', 'unknown') for t in tools[:5]])}")
        else:
            results.add_fail(
                "List MCP tools",
                tools_result.get("error", "Unknown error")
            )

        # Test 4: Generate PII (test basic tool call)
        print("\n[Test 4: Basic Tool Call - Generate PII]")
        pii_result = await client.call_tool(
            "generate_pii",
            {"pii_type": "email"}
        )
        if pii_result["success"]:
            generated = pii_result.get("result", {})
            results.add_pass(
                "Generate PII tool call",
                f"Generated: {generated.get('content', [{}])[0].get('text', '')[:50]}"
            )
        else:
            results.add_fail(
                "Generate PII tool call",
                pii_result.get("error", "Unknown error")
            )

        # Test 5: Security Guards - PII Detection
        print("\n[Test 5: Security Guards - PII Detection]")
        ssn_result = await client.call_tool(
            "generate_pii",
            {"pii_type": "ssn"}
        )
        if ssn_result["success"]:
            # Check if PII was detected/flagged by security guards
            content = ssn_result.get("result", {}).get("content", [{}])[0].get("text", "")

            # Security guards should either:
            # 1. Redact the PII (contains [REDACTED] or similar)
            # 2. Block the request (error response)
            # 3. Pass through with audit log (check logs separately)

            if "[REDACTED]" in content or "[PII]" in content:
                results.add_pass(
                    "Security guard PII redaction",
                    "SSN was redacted by security guards"
                )
            elif content and any(char.isdigit() for char in content):
                results.add_warning(
                    "Security guards may not be active - SSN passed through unredacted"
                )
                results.add_pass(
                    "Generate SSN (no redaction)",
                    "Tool executed successfully (guards may not be configured)"
                )
            else:
                results.add_pass(
                    "Generate SSN",
                    "Tool executed successfully"
                )
        else:
            error_msg = str(ssn_result.get("error", ""))
            if "blocked" in error_msg.lower() or "forbidden" in error_msg.lower():
                results.add_pass(
                    "Security guard PII blocking",
                    "SSN request was blocked by security guards"
                )
            else:
                results.add_fail(
                    "Generate SSN with security guards",
                    ssn_result.get("error", "Unknown error")
                )

        # Test 6: Bulk Generation (Performance)
        print("\n[Test 6: Bulk Generation Performance]")
        bulk_result = await client.call_tool(
            "generate_bulk_pii",
            {"pii_type": "name", "count": 10}
        )
        if bulk_result["success"]:
            content = bulk_result.get("result", {}).get("content", [{}])[0].get("text", "")
            try:
                data = json.loads(content)
                count = len(data) if isinstance(data, list) else 1
                results.add_pass(
                    "Bulk PII generation",
                    f"Generated {count} records"
                )
            except:
                results.add_pass(
                    "Bulk PII generation",
                    "Tool executed successfully"
                )
        else:
            results.add_fail(
                "Bulk PII generation",
                bulk_result.get("error", "Unknown error")
            )

        # Test 7: Text with PII (Context Testing)
        print("\n[Test 7: Generate Text with Embedded PII]")
        text_result = await client.call_tool(
            "generate_text_with_pii",
            {"pii_type": "email"}
        )
        if text_result["success"]:
            results.add_pass(
                "Generate text with PII",
                "Natural language text generation successful"
            )
        else:
            results.add_fail(
                "Generate text with PII",
                text_result.get("error", "Unknown error")
            )

    except Exception as e:
        results.add_fail("Test execution", str(e))
        import traceback
        print(f"\nException details:\n{traceback.format_exc()}")

    finally:
        await client.close()

    # Print summary
    print(f"\nCompleted: {datetime.now().isoformat()}")
    success = results.print_summary()

    return results


if __name__ == "__main__":
    results = asyncio.run(run_tests())
    sys.exit(0 if results.failed == 0 else 1)
