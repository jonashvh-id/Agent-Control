"""Standalone external MCP client: no GitHub credential, policy code or direct provider access."""

import argparse
import asyncio
import json
import os

import httpx2
from mcp import ClientSession
from mcp.client.streamable_http import streamable_http_client


async def run(url, tool, arguments):
    token = os.environ.get("AGENT_CONTROL_AGENT_TOKEN", "")
    if not token:
        raise ValueError("Provide the Agent Control credential through the process environment")
    async with httpx2.AsyncClient(headers={"Authorization": "Bearer " + token}, trust_env=False) as client:
        async with streamable_http_client(url, http_client=client) as streams:
            async with ClientSession(*streams) as session:
                await session.initialize()
                if tool == "discover":
                    return {"tools": [tool.name for tool in (await session.list_tools()).tools]}
                result = await session.call_tool(tool, arguments)
                return {"is_error": result.is_error, "result": result.structured_content}


def main():
    parser = argparse.ArgumentParser(description="External Agent Control MCP client")
    parser.add_argument("--url", default="http://127.0.0.1:8010/mcp/")
    parser.add_argument("--tool", required=True)
    parser.add_argument("--request-file")
    args = parser.parse_args()
    arguments = json.loads(open(args.request_file).read()) if args.request_file else {}
    print(json.dumps(asyncio.run(run(args.url, args.tool, arguments)), ensure_ascii=False))


if __name__ == "__main__":
    main()
