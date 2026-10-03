#!/usr/bin/env python3
"""Boundary tests: minimum inference must not turn missing/infra results into support."""
import argparse
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("compat", Path(__file__).with_name("run.py"))
compat = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compat)


def record(case, status="passed"):
    return {"schema_version": 1, "case": case, "status": status, "phase": "complete" if status == "passed" else "compile", "exit_code": 0 if status == "passed" else 1, "compile_attempted": True, "rust_requested": "1.99.0", "commit": "source", "binary_sha256": "a" * 64, "lock_sha256": hashlib.sha256((compat.ROOT / "Cargo.lock").read_bytes()).hexdigest()}


class ReportTests(unittest.TestCase):
    def setUp(self):
        self.env = patch.dict(os.environ, {"GITHUB_RUN_ID": "", "GITHUB_RUN_ATTEMPT": ""})
        self.env.start()
        self.addCleanup(self.env.stop)
        self.ubuntu = [c for c in compat.cases() if c["family"] == "ubuntu"]
        self.debian = [c for c in compat.cases() if c["family"] == "debian"]

    def summary(self, expected, observations):
        return compat.summarize(expected, observations, "1.99.0", "source")

    def test_numeric_debian_order_and_actual_failures(self):
        cases = self.debian[:3]
        report = self.summary(cases, [record(cases[2]), record(cases[0], "build_failed"), record(cases[1])])
        self.assertEqual(report["minima"]["debian"]["oldest_passed"], "10")
        self.assertTrue(report["minima"]["debian"]["minimum_demonstrated_within_matrix"])

    def test_environment_failure_does_not_prove_minimum(self):
        observations = [record(self.ubuntu[0], "environment_failed"), record(self.ubuntu[1])]
        minimum = self.summary(self.ubuntu[:2], observations)["minima"]["ubuntu"]
        self.assertEqual(minimum["oldest_passed"], "20.04")
        self.assertFalse(minimum["minimum_demonstrated_within_matrix"])
        self.assertEqual(minimum["earlier_unresolved"], ["ubuntu-18.04"])

    def test_testing_cannot_establish_stable_debian_minimum(self):
        testing = compat.cases(True)[-1]
        report = self.summary(self.debian + [testing], [record(testing)])
        self.assertIsNone(report["minima"]["debian"]["oldest_passed"])
        self.assertIn("没有观测到编译通过版本", compat.markdown(report))

    def test_missing_duplicate_malformed_are_not_successes(self):
        c = self.ubuntu[0]
        self.assertEqual(self.summary([c], [{"case": []}])["results"][0]["status"], "missing")
        self.assertEqual(self.summary([c], [record(c), record(c)])["results"][0]["status"], "invalid")
        broken = record(c)
        broken["binary_sha256"] = "not a digest"
        self.assertEqual(self.summary([c], [broken])["results"][0]["status"], "invalid")
        broken.update(status=[], phase=None)
        report = self.summary([c], [broken])
        self.assertEqual(report["results"][0]["status"], "invalid")
        self.assertIn("Malformed status / phase", compat.markdown(report))

    def test_stale_commit_toolchain_lock_or_attempt_refused(self):
        c = self.ubuntu[0]
        for field, value in [("commit", "stale"), ("rust_requested", "1.92.0"), ("lock_sha256", "stale")]:
            with self.subTest(field=field):
                r = record(c)
                r[field] = value
                self.assertEqual(self.summary([c], [r])["results"][0]["status"], "invalid")
        with patch.dict(os.environ, {"GITHUB_RUN_ID": "42", "GITHUB_RUN_ATTEMPT": "2"}):
            r = {**record(c), "run_id": "42", "run_attempt": "1"}
            self.assertEqual(self.summary([c], [r])["results"][0]["status"], "invalid")

    def test_glibc_weak_symbols_do_not_raise_floor(self):
        symbols = "2: 0 0 FUNC GLOBAL DEFAULT UND acosf@GLIBC_2.27 (2)\n3: 0 0 FUNC WEAK DEFAULT UND statx@GLIBC_2.39 (3)\n"
        self.assertEqual(compat.required_glibc(symbols), "2.27")

    def test_success_requires_finished_container(self):
        self.assertEqual(compat.classify("compile", 0), "incomplete")
        self.assertEqual(compat.classify("compile", 101), "build_failed")
        self.assertEqual(compat.classify("compile", 0, True), "timeout")

    def test_failure_without_compiler_evidence_cannot_establish_minimum(self):
        broken = record(self.ubuntu[0], "build_failed")
        broken.update(phase="fetch", compile_attempted=False)
        report = self.summary(self.ubuntu[:2], [broken, record(self.ubuntu[1])])
        self.assertEqual(report["results"][0]["status"], "invalid")
        self.assertFalse(report["minima"]["ubuntu"]["minimum_demonstrated_within_matrix"])

    def test_wrong_case_version_keeps_expected_order_as_unresolved(self):
        wrong = record({**self.ubuntu[0], "version": "99.04", "family": "other"})
        report = self.summary(self.ubuntu[:2], [wrong, record(self.ubuntu[1])])
        self.assertEqual(report["results"][0]["case"], self.ubuntu[0])
        self.assertEqual(report["results"][0]["status"], "invalid")
        self.assertEqual(report["minima"]["ubuntu"]["earlier_unresolved"], ["ubuntu-18.04"])

    def test_aggregate_writes_json_markdown_and_github_summary_with_missing_cases(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            artifact = root / "observations" / "ubuntu"
            artifact.mkdir(parents=True)
            (artifact / "result.json").write_text(json.dumps(record(self.ubuntu[1])))
            malformed = root / "observations" / "malformed"
            malformed.mkdir()
            (malformed / "result.json").write_text("{")
            args = argparse.Namespace(input=root / "observations", output=root / "report", rust_version="1.99.0", commit="source", include_testing=False)
            with patch.dict(os.environ, {"GITHUB_STEP_SUMMARY": str(root / "github-summary.md")}), contextlib.redirect_stdout(io.StringIO()):
                compat.aggregate(args)
            report = json.loads((args.output / "report.json").read_text())
            self.assertEqual(len(report["results"]), 10)
            self.assertEqual(report["minima"]["ubuntu"]["oldest_passed"], "20.04")
            self.assertFalse(report["minima"]["ubuntu"]["minimum_demonstrated_within_matrix"])
            self.assertEqual((root / "github-summary.md").read_text(), (args.output / "report.md").read_text())

    def test_execute_rejects_wrong_os_and_classifies_oom_as_unresolved(self):
        for scenario in ("wrong_os", "oom", "correct_os"):
            with self.subTest(scenario=scenario), tempfile.TemporaryDirectory() as folder:
                output = Path(folder) / "result"
                args = argparse.Namespace(case=self.ubuntu[0]["id"], rust_version="1.99.0", output=output, timeout_seconds=1, keep_binary=False)
                def docker(command, **kwargs):
                    if command[1] == "image":
                        return subprocess.CompletedProcess(command, 0, stdout='["ubuntu@sha256:test"]')
                    if command[1] == "run":
                        if scenario == "oom":
                            (output / "phase").write_text("compile")
                            return subprocess.CompletedProcess(command, 137)
                        version = "18.04" if scenario == "correct_os" else "20.04"
                        for name, content in {"phase": "complete", "exit-code": "0", "os-release": 'ID=ubuntu\nVERSION_ID="' + version + '"\n', "binary.sha256": "a" * 64 + "  /scratch/target/release/yyplayer"}.items():
                            (output / name).write_text(content)
                    return subprocess.CompletedProcess(command, 0)
                with patch.object(compat.subprocess, "run", side_effect=docker), contextlib.redirect_stdout(io.StringIO()):
                    compat.execute(args)
                report = json.loads((output / "result.json").read_text())
                self.assertEqual(report["status"], {"wrong_os": "invalid", "oom": "resource_failed", "correct_os": "passed"}[scenario])

    def test_markdown_escapes_diagnostics(self):
        r = record(self.ubuntu[0], "build_failed")
        r["error"] = "bad | package\nextra row"
        text = compat.markdown(self.summary(self.ubuntu[:1], [r]))
        self.assertIn("bad \\| package extra row", text)

    def test_docker_timeout_produces_report_and_cleans_only_own_container(self):
        calls = []
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "result"
            args = argparse.Namespace(case=self.ubuntu[0]["id"], rust_version="1.99.0", output=output, timeout_seconds=1, keep_binary=False)
            def docker(command, **kwargs):
                calls.append(command)
                if command[1] == "image":
                    return subprocess.CompletedProcess(command, 0, stdout='["ubuntu@sha256:test"]')
                if command[1] == "run":
                    (output / "phase").write_text("compile")
                    raise subprocess.TimeoutExpired(command, 1)
                return subprocess.CompletedProcess(command, 0)
            with patch.object(compat.subprocess, "run", side_effect=docker), contextlib.redirect_stdout(io.StringIO()):
                compat.execute(args)
            report = json.loads((output / "result.json").read_text())
            self.assertEqual(report["status"], "timeout")
            self.assertTrue(report["compile_attempted"])
            launch = next(c for c in calls if c[1] == "run")
            cleanup = next(c for c in calls if c[1] == "rm")
            self.assertEqual(cleanup[-1], launch[launch.index("--name") + 1])
            self.assertIn("dst=/workspace,readonly", " ".join(launch))

    def test_missing_docker_is_reported_as_image_failure(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / "result"
            args = argparse.Namespace(case=self.ubuntu[0]["id"], rust_version="1.99.0", output=output, timeout_seconds=1, keep_binary=False)
            with patch.object(compat.subprocess, "run", side_effect=FileNotFoundError("docker absent")), contextlib.redirect_stdout(io.StringIO()):
                compat.execute(args)
            report = json.loads((output / "result.json").read_text())
            self.assertEqual(report["status"], "image_failed")
            self.assertFalse(report["compile_attempted"])


if __name__ == "__main__":
    unittest.main()
