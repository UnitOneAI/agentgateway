#!/usr/bin/env python3
"""
Simple E2E test for MCP over SSE using raw httpx (no httpx_sse library)
"""
import asyncio
import json
import sys
import uuid

try:
    import httpx
except ImportError:
    print("Error: httpx not installed")
    print("Install with: pip install httpx")
    sys.exit(1)


async def test_mcp_initialize():
    """Test MCP initialize with raw httpx POST"""
    base_url = "https://unitone-agentgateway.whitecliff-a0c9f0f7.eastus2.azurecontainerapps.io"
    session_id = str(uuid.uuid4())

    url = f"{base_url}/mcp/pii-test-server?sessionId={session_id}"

    initialize_message = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "raw-test",
                "version": "1.0.0"
            }
        }
    }

    async with httpx.AsyncClient(timeout=60.0) as client:
        try:
            response = await client.post(
                url,
                json=initialize_message,
                headers={
                    "Content-Type": "application/json",
                    "Accept": "application/json, text/event-stream"
                }
            )

            print(f"Status: {response.status_code}")
            print(f"Headers: {dict(response.headers)}")
            print(f"Content:\n{response.text[:500]}")

            if response.status_code == 200:
                # Parse SSE response
                for line in response.text.split('\n'):
                    if line.startswith('data: '):
                        data_str = line[6:]  # Remove 'data: ' prefix
                        try:
                            data = json.loads(data_str)
                            if "result" in data:
                                print(f"\n✓ SUCCESS: Initialize worked!")
                                print(f"Server info: {data['result'].get('serverInfo', {}).get('name', 'unknown')}")
                                return 0
                        except json.JSONDecodeError:
                            pass

            print(f"\n✗ FAIL: HTTP {response.status_code}")
            return 1

        except Exception as e:
            print(f"\n✗ ERROR: {e}")
            import traceback
            traceback.print_exc()
            return 1


if __name__ == "__main__":
    sys.exit(asyncio.run(test_mcp_initialize()))
