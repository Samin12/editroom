"""A small Borumi MCP client: one stdio connection to `borumi mcp`, JSON in and out.

Borumi gates tools per connection (a tool refuses with `guide_required` until its guide was fetched
on the same connection) and only accepts ids it has shown on that connection, so a whole import runs
on one `Borumi()` connection:

    with Borumi() as bm:
        bm.guides("importing_media", "editing_segment_take_add")
        projects = bm.call("list_recent_projects")

`call` returns the tool's JSON result (structuredContent, else the text content parsed as JSON) and
raises `BorumiError` when the tool reports an error. A `guide_required` error fetches the named guide
once and retries.
"""
import json
import os
import queue
import re
import subprocess
import threading
import time

BIN = os.environ.get("BORUMI_BIN") or "/Applications/Borumi.app/Contents/MacOS/borumi"
GUIDE_RE = re.compile(r'guide_ids?"?\s*:\s*\[?\s*"([a-z0-9_]+)"', re.I)


class BorumiError(RuntimeError):
    def __init__(self, tool, message, payload=None):
        super().__init__(f"{tool}: {message}")
        self.tool = tool
        self.payload = payload


class Borumi:
    def __init__(self, bin_path=BIN, log=None, verbose=False):
        self.bin = bin_path
        self.log = log  # optional path: every request and reply is appended as one JSON line
        self.verbose = verbose
        self.proc = None
        self.q = queue.Queue()
        self.seq = 0
        self.fetched = set()

    # ---- connection ----
    def __enter__(self):
        self.open()
        return self

    def __exit__(self, *exc):
        self.close()

    def open(self):
        if not os.path.exists(self.bin):
            raise BorumiError("connect", f"Borumi is not installed at {self.bin}")
        self.proc = subprocess.Popen([self.bin, "mcp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=subprocess.DEVNULL, text=True, bufsize=1)
        threading.Thread(target=self._reader, daemon=True).start()
        self._rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                 "clientInfo": {"name": "editroom", "version": "0.1.0"}}, 60)
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        self.call("get_guides", {}, timeout=60)  # required first in every connection

    def close(self):
        if self.proc and self.proc.poll() is None:
            try:
                self.proc.stdin.close()
            except OSError:
                pass
            try:
                self.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.proc.terminate()
        self.proc = None

    def _reader(self):
        for line in self.proc.stdout:
            line = line.strip()
            if not line:
                continue
            try:
                self.q.put(json.loads(line))
            except ValueError:
                pass  # not JSON-RPC (stray log line)

    def _send(self, msg):
        self.proc.stdin.write(json.dumps(msg) + "\n")
        self.proc.stdin.flush()

    def _rpc(self, method, params, timeout):
        self.seq += 1
        i = self.seq
        self._send({"jsonrpc": "2.0", "id": i, "method": method, "params": params})
        t0 = time.time()
        while time.time() - t0 < timeout:
            if self.proc.poll() is not None:
                raise BorumiError(method, "the borumi mcp process exited")
            try:
                m = self.q.get(timeout=0.5)
            except queue.Empty:
                continue
            if m.get("id") == i:
                return m
        raise BorumiError(method, f"no reply in {timeout}s")

    # ---- tools ----
    def guides(self, *ids):
        """Fetch guides one id per call (Borumi's rule), skipping ones already fetched."""
        out = {}
        for g in ids:
            if g in self.fetched:
                continue
            out[g] = self.call("get_guides", {"guide_ids": [g]}, timeout=60)
            self.fetched.add(g)
        return out

    def call(self, tool, args=None, timeout=180, _retried=False):
        args = args or {}
        m = self._rpc("tools/call", {"name": tool, "arguments": args}, timeout)
        if self.log:
            with open(self.log, "a") as f:
                f.write(json.dumps({"t": time.time(), "tool": tool, "args": args, "reply": m})[:200000] + "\n")
        if "error" in m:
            raise BorumiError(tool, m["error"].get("message", str(m["error"])), m["error"])
        res = m.get("result") or {}
        text = "\n".join(c.get("text", "") for c in res.get("content", []) if c.get("type") == "text")
        data = res.get("structuredContent")
        if data is None:
            try:
                data = json.loads(text) if text.strip() else {}
            except ValueError:
                data = {"text": text}
        if res.get("isError"):
            gm = GUIDE_RE.search(text)
            if "guide_required" in text and gm and not _retried:
                self.fetched.discard(gm.group(1))
                self.guides(gm.group(1))
                return self.call(tool, args, timeout, _retried=True)
            raise BorumiError(tool, text.strip()[:2000] or "error", data)
        if self.verbose:
            print(f"  borumi {tool}: ok")
        return data


if __name__ == "__main__":
    # debugging: python3 bmcp.py <tool> '<json args>' [guide ...]
    import sys
    if len(sys.argv) < 2:
        raise SystemExit("usage: bmcp.py <tool> '<json args>' [guide_id ...]")
    with Borumi() as bm:
        bm.guides(*sys.argv[3:])
        print(json.dumps(bm.call(sys.argv[1], json.loads(sys.argv[2]) if len(sys.argv) > 2 else {}), indent=1))
