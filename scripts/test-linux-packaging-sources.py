#!/usr/bin/env python3
"""Check source preparation boundaries before building or creating packages."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("packaging", Path(__file__).with_name("package-linux.py"))
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


class StopBeforeStaging(Exception):
    pass


class SourcePreparationTests(unittest.TestCase):
    def test_complete_fetch_precedes_build_even_when_reusing_binary(self):
        for offline in (False, True):
            for skip_build in (False, True):
                with self.subTest(offline=offline, skip_build=skip_build), tempfile.TemporaryDirectory() as folder:
                    root = Path(folder)
                    binary = root / "target/release/yyplayer"
                    binary.parent.mkdir(parents=True)
                    binary.write_bytes(b"fixture")
                    (root / "Cargo.toml").write_text('[workspace.package]\nversion="0.0.1"\n')
                    argv = ["package-linux.py"] + (["--offline"] if offline else []) + (["--skip-build"] if skip_build else [])
                    with patch.object(packaging, "ROOT", root), patch.object(packaging, "DIST", root / "dist"), patch.object(packaging, "TARGET", root / "staging"), patch.object(sys, "argv", argv), patch.object(packaging, "output", side_effect=["amd64", "0123456789ab"]), patch.object(packaging, "run") as run, patch.object(packaging, "system_mpv", side_effect=StopBeforeStaging):
                        with self.assertRaises(StopBeforeStaging):
                            packaging.main()
                        commands = [call.args[0] for call in run.call_args_list]
                        self.assertEqual(commands[0], ["cargo", "fetch", "--locked"] + (["--offline"] if offline else []))
                        self.assertNotIn("--target", commands[0])
                        self.assertEqual(len(commands), 1 if skip_build else 2)
                        if not skip_build:
                            self.assertEqual(commands[1][0:3], ["cargo", "build", "--locked"])
                        self.assertTrue(all(call.kwargs["cwd"] == root for call in run.call_args_list))

    def test_missing_source_cache_stops_before_build_and_output_creation(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            with patch.object(packaging, "ROOT", root), patch.object(packaging, "DIST", root / "dist"), patch.object(packaging, "TARGET", root / "staging"), patch.object(sys, "argv", ["package-linux.py", "--offline"]), patch.object(packaging, "output", side_effect=["amd64", "0123456789ab"]), patch.object(packaging, "run", side_effect=subprocess.CalledProcessError(101, ["cargo", "fetch"])) as run, patch.object(packaging, "system_mpv") as mpv:
                with self.assertRaises(subprocess.CalledProcessError):
                    packaging.main()
                self.assertEqual(run.call_count, 1)
                mpv.assert_not_called()
                self.assertFalse((root / "dist").exists())
                self.assertFalse((root / "staging").exists())


if __name__ == "__main__":
    unittest.main()
