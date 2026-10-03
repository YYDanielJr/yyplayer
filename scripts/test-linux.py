#!/usr/bin/env python3
"""Linux qualification with owned fixtures and independent target/ configuration."""
import argparse
import contextlib
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time
import wave

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "target/linux-validation"

def run(command, *, env=None, timeout=150, log=None):
    if log:
        log.parent.mkdir(parents=True, exist_ok=True)
        with log.open("w") as file:
            result = subprocess.run([os.fspath(a) for a in command], cwd=ROOT, env=env, stdout=file, stderr=subprocess.STDOUT, timeout=timeout)
        if result.returncode:
            raise RuntimeError(f"{command[0]} exited {result.returncode}; {log}\n{log.read_text(errors='replace')[-3000:]}")
    else:
        subprocess.run([os.fspath(a) for a in command], cwd=ROOT, env=env, check=True, timeout=timeout)

def ffmpeg(*args):
    run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args])

def silent(path, seconds=32):
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as file:
        file.setnchannels(2); file.setsampwidth(2); file.setframerate(44100)
        file.writeframes(b"\0" * (44100 * 4 * seconds))

def fixtures():
    BASE.mkdir(parents=True, exist_ok=True)
    silent(BASE / "silence.wav")
    cover = ROOT / "target/audio-validation/cover.png"
    cover.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "assets/icons/yyplayer/liquid-orbit-disc/yyplayer-256.png", cover)
    if not (BASE / "video.mp4").exists():
        ffmpeg("-f", "lavfi", "-i", "smptebars=size=1280x720:rate=30", "-t", "24", "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", "-an", BASE / "video.mp4")
    ffmpeg("-i", BASE / "silence.wav", "-i", cover, "-map", "0:a", "-map", "1:v", "-c:a", "flac", "-c:v", "copy", "-disposition:v", "attached_pic", "-metadata", "title=Lake Lights", "-metadata", "artist=YYPlayer Validation", "-metadata", "album=Local Sessions", BASE / "song.flac")
    (BASE / "song.lrc").write_text("\n".join(f"[00:{i*3:02}.00]YYPlayer Linux lyric {i}" for i in range(6)))
    video = ROOT / "target/video-library"
    for name in ["library/A/湖边.mp4", "library/B/天空.mp4", "loose.mp4"]:
        dest = video / name; dest.parent.mkdir(parents=True, exist_ok=True); shutil.copy2(BASE / "video.mp4", dest)
    shutil.copy2(BASE / "silence.wav", video / "silence.wav")
    for folder, name in [("Quiet Hours", "Evening Air"), ("Quiet Hours", "Still Water"), ("Open Skies", "Golden Light"), ("Open Skies", "A Little Further")]:
        silent(ROOT / "target/theme-validation/library" / folder / (name + ".wav"))
    custom = ROOT / "target/ui-customization"
    (custom / "library").mkdir(parents=True, exist_ok=True)
    silent(custom / "tone.wav", 4)
    for i in range(1, 81):
        stem = f"{i:02} - Glass Session"
        shutil.copy2(custom / "tone.wav", custom / "library" / (stem + ".wav"))
        shutil.copy2(cover, custom / "library" / (stem + ".png"))
        (custom / "library" / (stem + ".lrc")).write_text("[00:00.00]Lyrics font · Aa 012345\n[00:02.00]Local lyrics · Glass Sessions\n")
    ffmpeg("-i", custom / "tone.wav", "-i", cover, "-map", "0:a", "-map", "1:v", "-metadata", "artist=YYPlayer Studio", "-metadata", "album=Glass Sessions", "-c:a", "flac", "-c:v", "png", "-disposition:v", "attached_pic", custom / "library/01 - Glass Session.flac")
    (custom / "library/01 - Glass Session.wav").unlink(missing_ok=True)
    (custom / "library/01 - Glass Session.png").unlink(missing_ok=True)
    shutil.copy2(BASE / "video.mp4", custom / "subtitle.mp4")
    (custom / "subtitle.srt").write_text("1\n00:00:00,000 --> 00:00:23,000\nSubtitle font · Aa 012345\n")
    (custom / "subtitle.ass").write_text("[Script Info]\nScriptType: v4.00+\nPlayResX: 1280\nPlayResY: 720\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,Noto Sans,36,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,0,2,20,20,20,1\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\nDialogue: 0,0:00:00.00,0:00:23.00,Default,,0,0,0,,ASS font · Aa 012345\n")

