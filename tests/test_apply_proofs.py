"""Tests for scripts/apply_proofs.py. Run: python3 -I -m unittest discover -s tests -v"""
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
SCRIPT = os.path.join(os.path.dirname(HERE), "scripts", "apply_proofs.py")
spec = importlib.util.spec_from_file_location("apply_proofs", SCRIPT)
ap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ap)

# The exact line `cutroom agent` wrote on the first real run ("tips raw", beats/ch04.yaml): a
# compact JSON object with sorted keys. apply_proofs 0.1 matched `src:` only and updated 0 of 6.
GROK_LINE = ('  - {"at":"w910","caption":"Grok Bot: an engineering intern for agents","kind":"image","proof":{"claim":'
             '"Ling Ji Li, a SpaceX engineer, describes Grockbot as an engineering intern that can manage other coding '
             'agents","find":"X/Twitter post by Ling Ji Li about Grockbot"},"source":"x.com","src":"proof/grockbot-spacex.png","until":"w930"}')
GROK_CAPTURE = {
    "src": "proof/grockbot-spacex.png", "verdict": "supported", "url": "https://x.com/lingxi/article/2094493172516966781",
    "highlight": [0.4822, 0.8045, 0.0919, 0.0275], "source_pill": "x.com/lingxi · captured Oct 31, 2026",
    "caption": "Lingxi Li: Grok Bot, an engineering intern for agents",
    "flag": "Names are misspelled in the transcript: Lingxi Li, Grok Bot.",
}


class JsonLines(unittest.TestCase):
    def test_grok_line_overwrites_source_and_applies_caption(self):
        txt = "scope: [w800, w1000]\nbeats:\n" + GROK_LINE + "\n"
        new, changed = ap.patch_text(txt, "proof/grockbot-spacex.png", ap.fields_for(GROK_CAPTURE))
        self.assertEqual(sorted(changed), ["caption", "highlight", "source"])
        line = new.split("\n")[2]
        self.assertTrue(line.startswith('  - {"at":"w910",'))  # same compact JSON style, sorted keys
        obj = json.loads(line.split("- ", 1)[1])
        self.assertEqual(obj["source"], "x.com/lingxi · captured Oct 31, 2026")  # overwrote "x.com"
        self.assertEqual(obj["highlight"], [0.4822, 0.8045, 0.0919, 0.0275])
        self.assertEqual(obj["caption"], "Lingxi Li: Grok Bot, an engineering intern for agents")
        self.assertEqual(obj["proof"]["find"], "X/Twitter post by Ling Ji Li about Grockbot")  # untouched
        self.assertEqual(list(obj), sorted(obj))
        self.assertNotIn(" ", line.split("- ", 1)[1].split('"caption"')[0])  # no spaces added
        again, changed2 = ap.patch_text(new, "proof/grockbot-spacex.png", ap.fields_for(GROK_CAPTURE))
        self.assertEqual((again, changed2), (new, []))  # idempotent

    def test_other_beats_untouched(self):
        other = '  - {"at":"w420","kind":"image","source":"x.com","src":"proof/karpathy.png"}'
        txt = "beats:\n" + other + "\n" + GROK_LINE + "\n"
        new, _ = ap.patch_text(txt, "proof/grockbot-spacex.png", ap.fields_for(GROK_CAPTURE))
        self.assertEqual(new.split("\n")[1], other)


