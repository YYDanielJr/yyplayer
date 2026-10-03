#!/usr/bin/env python3
"""Host-side Docker compile matrix and conservative report aggregation (stdlib only)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
import uuid

ROOT = Path(__file__).resolve().parents[2]
MATRIX = Path(__file__).with_name("matrix.json")
BUILD_COMMAND = "cargo build --locked --offline --release -p yyplayer-app --bin yyplayer"
UNKNOWN = {"environment_failed", "toolchain_failed", "fetch_failed", "image_failed", "resource_failed", "timeout", "incomplete", "missing", "invalid"}


def cases(include_testing=False):
    data = json.loads(MATRIX.read_text())
    return [c for c in data["cases"] if include_testing or c["channel"] == "stable"]


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def read_text(folder, name):
    path = folder / name
    return path.read_text(errors="replace").strip() if path.is_file() else ""


def version_key(version):
    return tuple(int(v) for v in version.split("."))


def classify(phase, code, timed_out=False):
    if timed_out:
        return "timeout"
    if code == 0 and phase == "complete":
        return "passed"
    if phase in {"compile", "inspect"} and code != 0:
        return "build_failed" if phase == "compile" else "inspection_failed"
    return {"environment": "environment_failed", "toolchain": "toolchain_failed", "fetch": "fetch_failed", "image": "image_failed"}.get(phase, "incomplete")


def os_release(text):
    return dict((k, v.strip('"')) for line in text.splitlines() if "=" in line for k, v in [line.split("=", 1)])


def required_glibc(symbols):
    # Weak optional symbols do not establish the main executable's ABI floor.
    values = [m.group(1) for line in symbols.splitlines() if " UND " in line and " WEAK " not in line for m in re.finditer(r"@GLIBC_(\d+(?:\.\d+)+)", line)]
    return max(values, key=version_key) if values else None


def execute(args):
    selected = next(c for c in cases(True) if c["id"] == args.case)
    if not re.fullmatch(r"\d+\.\d+\.\d+", args.rust_version):
        raise ValueError("Use an exact Rust release, e.g. 1.99.0")
    if version_key(args.rust_version) < (1, 92, 0):
        raise ValueError("This workspace requires Rust >= 1.92.0")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        raise ValueError("Use a fresh output directory; stale reports are not reused")
    output.chmod(0o777)  # Writable by container root; host Actions reads afterward.
    name = "yyplayer-compat-" + selected["id"] + "-" + uuid.uuid4().hex[:12]
    started = time.monotonic()
    report = {
        "schema_version": 1, "case": selected, "status": "incomplete", "phase": "image",
        "rust_requested": args.rust_version, "build_command": BUILD_COMMAND,
        "commit": os.environ.get("GITHUB_SHA", ""), "run_id": os.environ.get("GITHUB_RUN_ID", ""),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT", ""),
        "lock_sha256": hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest(),
        "compile_attempted": False, "runtime_tested": False,
    }
    result_path = output / "result.json"
    write_json(result_path, report)
    code = -1
    phase = "image"
    timed_out = False
    try:
        with (output / "build.log").open("w") as log:
            pull = subprocess.run(["docker", "pull", "--platform", "linux/amd64", selected["image"]], stdout=log, stderr=subprocess.STDOUT, timeout=300)
            code = pull.returncode
            if code == 0:
                inspected = subprocess.run(["docker", "image", "inspect", selected["image"], "--format", "{{json .RepoDigests}}"], text=True, capture_output=True, check=True, timeout=30)
                report["image_digests"] = json.loads(inspected.stdout)
                code = subprocess.run([
                    "docker", "run", "--rm", "--name", name, "--platform", "linux/amd64",
                    "--mount", "type=bind,src=" + str(ROOT) + ",dst=/workspace,readonly",
                    "--mount", "type=bind,src=" + str(output) + ",dst=/reports",
                    "--env", "YYPLAYER_COMPAT_CONTAINER=1", "--env", "RUST_VERSION=" + args.rust_version,
                    "--env", "CASE_FAMILY=" + selected["family"], "--env", "CASE_CODENAME=" + selected["codename"],
                    "--env", "CASE_ARCHIVE=" + str(selected["archive"]).lower(),
                    "--env", "KEEP_BINARY=" + str(args.keep_binary).lower(),
                    selected["image"], "bash", "/workspace/scripts/linux-compat/container-build.sh",
                ], stdout=log, stderr=subprocess.STDOUT, timeout=args.timeout_seconds).returncode
                phase = read_text(output, "phase") or "incomplete"
    except subprocess.TimeoutExpired as error:
        timed_out = True
        phase = read_text(output, "phase") or phase
        report["error"] = "Command timeout: " + str(error.cmd[0])
    except (OSError, subprocess.CalledProcessError, ValueError) as error:
        report["error"] = str(error)
    finally:
        # Exact per-job container only; a timed-out docker client may leave it running.
        try:
            subprocess.run(["docker", "rm", "--force", name], capture_output=True, timeout=30)
        except (OSError, subprocess.TimeoutExpired):
            pass
    report.update({"phase": phase, "exit_code": code, "elapsed_seconds": round(time.monotonic() - started, 2), "status": classify(phase, code, timed_out), "compile_attempted": phase in {"compile", "inspect", "complete"}})
    actual_os = os_release(read_text(output, "os-release"))
    report["actual_os"] = actual_os
    report["container_glibc"] = read_text(output, "glibc.txt")
    report["rustc"] = read_text(output, "rustc.txt")
    report["cargo"] = read_text(output, "cargo.txt")
    report["main_binary_required_glibc"] = required_glibc(read_text(output, "symbols.txt"))
    report["binary_sha256"] = read_text(output, "binary.sha256").split(" ")[0] or None
    if report["status"] == "build_failed":
        with (output / "build.log").open("rb") as log:
            log.seek(max(0, log.seek(0, 2) - 65536))
            tail = log.read().decode(errors="replace")
        if code == 137 or any(marker in tail for marker in ("No space left on device", "signal: 9, SIGKILL", "Killed signal terminated program", "ld terminated with signal 9")):
            report.update(status="resource_failed", error="Build was interrupted by resource exhaustion; see build.log")
    if report["status"] == "passed":
        version_matches = actual_os.get("VERSION_ID", "").split(".")[0] == selected["version"] if selected["family"] == "debian" else actual_os.get("VERSION_ID") == selected["version"]
        if selected["channel"] == "testing":
            version_matches = version_matches or actual_os.get("VERSION_CODENAME") == selected["codename"]
        if actual_os.get("ID") != selected["family"] or not version_matches or not re.fullmatch(r"[0-9a-f]{64}", report["binary_sha256"] or "") or read_text(output, "exit-code") != "0":
            report["status"] = "invalid"
            report["error"] = "Container identity / completion evidence does not match the requested case"
    write_json(result_path, report)
    print(selected["id"] + ": " + report["status"] + " (" + phase + ")", flush=True)
    # Negative matrix observations are data; one failure must not cancel others.
    return 0


def summarize(expected, records, rust_version, commit):
    rows = []
    for case in expected:
        matches = [r for r in records if isinstance(r.get("case"), dict) and r["case"].get("id") == case["id"]]
        if not matches:
            row = {"case": case, "status": "missing", "phase": "unknown", "error": "No result artifact; job/upload may have failed or timed out"}
        elif len(matches) != 1:
            row = {"case": case, "status": "invalid", "phase": "unknown", "error": "Duplicate result artifacts"}
        else:
            row = dict(matches[0])
            if not isinstance(row.get("status"), str) or not isinstance(row.get("phase"), str):
                row.update(status="invalid", phase="unknown", error="Malformed status / phase")
            if row.get("case") != case or row.get("rust_requested") != rust_version or row.get("commit") != commit or row.get("schema_version") != 1:
                row.update(status="invalid", error="Case/toolchain/commit/schema differs from this workflow run")
            # Inference always follows the expected matrix ordering, even when
            # an artifact reports a different family/version for the same ID.
            if row.get("case") != case:
                row["reported_case"] = row.get("case")
                row["case"] = case
            for field, expected_value in (("run_id", os.environ.get("GITHUB_RUN_ID")), ("run_attempt", os.environ.get("GITHUB_RUN_ATTEMPT"))):
                if expected_value and row.get(field) != expected_value:
                    row.update(status="invalid", error="Artifact belongs to a different run / attempt")
            expected_lock = hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest()
            if row.get("lock_sha256") != expected_lock:
                row.update(status="invalid", error="Cargo.lock differs from the selected source")
            if row.get("status") == "passed" and (row.get("compile_attempted") is not True or row.get("phase") != "complete" or type(row.get("exit_code")) is not int or row.get("exit_code") != 0 or not re.fullmatch(r"[0-9a-f]{64}", str(row.get("binary_sha256", "")))):
                row.update(status="invalid", error="Successful compile completion evidence absent")
            if row.get("status") in {"build_failed", "inspection_failed"}:
                expected_phase = "compile" if row["status"] == "build_failed" else "inspect"
                if row.get("compile_attempted") is not True or row.get("phase") != expected_phase or type(row.get("exit_code")) is not int or row["exit_code"] <= 0:
                    row.update(status="invalid", error="Actual build / inspection failure evidence absent")
            if row.get("status") not in UNKNOWN | {"passed", "build_failed", "inspection_failed"}:
                row.update(status="invalid", error="Unrecognized status")
        rows.append(row)
    minima = {}
    for family in ("ubuntu", "debian"):
        stable = sorted((r for r in rows if r["case"]["family"] == family and r["case"]["channel"] == "stable"), key=lambda r: version_key(r["case"]["version"]))
        passing = [r for r in stable if r["status"] == "passed"]
        first = passing[0] if passing else None
        unresolved = [r["case"]["id"] for r in stable if (not first or version_key(r["case"]["version"]) < version_key(first["case"]["version"])) and r["status"] not in {"passed", "build_failed"}]
        minima[family] = {"oldest_passed": first["case"]["version"] if first else None, "earlier_unresolved": unresolved, "minimum_demonstrated_within_matrix": bool(first and not unresolved)}
    return {"schema_version": 1, "rust_version": rust_version, "commit": commit, "build_command": BUILD_COMMAND, "runtime_tested": False, "minima": minima, "results": rows}


def cell(value):
    return str(value or "—").replace("|", "\\|").replace("\n", " ").replace("\r", " ")


def markdown(report):
    lines = ["# Linux 编译兼容性报告", "", "提交：`" + report["commit"] + "`；统一 Rust：`" + report["rust_version"] + "`。", "", "命令：`" + BUILD_COMMAND + "`。所有 case 使用锁定 Cargo.lock、独立容器与 target；只测试完整 Release 编译，不测试播放 / GPU / libmpv 运行时或旧内核。", ""]
    for family, name in (("ubuntu", "Ubuntu LTS"), ("debian", "Debian 正式版本")):
        minimum = report["minima"][family]
        if minimum["oldest_passed"]:
            suffix = "更早的矩阵版本均已实际编译失败。" if minimum["minimum_demonstrated_within_matrix"] else "更早版本仍有未完成 / 环境失败，不能确定绝对下限。"
            oldest_tested = min((r["case"]["version"] for r in report["results"] if r["case"]["family"] == family and r["case"]["channel"] == "stable"), key=version_key)
            if minimum["oldest_passed"] == oldest_tested:
                suffix = "它是本矩阵起点；更早系统未测试。"
            lines.append("- " + name + "最低实测编译通过版本：**" + minimum["oldest_passed"] + "**。" + suffix)
        else:
            lines.append("- " + name + "：**没有观测到编译通过版本**；最低版本未确定。")
        if minimum["earlier_unresolved"]:
            lines.append("  未决项：" + ", ".join(minimum["earlier_unresolved"]) + "。")
    lines += ["", "testing 单列，不能改变正式 Debian 的最低结论。未提供任何运行兼容性结论。", "", "| 系统 | 结果 | 阶段 | 容器 glibc | 主程序 GLIBC 下限 | 用时（秒） |", "| --- | --- | --- | --- | --- | --- |"]
    for row in report["results"]:
        label = row["case"]["id"] + (" (testing)" if row["case"]["channel"] == "testing" else "")
        lines.append("| " + " | ".join(map(cell, [label, row["status"], row.get("phase"), row.get("container_glibc"), row.get("main_binary_required_glibc"), row.get("elapsed_seconds")])) + " |")
    lines += ["", "## 失败 / 未决项", ""]
    for row in report["results"]:
        if row["status"] != "passed":
            lines.append("- **" + row["case"]["id"] + "**：" + cell(row.get("error", "见该 case artifact 的 build.log；阶段 " + cell(row.get("phase")))) + "。")
    lines += ["", "每个 case artifact 保留完整构建日志、image digest、系统 / 包 / 编译器版本、可用 mpv 包版本以及 ELF / ldd / GLIBC 符号证据。主程序 GLIBC 下限不代表所有 dlopen 依赖或 GPU 驱动的下限。镜像 tag 与 apt 仓库可能更新，按该次记录解释结果。", ""]
    return "\n".join(lines)


def aggregate(args):
    records = []
    for path in args.input.rglob("result.json"):
        try:
            value = json.loads(path.read_text())
            if not isinstance(value, dict):
                raise ValueError("Result is not an object")
            records.append(value)
        except (ValueError, OSError) as error:
            print("Unreadable result: " + str(path) + ": " + str(error))
    report = summarize(cases(args.include_testing), records, args.rust_version, args.commit)
    args.output.mkdir(parents=True, exist_ok=True)
    write_json(args.output / "report.json", report)
    text = markdown(report)
    (args.output / "report.md").write_text(text)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a") as file:
            file.write(text)
    print(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    matrix = commands.add_parser("matrix")
    matrix.add_argument("--include-testing", action="store_true")
    run = commands.add_parser("run")
    run.add_argument("--case", required=True, choices=[c["id"] for c in cases(True)])
    run.add_argument("--rust-version", default="1.99.0")
    run.add_argument("--output", required=True, type=Path)
    run.add_argument("--timeout-seconds", type=int, default=5400)
    run.add_argument("--keep-binary", action="store_true")
    summary = commands.add_parser("aggregate")
    summary.add_argument("--input", type=Path, required=True)
    summary.add_argument("--output", type=Path, required=True)
    summary.add_argument("--rust-version", required=True)
    summary.add_argument("--commit", required=True)
    summary.add_argument("--include-testing", action="store_true")
    args = parser.parse_args()
    if args.command == "matrix":
        print(json.dumps({"include": cases(args.include_testing)}, separators=(",", ":")))
    elif args.command == "run":
        return execute(args)
    else:
        aggregate(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
