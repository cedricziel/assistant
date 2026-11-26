#!/usr/bin/env python3
"""Smoke-test the MCP server of an instance from start-instance.sh.

Usage: mcp-smoke.py <URL> <HOME> [TOOL [JSON_ARGS]]

1. stdio: runs `assistant mcp` with the instance's HOME, lists tools.
2. HTTP: registers a client (RFC 7591), signs in as the seeded admin with
   PKCE and `resource=<URL>/mcp`, then lists tools on /mcp and calls TOOL
   (default `mcp__demo__get-forecast` with {"city": "Berlin"}).

Prints one PASS/FAIL line per check and exits non-zero on any failure.
Standard library only.
"""

import base64
import hashlib
import json
import os
import pathlib
import secrets
import subprocess
import sys
import urllib.error
import urllib.parse
import urllib.request

PROTOCOL = "2025-11-25"
failed = False


def report(name, ok, detail=""):
    global failed
    failed |= not ok
    print(f"{'PASS' if ok else 'FAIL'} {name}{': ' + detail if detail else ''}")


def stdio_tools(binary, home):
    proc = subprocess.Popen(
        [binary, "mcp"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        env={"PATH": os.environ["PATH"], "HOME": home},
    )

    def rpc(msg):
        proc.stdin.write(json.dumps(msg) + "\n")
        proc.stdin.flush()
        if "id" in msg:
            return json.loads(proc.stdout.readline())

    try:
        rpc({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": PROTOCOL, "capabilities": {},
            "clientInfo": {"name": "verify", "version": "1"}}})
        rpc({"jsonrpc": "2.0", "method": "notifications/initialized"})
        listed = rpc({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
        return [t["name"] for t in listed["result"]["tools"]]
    finally:
        proc.stdin.close()
        proc.wait(10)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args):
        return None


def post(url, body, headers=None, form=False):
    data = urllib.parse.urlencode(body).encode() if form else json.dumps(body).encode()
    req = urllib.request.Request(url, data=data, method="POST", headers={
        "Content-Type": "application/x-www-form-urlencoded" if form else "application/json",
        **(headers or {})})
    try:
        with urllib.request.build_opener(NoRedirect).open(req, timeout=60) as resp:
            return resp.status, dict(resp.headers), resp.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read().decode()


def mcp_token(url, home):
    creds = next(pathlib.Path(home, ".assistant", "orgs").rglob("admin_credentials.txt"))
    fields = dict(line.split(": ", 1) for line in creds.read_text().splitlines() if ": " in line)
    redirect = "http://127.0.0.1:9/callback"
    status, _, body = post(f"{url}/oauth/register", {
        "client_name": "verify", "redirect_uris": [redirect],
        "grant_types": ["authorization_code"], "response_types": ["code"],
        "token_endpoint_auth_method": "none"})
    if status != 201:
        raise RuntimeError(f"register {status}: {body}")
    client_id = json.loads(body)["client_id"]
    verifier = secrets.token_urlsafe(48)
    challenge = base64.urlsafe_b64encode(hashlib.sha256(verifier.encode()).digest()).rstrip(b"=").decode()
    resource = f"{url}/mcp"
    status, headers, body = post(f"{url}/oauth/authorize", {
        "org": "default", "email": fields.get("email", "admin@localhost"),
        "password": fields["password"], "client_id": client_id, "redirect_uri": redirect,
        "response_type": "code", "state": "verify", "code_challenge": challenge,
        "code_challenge_method": "S256", "scope": "mcp:execute", "resource": resource,
    }, headers={"Origin": url}, form=True)
    location = headers.get("location") or headers.get("Location") or ""
    code = urllib.parse.parse_qs(urllib.parse.urlparse(location).query).get("code", [None])[0]
    if status != 303 or not code:
        raise RuntimeError(f"authorize {status}: {location or body[:300]}")
    status, _, body = post(f"{url}/oauth/token", {
        "grant_type": "authorization_code", "code": code, "redirect_uri": redirect,
        "client_id": client_id, "code_verifier": verifier, "resource": resource}, form=True)
    if status != 200:
        raise RuntimeError(f"token {status}: {body}")
    return json.loads(body)["access_token"]


def mcp_http(url, token):
    headers = {"Authorization": f"Bearer {token}",
               "Accept": "application/json, text/event-stream",
               "MCP-Protocol-Version": PROTOCOL}

    def rpc(msg):
        status, resp_headers, body = post(f"{url}/mcp", msg, headers)
        session = resp_headers.get("mcp-session-id") or resp_headers.get("Mcp-Session-Id")
        if session:
            headers["Mcp-Session-Id"] = session
        if "id" not in msg:
            return status
        if status != 200:
            raise RuntimeError(f"{msg['method']} {status}: {body[:300]}")
        events = [line[5:].strip() for line in body.splitlines() if line.startswith("data:")]
        return json.loads(events[-1] if events else body)

    rpc({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": PROTOCOL, "capabilities": {},
        "clientInfo": {"name": "verify", "version": "1"}}})
    rpc({"jsonrpc": "2.0", "method": "notifications/initialized"})
    return rpc, rpc({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})["result"]["tools"]


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    url, home = sys.argv[1].rstrip("/"), sys.argv[2]
    # `assistant mcp` with an empty or wrong HOME would use ~/.assistant.
    if not url or not pathlib.Path(home).name.startswith("assistant-verify."):
        sys.exit(f"refusing: {home!r} is not a start-instance.sh HOME")
    tool = sys.argv[3] if len(sys.argv) > 3 else "mcp__demo__get-forecast"
    args = json.loads(sys.argv[4]) if len(sys.argv) > 4 else {"city": "Berlin"}
    binary = str(pathlib.Path(__file__).resolve().parents[4] / "target" / "debug" / "assistant")

    try:
        names = stdio_tools(binary, home)
        report("stdio tools/list", "run-prompt" in names, f"{len(names)} tools")
    except Exception as e:  # noqa: BLE001 - report and continue
        report("stdio tools/list", False, str(e))

    try:
        token = mcp_token(url, home)
        report("DCR + PKCE MCP token", True)
        rpc, tools = mcp_http(url, token)
        names = [t["name"] for t in tools]
        report("HTTP tools/list", tool in names, f"{len(names)} tools")
        result = rpc({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                      "params": {"name": tool, "arguments": args}})["result"]
        text = " ".join(c.get("text", "") for c in result.get("content", []))
        report(f"HTTP tools/call {tool}", not result.get("isError"), text[:120])
    except Exception as e:  # noqa: BLE001 - report and continue
        report("HTTP /mcp", False, repr(e))

    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
