#!/usr/bin/env python3
"""
Example MCP client for CAGE
Demonstrates how to use the Model Context Protocol to interact with CAGE
"""

import asyncio
import json
import websockets
import uuid

class CAGEMCPClient:
    def __init__(self, url="ws://localhost:8080/mcp", user_id="example_user"):
        self.url = url
        self.user_id = user_id
        self.ws = None

    async def connect(self):
        """Connect to CAGE MCP WebSocket"""
        self.ws = await websockets.connect(self.url)

        # Send initialize
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "user_id": self.user_id,
                "clientInfo": {
                    "name": "CAGE MCP Client",
                    "version": "1.0.0"
                }
            }
        }

        await self.ws.send(json.dumps(init_request))
        response = json.loads(await self.ws.recv())

        if "error" in response:
            raise Exception(f"Initialize failed: {response['error']}")

        print(f"Connected to CAGE MCP Server")
        print(f"Server: {response['result']['serverInfo']}")
        return response

    async def list_tools(self):
        """List available tools"""
        request = {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }

        await self.ws.send(json.dumps(request))
        response = json.loads(await self.ws.recv())

        if "error" in response:
            raise Exception(f"Error: {response['error']}")

        return response['result']['tools']

    async def execute_code(self, code, language="python", persistent=False, timeout=30):
        """Execute code via MCP tool"""
        request = {
            "jsonrpc": "2.0",
            "id": str(uuid.uuid4()),
            "method": "tools/call",
            "params": {
                "name": "execute_code",
                "arguments": {
                    "code": code,
                    "language": language,
                    "persistent": persistent,
                    "timeout_seconds": timeout
                }
            }
        }

        await self.ws.send(json.dumps(request))
        response = json.loads(await self.ws.recv())

        if "error" in response:
            return {"error": response['error']['message']}

        return response['result']

    async def list_files(self, path="/"):
        """List files via MCP"""
        request = {
            "jsonrpc": "2.0",
            "id": str(uuid.uuid4()),
            "method": "tools/call",
            "params": {
                "name": "list_files",
                "arguments": {"path": path}
            }
        }

        await self.ws.send(json.dumps(request))
        response = json.loads(await self.ws.recv())

        if "error" in response:
            return {"error": response['error']['message']}

        return response['result']

    async def close(self):
        """Close connection"""
        if self.ws:
            await self.ws.close()


async def main():
    """Demo MCP client usage"""
    client = CAGEMCPClient(user_id="demo_user")

    try:
        # Connect
        await client.connect()

        # List available tools
        tools = await client.list_tools()
        print(f"\nAvailable tools: {len(tools)}")
        for tool in tools:
            print(f"  - {tool['name']}: {tool['description']}")

        # Execute Python code
        print("\n--- Executing Python code ---")
        result = await client.execute_code("print('Hello from MCP!')")
        print(f"Output: {result['content'][0]['text']}")

        # Execute code with persistent interpreter
        print("\n--- Testing persistent mode ---")
        result1 = await client.execute_code("x = 42", persistent=True)
        print(f"Set x=42: {result1}")

        result2 = await client.execute_code("print(f'x is {x}')", persistent=True)
        print(f"Retrieved x: {result2['content'][0]['text']}")

        # List files
        print("\n--- Listing files ---")
        files = await client.list_files()
        print(f"Files: {files}")

    finally:
        await client.close()

if __name__ == "__main__":
    asyncio.run(main())
