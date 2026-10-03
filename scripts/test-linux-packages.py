#!/usr/bin/env python3
"""Verify deb/AppImage payloads and real playback, without installing as root."""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("linux_validation", ROOT / "scripts/test-linux.py")
tests = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tests)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", choices=["wayland", "x11"], default="wayland")
    parser.add_argument("--headless", action="store_true")
    parser.add_argument("--skip-fuse", action="store_true", help="Explicitly leave normal FUSE launch unverified")
    args = parser.parse_args()
    tests.BASE.mkdir(parents=True, exist_ok=True)
    tests.fixtures()
    if not (tests.BASE / "static-hevc.mp4").exists():
        tests.ffmpeg("-f", "lavfi", "-i", "smptebars=size=3840x2160:rate=30", "-t", "8", "-c:v", "libx265", "-preset", "ultrafast", "-x265-params", "log-level=error:pools=2:frame-threads=2", "-pix_fmt", "yuv420p", "-an", tests.BASE / "static-hevc.mp4")
    info = json.loads((ROOT / "dist/YYPlayer-linux-BUILD-INFO.json").read_text())
    version = info["version"]
    deb = ROOT / f"dist/yyplayer_{version}_amd64.deb"
    appimage = ROOT / f"dist/YYPlayer-{version}-x86_64.AppImage"
    subprocess.run(["sha256sum", "--check", "SHA256SUMS-linux"], cwd=ROOT / "dist", check=True)
    records = []
    with tempfile.TemporaryDirectory(prefix="packages-", dir=tests.BASE) as temporary, tests.desktop(args.backend, args.headless) as env:
        temporary = Path(temporary)
        extracted_deb = temporary / "deb"
        tests.run(["dpkg-deb", "--extract", deb, extracted_deb])
        player = extracted_deb / "usr/bin/yyplayer"
        tests.expect(hashlib.sha256(player.read_bytes()).hexdigest() == info["binary_sha256"], "deb binary differs from qualified Release")
        subprocess.run([str(appimage), "--appimage-extract"], cwd=temporary, stdout=subprocess.DEVNULL, check=True)
        appdir = temporary / "squashfs-root"
        manifest_path = appdir / "usr/bin/runtime/manifest.json"
        manifest = json.loads(manifest_path.read_text())
        runtime = appdir / "usr/bin/runtime/libmpv.so.2"
        tests.expect(hashlib.sha256(runtime.read_bytes()).hexdigest() == manifest["library_sha256"], "AppImage runtime digest differs")
        tests.expect((appdir / "usr/share/doc/yyplayer/runtime-manifest.json").exists(), "Runtime source/license manifest absent")
        tests.run(["desktop-file-validate", appdir / "yyplayer.desktop"])
        entries = [("deb", player), ("appimage-extracted", appdir / "AppRun")]
        if not args.skip_fuse:
            entries.append(("appimage-fuse", appimage))
        for name, executable in entries:
            print(name, "audio", flush=True)
            scripted = name == "appimage-extracted"
            state = tests.smoke(executable, "package-" + name + "-audio", env, tests.BASE / "song.flac", seconds=14 if scripted else 7, extra={"LD_DEBUG": "libs", **({"YYPLAYER_AUDIO_SCRIPT": "1"} if scripted else {})})
            tests.expect(state["phase"] == "Playing" and state["position"] > 2 and not state["error"] and state["exclusive"] is False, name + " real audio failed")
            tests.expect(state["render_lifecycle"]["initializations"] == 0, name + " audio allocated a renderer")
            if name.startswith("appimage"):
                log = (tests.BASE / ("package-" + name + "-audio") / "stdout.log").read_text(errors="replace")
                tests.expect("/usr/bin/runtime/libmpv.so.2" in log, name + " did not load bundled libmpv")
            if scripted:
                tests.expect(state["script_steps"] == 7 and not state["eq_filter"] and any("g=4.500000" in s["eq_filter"] for s in state["checkpoints"]) and any("g=-6.000000" in s["eq_filter"] for s in state["checkpoints"]), "AppImage bundled EQ / output recovery script failed")
            print(name, "video", flush=True)
            video = tests.smoke(executable, "package-" + name + "-video", env, tests.BASE / "static-hevc.mp4", seconds=7)
            tests.expect(video["video"] and video["render_frames"] >= 60 and not video["error"], name + " real video failed")
            records.append({"package": name, "audio_phase": state["phase"], "video_frames": video["render_frames"], "hwdec": video["hwdec"], "window_system": video["window_system"]})
        state = tests.smoke(appdir / "AppRun", "package-missing-device", env, tests.BASE / "song.flac", settings={"device": "pipewire/yyplayer-validation-missing"})
        tests.expect(state["audio_blocked"] and state["error"] and state["status"] and state["audio_output_rate"] is None and state["tracks"] == 0 and state["phase"] != "Playing", "Packaged player initialized default output for an unavailable saved device")
        # Damage only the temporary extracted manifest; the distributed asset is untouched.
        damaged = {**manifest, "library_sha256": "0" * 64}
        manifest_path.write_text(json.dumps(damaged))
        state = tests.smoke(appdir / "AppRun", "package-damaged-manifest", env, seconds=4)
        tests.expect("校验失败" in state["error"] and not state["ready"] and state["render_lifecycle"]["initializations"] == 0, "Damaged bundle silently bypassed to system libmpv")
        report = {"passed": True, "backend": args.backend, "headless": args.headless, "suite": "packages", "fuse_launch_verified": not args.skip_fuse, "damaged_bundle_refused": True, "unavailable_saved_device_refused": True, "installed_as_root": False, "packages": records}
        (tests.BASE / "packages-results.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print("PASS: package payloads / real playback / checked runtime / normal exit")

if __name__ == "__main__":
    main()
