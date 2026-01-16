"""CAGE REST API Client"""

import base64
from typing import Dict, List, Optional, Any
import requests


class CAGEError(Exception):
    """Base exception for CAGE SDK"""
    pass


class AuthenticationError(CAGEError):
    """Authentication failed"""
    pass


class ExecutionError(CAGEError):
    """Code execution failed"""
    pass


class CAGEClient:
    """
    CAGE REST API Client

    Provides methods for executing code, managing files, and sessions.

    Example:
        >>> client = CAGEClient(api_url="http://localhost:8080", api_key="dev_myuser")
        >>> result = client.execute("print('Hello CAGE!')")
        >>> print(result['stdout'])
        Hello CAGE!
    """

    def __init__(
        self,
        api_url: str = "http://127.0.0.1:8080",
        api_key: str = "dev_user",
        timeout: int = 60,
    ):
        """
        Initialize CAGE client

        Args:
            api_url: Base URL of CAGE orchestrator
            api_key: API key for authentication
            timeout: Default request timeout in seconds
        """
        self.api_url = api_url.rstrip('/')
        self.api_key = api_key
        self.timeout = timeout
        self._session = requests.Session()
        self._session.headers.update({
            "Authorization": f"ApiKey {api_key}",
            "Content-Type": "application/json",
        })

    def execute(
        self,
        code: str,
        language: str = "python",
        timeout_seconds: Optional[int] = None,
        persistent: bool = False,
        env: Optional[Dict[str, str]] = None,
    ) -> Dict[str, Any]:
        """
        Execute code in sandbox

        Args:
            code: Code to execute
            language: Programming language (python, javascript, bash, r, julia, typescript, ruby, go)
            timeout_seconds: Maximum execution time (default: 30)
            persistent: Use persistent interpreter mode (Python only)
            env: Additional environment variables

        Returns:
            Execution result with stdout, stderr, exit_code, etc.

        Raises:
            ExecutionError: If execution fails
            AuthenticationError: If authentication fails
        """
        payload = {
            "code": code,
            "language": language,
            "persistent": persistent,
        }

        if timeout_seconds is not None:
            payload["timeout_seconds"] = timeout_seconds

        if env:
            payload["env"] = env

        try:
            response = self._session.post(
                f"{self.api_url}/api/v1/execute",
                json=payload,
                timeout=self.timeout,
            )
        except requests.RequestException as e:
            raise CAGEError(f"Request failed: {e}")

        if response.status_code == 401:
            raise AuthenticationError("Invalid API key")
        elif response.status_code == 429:
            raise ExecutionError("Rate limit exceeded")
        elif not response.ok:
            error_data = response.json() if response.headers.get("content-type") == "application/json" else {}
            raise ExecutionError(f"Execution failed: {error_data.get('message', response.text)}")

        return response.json()

    def execute_async(
        self,
        code: str,
        language: str = "python",
        timeout_seconds: Optional[int] = None,
    ) -> str:
        """
        Execute code asynchronously

        Returns:
            job_id for polling status
        """
        payload = {
            "code": code,
            "language": language,
        }

        if timeout_seconds is not None:
            payload["timeout_seconds"] = timeout_seconds

        response = self._session.post(
            f"{self.api_url}/api/v1/execute/async",
            json=payload,
            timeout=self.timeout,
        )

        if not response.ok:
            raise ExecutionError(f"Async execution failed: {response.text}")

        return response.json()["job_id"]

    def get_job_status(self, job_id: str) -> Dict[str, Any]:
        """Get async job status and result"""
        response = self._session.get(
            f"{self.api_url}/api/v1/jobs/{job_id}",
            timeout=self.timeout,
        )

        if response.status_code == 404:
            raise CAGEError(f"Job {job_id} not found")
        elif not response.ok:
            raise CAGEError(f"Failed to get job status: {response.text}")

        return response.json()

    def upload_file(
        self,
        file_path: str,
        target_path: str = "/",
    ) -> Dict[str, Any]:
        """
        Upload a file to workspace

        Args:
            file_path: Local file path
            target_path: Target path in workspace

        Returns:
            Upload result with path, size, checksum
        """
        with open(file_path, 'rb') as f:
            files = {'file': f}
            data = {'path': target_path}

            # Remove Content-Type header for multipart
            headers = dict(self._session.headers)
            headers.pop('Content-Type', None)

            response = requests.post(
                f"{self.api_url}/api/v1/files",
                files=files,
                data=data,
                headers=headers,
                timeout=self.timeout,
            )

        if not response.ok:
            raise CAGEError(f"Upload failed: {response.text}")

        return response.json()

    def download_file(self, file_path: str, output_path: Optional[str] = None) -> bytes:
        """
        Download a file from workspace

        Args:
            file_path: File path in workspace
            output_path: Optional local path to save (if None, returns content)

        Returns:
            File content as bytes
        """
        response = self._session.get(
            f"{self.api_url}/api/v1/files/{file_path}",
            timeout=self.timeout,
        )

        if response.status_code == 404:
            raise CAGEError(f"File {file_path} not found")
        elif not response.ok:
            raise CAGEError(f"Download failed: {response.text}")

        content = response.content

        if output_path:
            with open(output_path, 'wb') as f:
                f.write(content)

        return content

    def list_files(self, path: str = "/", recursive: bool = False) -> List[Dict[str, Any]]:
        """
        List files in workspace

        Args:
            path: Directory path to list
            recursive: List recursively

        Returns:
            List of file info dicts
        """
        params = {"path": path}
        if recursive:
            params["recursive"] = "true"

        response = self._session.get(
            f"{self.api_url}/api/v1/files",
            params=params,
            timeout=self.timeout,
        )

        if not response.ok:
            raise CAGEError(f"List files failed: {response.text}")

        return response.json()["files"]

    def delete_file(self, file_path: str):
        """Delete a file from workspace"""
        response = self._session.delete(
            f"{self.api_url}/api/v1/files/{file_path}",
            timeout=self.timeout,
        )

        if response.status_code == 404:
            raise CAGEError(f"File {file_path} not found")
        elif not response.ok:
            raise CAGEError(f"Delete failed: {response.text}")

    def get_session(self) -> Dict[str, Any]:
        """Get current session information"""
        response = self._session.get(
            f"{self.api_url}/api/v1/session",
            timeout=self.timeout,
        )

        if response.status_code == 404:
            raise CAGEError("No active session")
        elif not response.ok:
            raise CAGEError(f"Failed to get session: {response.text}")

        return response.json()

    def create_session(self, language: str = "python", reset_workspace: bool = False) -> Dict[str, Any]:
        """Create or restart session"""
        payload = {
            "language": language,
            "reset_workspace": reset_workspace,
        }

        response = self._session.post(
            f"{self.api_url}/api/v1/session",
            json=payload,
            timeout=self.timeout,
        )

        if not response.ok:
            raise CAGEError(f"Failed to create session: {response.text}")

        return response.json()

    def terminate_session(self, purge_data: bool = False):
        """Terminate current session"""
        params = {"purge_data": str(purge_data).lower()}

        response = self._session.delete(
            f"{self.api_url}/api/v1/session",
            params=params,
            timeout=self.timeout,
        )

        if not response.ok:
            raise CAGEError(f"Failed to terminate session: {response.text}")

    def health(self) -> Dict[str, Any]:
        """Get server health status"""
        response = requests.get(
            f"{self.api_url}/health",
            timeout=self.timeout,
        )

        if not response.ok:
            raise CAGEError(f"Health check failed: {response.text}")

        return response.json()

    def __enter__(self):
        """Context manager support"""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Cleanup on context manager exit"""
        self._session.close()