@contextlib.contextmanager
def desktop(backend, headless):
    env = os.environ.copy()
    for name in ["YYPLAYER_MPV_LIBRARY", "YYPLAYER_SMOKE_SCRIPT", "YYPLAYER_AUDIO_SCRIPT", "YYPLAYER_UI_CAPTURE", "YYPLAYER_RENDER_TRACE"]:
        env.pop(name, None)
    env["WINIT_UNIX_BACKEND"] = backend
    child = None
    runtime = None
    try:
        if backend == "wayland":
            env.pop("DISPLAY", None)
            if headless:
                import tempfile
                runtime = tempfile.TemporaryDirectory(prefix="yyplayer-weston-", dir=BASE)
                os.chmod(runtime.name, 0o700)
                env["XDG_RUNTIME_DIR"] = runtime.name
                env["WAYLAND_DISPLAY"] = "yyplayer-test"
                child = subprocess.Popen(["weston", "--no-config", "--backend=headless", "--renderer=gl", "--socket=yyplayer-test", "--idle-time=0", "--log=" + str(BASE / "weston.log")], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                for _ in range(100):
                    if (Path(runtime.name) / "yyplayer-test").exists(): break
                    if child.poll() is not None: raise RuntimeError("Weston failed; inspect weston.log")
                    time.sleep(.1)
                else: raise RuntimeError("Weston socket timeout")
        else:
            env.pop("WAYLAND_DISPLAY", None)
            if headless:
                read_fd, write_fd = os.pipe()
                child = subprocess.Popen(["Xvfb", "-displayfd", str(write_fd), "-screen", "0", "1920x1080x24", "-nolisten", "tcp"], pass_fds=(write_fd,), stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                os.close(write_fd)
                with os.fdopen(read_fd) as read:
                    number = read.readline().strip()
                if not number: raise RuntimeError("Xvfb failed to select a display")
                env["DISPLAY"] = ":" + number
        yield env
    finally:
        if child:
            child.terminate()
            try: child.wait(timeout=10)
            except subprocess.TimeoutExpired: child.kill(); child.wait()
        if runtime: runtime.cleanup()

def smoke(player, name, env, media=None, seconds=5, settings=None, extra=None):
    case = BASE / name
    case.mkdir(exist_ok=True)
    config = case / "settings.json"
    config.write_text(json.dumps({"version": 1, "volume": 0, **(settings or {})}))
    report = case / "state.json"
    report.unlink(missing_ok=True)
    scoped = {**env, "YYPLAYER_CONFIG": str(config), "YYPLAYER_SMOKE_SECONDS": str(seconds), "YYPLAYER_DIAGNOSTICS": str(report), **(extra or {})}
    run([player] + ([media] if media else []), env=scoped, log=case / "stdout.log", timeout=seconds + 45)
    if not report.exists(): raise RuntimeError(f"Fresh diagnostics absent: {name}")
    state = json.loads(report.read_text())
    if state.get("window_system") != env["WINIT_UNIX_BACKEND"]: raise RuntimeError(f"Wrong native window system: {state.get('window_system')}")
    if state["render_error"]: raise RuntimeError(f"{name}: {state['render_error']}")
    return state

def expect(value, message):
    if not value: raise RuntimeError(message)

def ui(env):
    state = smoke(ROOT / "target/debug/yyplayer", env["WINIT_UNIX_BACKEND"] + "-ui-backend", env)
    expect(state["ready"] and not state["error"] and state["render_lifecycle"]["initializations"] == 0, "Native UI backend / idle kernel failed")
    for example in ["navigation-ui", "ui-feedback", "audio-ui", "background-preview"]:
        print("UI:", example, flush=True)
        run([ROOT / "target/debug/examples" / example], env=env, log=BASE / f"{env['WINIT_UNIX_BACKEND']}-{example}.log")

def playback(player, env):
    idle = smoke(player, "idle", env)
    expect(idle["runtime"].startswith("mpv v0.41") and not idle["error"], "libmpv initialization failed")
    expect(idle["render_lifecycle"]["initializations"] == 0, "Idle initialized a video renderer")
    shared = smoke(player, "shared", env, BASE / "song.flac")
    expect(shared["phase"] == "Playing" and shared["position"] > 2 and shared["exclusive"] is False and not shared["error"], "Shared playback not confirmed")
    expect(shared["music_title"] == "Lake Lights" and shared["lyric_count"] == 6 and shared["cover_width"] > 0, "Real tags / embedded artwork / lyrics failed")
    expect(shared["render_lifecycle"]["initializations"] == 0, "Audio allocated a video renderer")
    missing = smoke(player, "missing-saved-device", env, BASE / "song.flac", settings={"device": "pipewire/yyplayer-validation-missing"})
    expect(missing["audio_blocked"] and missing["error"] and missing["status"] and missing["phase"] != "Playing" and missing["audio_output_rate"] is None and missing["tracks"] == 0 and (missing["position"] or 0) == 0 and "pipewire/yyplayer-validation-missing" in missing["audio_info"], "Unavailable saved device fell through to default output / lost its visible error")
    raw = os.fsdecode(os.fsencode(BASE) + b"/raw-\xff %?#.flac")
    shutil.copy2(BASE / "song.flac", raw)
    raw_state = smoke(player, "raw-filename", env, raw)
    expect(raw_state["phase"] == "Playing" and not raw_state["error"], "Non-UTF-8 path changed identity / failed to play")
    pipewire = next((d for d in idle["device_ids"] if d.startswith("pipewire/")), None)
    if pipewire:
        exclusive = smoke(player, "pipewire-exclusive", env, BASE / "song.flac", seconds=7, settings={"device": pipewire, "audio": {"mode": "StrictExclusive"}})
        expect(exclusive["exclusive"] is True and not exclusive["error"] and exclusive["phase"] == "Playing", "PipeWire exclusive stream was not connected")
        scripted = smoke(player, "audio-controls", env, BASE / "song.flac", seconds=14, extra={"YYPLAYER_AUDIO_SCRIPT": "1"})
        expect(scripted["script_steps"] == 7 and not scripted["eq_filter"] and scripted["phase"] == "Playing" and not scripted["error"], "EQ / device / exclusive / shared recovery script failed")
        expect(any("g=4.500000" in s["eq_filter"] for s in scripted["checkpoints"]) and any("g=-6.000000" in s["eq_filter"] for s in scripted["checkpoints"]), "EQ precedence evidence missing")
    pulse = next((d for d in idle["device_ids"] if d.startswith("pulse/")), None)
    if pulse:
        result = smoke(player, "pulse-shared", env, BASE / "song.flac", settings={"device": pulse})
        expect(result["exclusive"] is False and not result["error"] and "实际音频 API：pulse" in result["audio_info"], "Pulse shared / concrete backend failed")
        result = smoke(player, "pulse-strict-refusal", env, BASE / "song.flac", settings={"device": pulse, "audio": {"mode": "StrictExclusive"}})
        expect(result["exclusive"] is not True and result["error"] and (result["position"] or 0) < .1, "Unsupported strict output played / falsely confirmed")
        result = smoke(player, "pulse-prefer-fallback", env, BASE / "song.flac", settings={"device": pulse, "audio": {"mode": "PreferExclusive"}})
        expect(result["exclusive"] is False and result["audio_fallback"] and not result["error"], "Same-device Pulse fallback failed")
    video = smoke(player, "video-controls", env, BASE / "video.mp4", seconds=14, extra={"YYPLAYER_SMOKE_SCRIPT": "1"})
    expect(video["video"] and video["render_frames"] >= 60 and video["script_steps"] == 11 and not video["error"], "Real video control/reload script failed")
    return {"idle": idle, "shared": shared, "missing_saved_device": missing, "video": video, "pipewire_device": pipewire, "pulse_device": pulse}

def integration(env):
    for example, folder in [("library-theme", "theme-validation"), ("video-library", "video-library"), ("ui-customization", "ui-customization")]:
        print("Controller:", example, flush=True)
        scoped = {**env, "YYPLAYER_CONFIG": str(ROOT / "target" / folder / "settings.json")}
        run([ROOT / "target/debug/examples" / example], env=scoped, log=BASE / f"{env['WINIT_UNIX_BACKEND']}-{example}.log", timeout=200)

def alsa(player, env, device):
    expect(device and device.startswith("alsa/hw:"), "Select an explicit ALSA hw: device with --alsa-device")
    state = smoke(player, "alsa-direct-strict", env, BASE / "song.flac", seconds=7, settings={"device": device, "audio": {"mode": "StrictExclusive"}})
    opened = device.removeprefix("alsa/")
    expect("ALSA 打开设备：" + opened in state["audio_info"], "ALSA request was routed to the wrong backend")
    if state["exclusive"] is True:
        expect(state["phase"] == "Playing" and state["position"] > 2 and not state["error"], "Confirmed hardware did not play")
        result = "hardware opened and negotiated"
    else:
        expect(state["error"] and state["phase"] != "Playing" and not state["audio_fallback"], "Failed ALSA output switched devices or played")
        result = "refused; no automatic device change"
    return {"device": device, "result": result, "exclusive": state["exclusive"], "source_rate": state["audio_source_rate"], "api_rate": state["audio_output_rate"], "log": state["audio_log"], "error": state["error"]}

def render(player, env, hardware):
    fixture = BASE / "static-hevc.mp4"
    if not fixture.exists():
        ffmpeg("-f", "lavfi", "-i", "smptebars=size=3840x2160:rate=30", "-t", "8", "-c:v", "libx265", "-preset", "ultrafast", "-x265-params", "log-level=error:pools=2:frame-threads=2", "-pix_fmt", "yuv420p", "-an", fixture)
    cases = [("software", "Software", False), ("deband", "Auto", True)]
    if hardware: cases.append(("hardware", "Auto", False))
    records = []
    for name, mode, deband in cases:
        capture = BASE / (name + ".ppm")
        capture.unlink(missing_ok=True)
        state = smoke(player, "render-" + name, env, fixture, seconds=7, settings={"global": {"mode": mode, "deband": deband}}, extra={"YYPLAYER_UI_CAPTURE": str(capture)})
        expect(state["video"] and state["render_frames"] >= 60 and not state["error"] and capture.exists(), name + " GPU capture failed")
        if name == "hardware": expect(state["hwdec"] not in ["", "no"], "Hardware decoder not actually used")
        ffmpeg("-i", capture, "-frames:v", "1", BASE / (name + ".png"))
        records.append({"case": name, "frames": state["render_frames"], "hwdec": state["hwdec"]})
    for name, _, _ in cases[1:]:
        run([ROOT / "target/debug/examples/render-compare", BASE / "software.png", BASE / (name + ".png"), BASE / (name + "-metrics.json")])
    with fixture.open("rb") as file: fixture_hash = hashlib.file_digest(file, "sha256").hexdigest()
    return {"fixture_sha256": fixture_hash, "results": records, "hardware_required": hardware}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--backend", choices=["wayland", "x11"], default="wayland")
    parser.add_argument("--headless", action="store_true")
    parser.add_argument("--suite", choices=["ui", "playback", "integration", "render", "alsa", "all"], default="all")
    parser.add_argument("--alsa-device", help="Explicit alsa/hw: device for the ALSA qualification suite")
    parser.add_argument("--require-hardware", action="store_true")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--player", type=Path, default=ROOT / "target/debug/yyplayer")
    args = parser.parse_args()
    BASE.mkdir(parents=True, exist_ok=True)
    if not args.skip_build:
        run(["cargo", "build", "--locked", "--offline", "-p", "yyplayer-app", "--bin", "yyplayer", "--examples"], timeout=900)
    fixtures()
    result = {"backend": args.backend, "headless": args.headless, "suite": args.suite}
    with desktop(args.backend, args.headless) as env:
        if args.suite in ["ui", "all"]: ui(env); result["ui"] = "PASS"
        if args.suite in ["playback", "all"]: result["playback"] = playback(args.player.resolve(), env)
        if args.suite in ["integration", "all"]: integration(env); result["integration"] = "PASS"
        if args.suite == "alsa": result["alsa"] = alsa(args.player, env, args.alsa_device)
        if args.suite in ["render", "all"]: result["render"] = render(args.player.resolve(), env, args.require_hardware)
    result["passed"] = True
    (BASE / f"{args.backend}-{args.suite}-results.json").write_text(json.dumps(result, ensure_ascii=False, indent=2))
    print("PASS:", args.backend, args.suite, flush=True)

if __name__ == "__main__":
    main()