class YamlStyles(unittest.TestCase):
    F = {"highlight": [0.1, 0.2, 0.3, 0.04], "source": "a.com · captured Oct 31, 2026", "caption": "Fixed Name"}

    def test_flow_overwrites_and_adds(self):
        line = '  - {at: w40, kind: image, src: proof/rates.png, source: a.com, proof: {claim: "x, y", find: "z"}}'
        new, changed = ap.patch_text("beats:\n" + line, "proof/rates.png", self.F)
        out = new.split("\n")[1]
        self.assertIn('source: "a.com · captured Oct 31, 2026"', out)
        self.assertIn("highlight: [0.1, 0.2, 0.3, 0.04]", out)
        self.assertIn('caption: "Fixed Name"', out)
        self.assertIn('proof: {claim: "x, y", find: "z"}', out)  # nested map with a comma survives
        self.assertTrue(out.endswith("}"))
        self.assertEqual(sorted(changed), ["caption", "highlight", "source"])

    def test_quoted_src_flow(self):
        line = '  - {at: w40, kind: image, src: "proof/rates.png"}'
        new, changed = ap.patch_text(line, "proof/rates.png", {"source": "b.com"})
        self.assertEqual(new, '  - {at: w40, kind: image, src: "proof/rates.png", source: "b.com"}')

    def test_block_overwrites_and_inserts(self):
        txt = ("beats:\n  - at: w80\n    kind: image\n    src: \"proof/block.png\"\n    source: old.com\n"
               "    caption: Pricing\n  - {at: w99, kind: title, text: Next}\n")
        new, changed = ap.patch_text(txt, "proof/block.png", self.F)
        lines = new.split("\n")
        self.assertIn('    source: "a.com · captured Oct 31, 2026"', lines)
        self.assertIn('    caption: "Fixed Name"', lines)
        self.assertIn("    highlight: [0.1, 0.2, 0.3, 0.04]", lines)
        self.assertEqual(lines[-2], "  - {at: w99, kind: title, text: Next}")
        self.assertEqual(sorted(changed), ["caption", "highlight", "source"])

    def test_block_written_by_drop_missing(self):
        # `cutroom proofs --drop-missing` re-serializes the whole file with serde_yaml: block style,
        # lists as "- x" lines at the key's own indent. A second apply must not leave them dangling.
        txt = ("beats:\n- at: w100\n  caption: Boris Cherney on agents\n  highlight:\n  - 0.1\n  - 0.2\n  - 0.3\n  - 0.04\n"
               "  kind: image\n  proof:\n    claim: x\n  source: x.com\n  src: proof/boris.png\n  until: w130\n"
               "- at: w300\n  kind: callout\n  text: Next\n")
        f = {"highlight": [0.4605, 0.3675, 0.3204, 0.0602], "source": "x.com/bcherny · captured Oct 31, 2026",
             "caption": "Boris Cherny on agents"}
        new, changed = ap.patch_text(txt, "proof/boris.png", f)
        self.assertEqual(sorted(changed), ["caption", "highlight", "source"])
        lines = new.split("\n")
        self.assertIn("  highlight: [0.4605, 0.3675, 0.3204, 0.0602]", lines)
        self.assertNotIn("  - 0.1", lines)
        self.assertIn("  kind: image", lines)
        self.assertIn("    claim: x", lines)
        self.assertEqual(lines[-4:], ["- at: w300", "  kind: callout", "  text: Next", ""])
        again, changed2 = ap.patch_text(new, "proof/boris.png", f)
        self.assertEqual((again, changed2), (new, []))
        same, unchanged = ap.patch_text(txt.replace("0.1\n  - 0.2\n  - 0.3\n  - 0.04", "0.4605\n  - 0.3675\n  - 0.3204\n  - 0.0602"),
                                        "proof/boris.png", {"highlight": f["highlight"]})
        self.assertEqual(unchanged, [])  # an equal multi-line list is left alone

    def test_absent_src(self):
        self.assertEqual(ap.patch_text("beats: []\n", "proof/none.png", self.F), ("beats: []\n", None))


def write(path, data, mode="w"):
    with open(path, mode) as f:
        f.write(data)


class EndToEnd(unittest.TestCase):
    def test_work_dir(self):
        with tempfile.TemporaryDirectory() as w:
            os.makedirs(os.path.join(w, "beats"))
            os.makedirs(os.path.join(w, "proof"))
            write(os.path.join(w, "beats", "ch04.yaml"), "beats:\n" + GROK_LINE + "\n")
            write(os.path.join(w, "edit.yaml"), "beats: []\n")
            write(os.path.join(w, "proof", "grockbot-spacex.png"), b"png", "wb")
            write(os.path.join(w, "proof", "grockbot-spacex.source.json"), json.dumps(GROK_CAPTURE))
            write(os.path.join(w, "proof", "gone.source.json"), json.dumps({"src": "proof/gone.png", "verdict": "unsupported"}))
            out = subprocess.run([sys.executable, "-I", SCRIPT, w], capture_output=True, text=True).stdout
            self.assertIn("1 beats updated, 1 captures not usable", out)
            self.assertIn("caption corrected", out)
            out2 = subprocess.run([sys.executable, "-I", SCRIPT, w], capture_output=True, text=True).stdout
            self.assertIn("0 beats updated", out2)


if __name__ == "__main__":
    unittest.main()
