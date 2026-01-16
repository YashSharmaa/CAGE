"""CAGE MCP (Model Context Protocol) Client"""

import asyncio
import json
import uuid
from typing import Dict, List, Optional, Any

try:
    import websockets
except ImportError:
    websockets = None


class MCPClient:
    """
    CAGE MCP WebSocket Client

    Provides async interface to CAGE via Model Context Protocol.

    Example:
        >>> async def main():
        ...     async with MCPClient(user_id="demo") as client:
        ...         result = await client.execute_code("print('Hello')")
        ...         print(result)
        >>> asyncio.run(main())
    """

    def __init__(
        self,
        api_url: str = "ws://127.0.0.1:8080/mcp",
        user_id: str = "default",
    ):
        """
        Initialize MCP client

        Args:
            api_url: WebSocket URL of CAGE MCP endpoint
            user_id: User identifier for authentication
        """
        if websockets is None:
            raise ImportError("websockets library required. Install with: pip install websockets")

        self.api_url = api_url
        self.user_id = user_id
        self.ws = None
        self._msg_id = 0

    async def __aenter__(self):
        """Async context manager entry"""
        await self.connect()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        await self.close()

    async def connect(self):
        """Connect to CAGE MCP WebSocket"""
        self.ws = await websockets.connect(self.api_url)

        # Send initialize
        init_response = await self._send_request("initialize", {
            "user_id": self.user_id,
            "clientInfo": {
                "name": "CAGE Python SDK",
                "version": "1.0.0"
            }
        })

        if "error" in init_response:
            raise Exception(f"Initialize failed: {init_response['error']}")

        return init_response["result"]

    async def list_tools(self) -> List[Dict[str, Any]]:
        """List available tools"""
        response = await self._send_request("tools/list")

        if "error" in response:
            raise Exception(f"List tools failed: {response['error']}")

        return response["result"]["tools"]

    async def execute_code(
        self,
        code: str,
        language: str = "python",
        persistent: bool = False,
        timeout_seconds: int = 30,
    ) -> Dict[str, Any]:
        """
        Execute code via MCP

        Args:
            code: Code to execute
            language: Programming language
            persistent: Use persistent interpreter (Python only)
            timeout_seconds: Maximum execution time

        Returns:
            Execution result
        """
        response = await self._call_tool("execute_code", {
            "code": code,
            "language": language,
            "persistent": persistent,
            "timeout_seconds": timeout_seconds,
        })

        if "error" in response:
            raise Exception(f"Execution failed: {response['error']}")

        return response["result"]

    async def list_files(self, path: str = "/") -> Dict[str, Any]:
        """List workspace files via MCP"""
        response = await self._call_tool("list_files", {"path": path})

        if "error" in response:
            raise Exception(f"List files failed: {response['error']}")

        # Parse the text content which contains JSON
        content_text = response["result"]["content"][0]["text"]
        return json.loads(content_text)

    async def upload_file(self, filename: str, content: bytes) -> Dict[str, Any]:
        """Upload file via MCP (base64 encoded)"""
        content_b64 = base64.b64encode(content).decode('utf-8')

        response = await self._call_tool("upload_file", {
            "filename": filename,
            "content": content_b64,
        })

        if "error" in response:
            raise Exception(f"Upload failed: {response['error']}")

        return response["result"]

    async def _send_request(self, method: str, params: Optional[Dict] = None) -> Dict:
        """Send JSON-RPC 2.0 request"""
        self._msg_id += 1

        request = {
            "jsonrpc": "2.0",
            "id": self._msg_id,
            "method": method,
        }

        if params is not None:
            request["params"] = params

        await self.ws.send(json.dumps(request))
        response_text = await self.ws.recv()
        return json.loads(response_text)

    async def _call_tool(self, tool_name: str, arguments: Dict) -> Dict:
        """Call an MCP tool"""
        return await self._send_request("tools/call", {
            "name": tool_name,
            "arguments": arguments,
        })

    async def close(self):
        """Close WebSocket connection"""
        if self.ws:
            await self.ws.close()
