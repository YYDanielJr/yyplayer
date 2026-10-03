#!/usr/bin/env python3
"""Exercise packaging Git queries with Git's own foreign-owner test hook."""
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("packaging", Path(__file__).with_name("package-linux.py"))
packaging = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packaging)


class RepositoryOwnershipTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory(prefix="yyplayer-git-test-")
        self.addCleanup(folder.cleanup)
        self.folder = Path(folder.name)
        self.repo = self.folder / "checkout 中文 with spaces"
        self.repo.mkdir()
        self.config = self.folder / "gitconfig"
        self.config.write_text("[user]\n\tname = Packaging fixture\n")
        self.env = patch.dict(os.environ, {
            "GIT_CONFIG_GLOBAL": str(self.config), "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_COUNT": "0", "GIT_TEST_ASSUME_DIFFERENT_OWNER": "0",
        })
        self.env.start()
        self.addCleanup(self.env.stop)
        self.git("init", "--quiet")
        self.tracked = "tracked\nname.txt"
        (self.repo / self.tracked).write_text("original\n")
        self.git("add", "--", self.tracked)
        self.git("-c", "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false",
                 "-c", "core.hooksPath=/dev/null", "commit", "--quiet", "-m", "Fixture")
        self.revision = self.git("rev-parse", "--short=12", "HEAD").stdout.strip()
        self.root = patch.object(packaging, "ROOT", self.repo)
        self.root.start()
        self.addCleanup(self.root.stop)
        os.environ["GIT_TEST_ASSUME_DIFFERENT_OWNER"] = "1"

    def git(self, *args, check=True):
        return subprocess.run(["git", "-C", str(self.repo), *args], capture_output=True, text=True, check=check)

    def test_foreign_owner_reproduced_and_revision_query_fixed(self):
        denied = self.git("rev-parse", "HEAD", check=False)
        self.assertEqual(denied.returncode, 128)
        self.assertIn("dubious ownership", denied.stderr)
        self.assertEqual(packaging.output(packaging.git_command("rev-parse", "--short=12", "HEAD")), self.revision)

    def test_dirty_status_and_nul_delimited_source_inventory(self):
        (self.repo / self.tracked).write_text("changed\n")
        untracked = "new file.txt"
        (self.repo / untracked).write_text("new\n")
        self.assertTrue(packaging.output(packaging.git_command("status", "--porcelain")))
        inventory = subprocess.check_output(packaging.git_command("ls-files", "--cached", "--others", "--exclude-standard", "-z"))
        self.assertEqual({os.fsdecode(n) for n in inventory.split(b"\0") if n}, {self.tracked, untracked})

    def test_trust_is_not_persisted_or_applied_to_other_repositories(self):
        before = self.config.read_bytes()
        packaging.output(packaging.git_command("rev-parse", "HEAD"))
        self.assertEqual(self.config.read_bytes(), before)
        self.assertEqual(self.git("rev-parse", "HEAD", check=False).returncode, 128)
        other = self.folder / "another-repository"
        with patch.dict(os.environ, {"GIT_TEST_ASSUME_DIFFERENT_OWNER": "0"}):
            subprocess.run(["git", "init", "--quiet", str(other)], check=True)
        query = subprocess.run(packaging.git_command("-C", str(other), "status"), capture_output=True, text=True)
        self.assertEqual(query.returncode, 128)
        self.assertIn("dubious ownership", query.stderr)

    def test_failed_repository_preflight_does_not_start_cargo(self):
        def output(args):
            if args[0] == "dpkg":
                return "amd64"
            raise subprocess.CalledProcessError(128, args)
        with patch.object(sys, "argv", ["package-linux.py"]), patch.object(packaging, "check_host_tools"), patch.object(packaging, "output", side_effect=output), patch.object(packaging, "run") as run:
            with self.assertRaises(subprocess.CalledProcessError):
                packaging.main()
            run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
