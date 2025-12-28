"""CLI wrapper for executing iMi commands via subprocess."""

import asyncio
import json
import shutil
from typing import List

from .config import MCPConfig
from .schemas import CLIResult


async def run_imi_command(args: List[str], config: MCPConfig) -> CLIResult:
    """Execute an iMi CLI command and parse the result.

    Args:
        args: Command arguments (e.g., ["add", "feat", "my-feature"])
        config: MCP configuration with binary path and timeout

    Returns:
        CLIResult with parsed output or error information

    Raises:
        FileNotFoundError: If iMi binary not found
        asyncio.TimeoutError: If command exceeds timeout
    """
    # Locate iMi binary
    imi_binary = shutil.which(config.imi_binary_path)
    if not imi_binary:
        return CLIResult(
            success=False,
            error=f"iMi binary not found: {config.imi_binary_path}. "
            f"Ensure iMi is installed and in PATH.",
            exit_code=127,
        )

    # Always request JSON output
    cmd_args = [imi_binary, "--json"] + args

    try:
        # Spawn subprocess
        process = await asyncio.create_subprocess_exec(
            *cmd_args,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        # Wait with timeout
        stdout, stderr = await asyncio.wait_for(
            process.communicate(), timeout=config.timeout_seconds
        )

        stdout_str = stdout.decode("utf-8").strip()
        stderr_str = stderr.decode("utf-8").strip()

        # Parse JSON output
        if process.returncode == 0:
            try:
                # iMi outputs JSON on the last line after any colored output
                lines = stdout_str.split("\n")
                json_line = None

                # Find the JSON response (starts with "{" and contains "success")
                for line in reversed(lines):
                    if line.strip().startswith("{"):
                        try:
                            parsed = json.loads(line.strip())
                            if "success" in parsed:
                                json_line = line.strip()
                                break
                        except json.JSONDecodeError:
                            continue

                if json_line:
                    data = json.loads(json_line)
                    return CLIResult(
                        success=data.get("success", True),
                        data=data.get("data"),
                        error=data.get("error"),
                        stdout=stdout_str,
                        exit_code=process.returncode,
                    )
                else:
                    # No JSON found, return raw output
                    return CLIResult(
                        success=True,
                        stdout=stdout_str,
                        stderr=stderr_str,
                        exit_code=process.returncode,
                    )

            except json.JSONDecodeError as e:
                return CLIResult(
                    success=False,
                    error=f"Failed to parse JSON output: {e}",
                    stdout=stdout_str,
                    stderr=stderr_str,
                    exit_code=process.returncode,
                )
        else:
            # Command failed
            error_msg = stderr_str if stderr_str else stdout_str
            return CLIResult(
                success=False,
                error=error_msg or f"Command failed with exit code {process.returncode}",
                stdout=stdout_str,
                stderr=stderr_str,
                exit_code=process.returncode,
            )

    except asyncio.TimeoutError:
        return CLIResult(
            success=False,
            error=f"Command timed out after {config.timeout_seconds} seconds",
            exit_code=124,
        )

    except Exception as e:
        return CLIResult(
            success=False, error=f"Unexpected error: {str(e)}", exit_code=1
        )
