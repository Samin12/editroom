"""Offline tests for decide.py: request shapes, answer normalization, cache labels (no network, no keys).

    python3 tools/jev/test_decide.py
"""
import json
import os
import struct
import tempfile
import unittest
import zlib

import decide


def png(w, h):
    raw = b"".join(b"\x00" + b"\x80" * (w * 3) for _ in range(h))
    def chunk(t, d):
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + \
        chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


class T(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        decide.CACHE = os.path.join(self.tmp, "cache")
        self.img = os.path.join(self.tmp, "a.png")
        open(self.img, "wb").write(png(400, 300))
        os.environ["TYPESAFE_API_KEY"] = "test-key-not-real"
        os.environ["SAGE_API_KEY"] = "test-key-not-real"

    def test_header_size(self):
        self.assertEqual(decide.header_size(open(self.img, "rb").read()), (400, 300))

    def test_jev_request_shares_one_image(self):
        job = {"image": self.img, "questions": [
            {"id": "a", "kind": "yesno", "instructions": "Is there text in the image?", "criteria": {"true": "t", "false": "f"}},
            {"id": "b", "kind": "box", "instructions": "the text"},
            {"id": "c", "kind": "choice", "instructions": "Tone?", "options": [{"option": "calm"}, {"option": "angry", "description": "loud"}]},
            {"id": "d", "kind": "score", "instructions": "How good?", "levels": ["bad", "ok", "good"]}]}
        body, headers = decide.jev_request(job)
        self.assertEqual(len(body["assets"]), 1)
        ref = next(iter(body["assets"]))
        self.assertEqual(body["state"]["image"], ref)
        self.assertEqual(headers["TypeSafe-Danger-Zone-Images"], "true")
        q = body["questions"]
        self.assertEqual(q["a"]["type"], "noul")
        self.assertIn("`image`", q["a"]["instructions"])
        self.assertEqual(q["b"]["type"], "box")
        self.assertEqual(q["c"]["criteria"], {"calm": None, "angry": "loud"})
        self.assertEqual(q["d"]["criteria"], ["bad", "ok", "good"])

    def test_jev_answers_normalized(self):
        def fake(url, body, headers, **kw):
            return {"model": "jev-1.14.0", "usage": {"input_tokens": 400},
                    "answers": {"a": {"type": "noul", "noul": 0.55}, "b": {"type": "box", "confidence": 0.8,
                                "box": {"left": 0.1, "right": 0.3, "top": 0.2, "bottom": 0.5}}}}, 12.0
        decide.post = fake
        rows = decide.ask({"key": "k", "image": self.img, "questions": [
            {"id": "a", "kind": "yesno", "instructions": "x"}, {"id": "b", "kind": "box", "instructions": "y"}]}, "jev")
        self.assertIsNone(rows[0]["answer"])  # 0.55 is in the unsure band: the agent decides
        self.assertEqual([round(v, 3) for v in rows[1]["box"]], [0.1, 0.2, 0.2, 0.3])

    def test_sage_batch_and_cache_key_label(self):
        seen = []

        def fake(url, body, headers, **kw):
            seen.append(body)
            return {"results": [{"answers": [{"ok": True, "result": {"id": "a", "kind": "yesno",
                    "result": {"answer": "yes", "probability": 0.9}}}]}],
                    "meta": {"model": "sage", "usage": {"billed_input_tokens": 1441}}}, 300.0
        decide.post = fake
        q = [{"id": "a", "kind": "yesno", "instructions": "x"}, {"id": "b", "kind": "box", "instructions": "y"}]
        r1 = decide.ask({"key": "first", "image": self.img, "questions": q}, "sage")
        self.assertTrue(seen[0]["requests"][0]["content"]["media"].startswith("data:image/"))
        self.assertEqual(r1[0]["answer"], "yes")
        self.assertIn("cannot answer box", r1[1]["error"])
        self.assertEqual(r1[0]["input_tokens"], 1441)
        r2 = decide.ask({"key": "second", "image": self.img, "questions": q}, "sage")  # same content: cached
        self.assertTrue(r2[0]["cached"])
        self.assertEqual(r2[0]["key"], "second")
        self.assertEqual(len(seen), 1)


if __name__ == "__main__":
    unittest.main()
