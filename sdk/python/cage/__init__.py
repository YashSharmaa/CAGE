"""
CAGE Python SDK - Secure sandboxed code execution for LLM agents

This SDK provides a Python interface to the CAGE orchestrator API.
"""

from .client import CAGEClient, CAGEError, ExecutionError, AuthenticationError
from .mcp import MCPClient

__version__ = "1.0.0"
__all__ = ["CAGEClient", "MCPClient", "CAGEError", "ExecutionError", "AuthenticationError"]
