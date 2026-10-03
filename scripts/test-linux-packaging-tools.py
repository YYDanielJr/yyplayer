#!/usr/bin/env python3
"""Missing tools must fail before downloading, compiling or producing partial packages."""
import contextlib
import importlib.util
import io
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("packaging", Path(__file__).with_name("package-linux.py"))
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


class ToolPreflightTests(unittest.TestCase):
    def test_all_missing_tools_reported_before_any_work(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            def which(name):
                return None if name in {"file", "patchelf", "cargo"} else "/fixture/" + name
            with patch.object(sys, "argv", ["package-linux.py", "--skip-build"]), patch.object(packaging, "DIST", root / "dist"), patch.object(packaging, "TARGET", root / "staging"), patch.object(packaging.shutil, "which", side_effect=which), patch.object(packaging, "output") as output, patch.object(packaging, "run") as run:
                with self.assertRaises(RuntimeError) as error:
                    packaging.main()
                for name in ("file", "patchelf", "cargo", "apt", "Rust"):
                    self.assertIn(name, str(error.exception))
                output.assert_not_called()
                run.assert_not_called()
                self.assertFalse((root / "dist").exists())
                self.assertFalse((root / "staging").exists())

    def test_check_only_does_not_require_git_metadata_or_download(self):
        with patch.object(sys, "argv", ["package-linux.py", "--check-tools"]), patch.object(packaging.shutil, "which", side_effect=lambda name: "/fixture/" + name), patch.object(packaging, "output") as output, patch.object(packaging, "run") as run, contextlib.redirect_stdout(io.StringIO()) as stdout:
            packaging.main()
            output.assert_not_called()
            run.assert_not_called()
            self.assertIn("检查通过", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
