"""Offline tests for the import's safety gate and file picking (no Borumi needed).

    python3 -I -m unittest discover -s studio/borumi/tests
"""
import importlib.util
import json
import os
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location("imp", os.path.join(HERE, "..", "import.py"))
imp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(imp)


class Safety(unittest.TestCase):
    def setUp(self):
        self.reg = {"projects": [{"name": "tips raw - editroom", "path": "/x/tips raw - editroom.bmprojbundle"}]}

    def test_writes_only_to_registered_editroom_projects(self):
        imp.assert_writable(self.reg, {"name": "tips raw - editroom", "path": "/x/tips raw - editroom.bmprojbundle"})
        with self.assertRaises(SystemExit):  # right suffix, but editroom did not create it
            imp.assert_writable(self.reg, {"name": "honcho - editroom", "path": "/x/honcho - editroom.bmprojbundle"})
        with self.assertRaises(SystemExit):  # created by editroom, renamed by hand
            imp.assert_writable({"projects": [{"name": "tips raw", "path": "/x/tips raw.bmprojbundle"}]},
                                {"name": "tips raw", "path": "/x/tips raw.bmprojbundle"})
        with self.assertRaises(SystemExit):  # a source project
            imp.assert_writable(self.reg, {"name": "tips raw", "path": "/x/tips raw.bmprojbundle"})

    def test_default_name(self):
        self.assertEqual(imp.default_name("/a/Borumi Projects/tips raw editroom"), "tips raw - editroom")
        self.assertEqual(imp.default_name("/a/Borumi Projects/tips raw editroom/"), "tips raw - editroom")

    def test_newest_render_takes_the_highest_version(self):
        d = tempfile.mkdtemp()
        os.makedirs(os.path.join(d, "out"))
        for n in ["x - editroom v2.mp4", "x - editroom v10.mp4", "x - editroom v3.mp4"]:
            open(os.path.join(d, "out", n), "w").close()
            time.sleep(0.01)
        self.assertTrue(imp.newest_render(d).endswith("v10.mp4"))

    def test_section_films_prefer_done_full_renders(self):
        d = tempfile.mkdtemp()
        for slot, state, draft in [("T1", "done", False), ("T2", "rendering", False), ("T3", "done", True)]:
            j = os.path.join(d, "studio", "jobs", slot)
            os.makedirs(j)
            p = os.path.join(j, f"{slot}.mp4")
            open(p, "w").close()
            json.dump({"state": state, "path": p, "draft": draft}, open(os.path.join(j, "render-status.json"), "w"))
        self.assertEqual([os.path.basename(f) for f in imp.section_films(d)], ["T1.mp4"])

    def test_fingerprint_changes_with_content(self):
        d = tempfile.mkdtemp()
        a, b = os.path.join(d, "a"), os.path.join(d, "b")
        open(a, "wb").write(b"x" * 1000)
        open(b, "wb").write(b"y" * 1000)
        self.assertNotEqual(imp.fingerprint(a), imp.fingerprint(b))
        self.assertEqual(imp.fingerprint(a), imp.fingerprint(a))


if __name__ == "__main__":
    unittest.main()
