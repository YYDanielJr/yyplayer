"""Windows-only native libmpv GPU probe; does not change the Slint presenter.

Uses the pinned DLL and the self-owned Test-Render fixture, creates temporary
native video windows, and saves actual GPU screenshots under target. No Python
packages, user mpv configuration, registry changes or external mpv.exe needed.
"""
import ctypes as c
import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path
import time


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "target" / "gpu-investigation" / "native"
FIXTURE = ROOT / "target" / "render-regression" / "static" / "static-hevc.mp4"
LIBRARY = ROOT / "third_party" / "mpv" / "windows-x64" / "libmpv-2.dll"


def probe(api, vo, backend, deband, device):
    name = f"{vo}-{backend}-{'deband' if deband else 'plain'}"
    log = OUT / f"{name}.log"
    screenshot = OUT / f"{name}.png"
    handle = api.mpv_create()
    if not handle:
        raise RuntimeError("mpv_create failed")

    def checked(code):
        if code < 0:
            raise RuntimeError(api.mpv_error_string(code).decode("utf-8"))

    def get(name):
        value = api.mpv_get_property_string(handle, name.encode("utf-8"))
        if not value:
            return None
        try:
            return c.string_at(value).decode("utf-8", "replace")
        finally:
            api.mpv_free(value)

    def command(*args):
        values = (c.c_char_p * (len(args) + 1))(
            *(str(arg).encode("utf-8") for arg in args), None
        )
        checked(api.mpv_command(handle, values))

    result = {"case": name, "requested_api": backend, "deband": deband}
    try:
        options = {
            "config": "no", "load-scripts": "no", "terminal": "no",
            "osc": "no", "osd-level": "0", "input-default-bindings": "no",
            "vo": vo, "gpu-api": backend, "hwdec": "auto-copy",
            "audio": "no", "keep-open": "yes", "idle": "yes",
            "geometry": "1280x720", "force-window": "yes",
            "screenshot-format": "png", "screenshot-high-bit-depth": "no",
            "deband": "yes" if deband else "no", "log-file": str(log),
        }
        if backend == "opengl":
            options.update({"gpu-context": "win", "opengl-es": "no"})
        else:
            options.update({"gpu-context": "winvk", "vulkan-device": device})
        for key, value in options.items():
            checked(api.mpv_set_option_string(handle, key.encode(), value.encode("utf-8")))
        checked(api.mpv_initialize(handle))
        command("loadfile", FIXTURE, "replace")
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            api.mpv_wait_event(handle, 0.05)
            position = get("time-pos")
            if position is not None and float(position) >= 1.5:
                break
        else:
            raise RuntimeError("No real playback position before timeout")
        checked(api.mpv_set_property_string(handle, b"pause", b"yes"))
        command("screenshot-to-file", screenshot, "window")
        result.update({"current_vo": get("current-vo"), "hwdec": get("hwdec-current"),
                       "position": get("time-pos"), "pause": get("pause"),
                       "mpv_version": get("mpv-version"), "screenshot": screenshot.name})
        if not screenshot.is_file() or screenshot.stat().st_size < 1000:
            raise RuntimeError("No GPU screenshot")
        result["passed"] = True
    except Exception as error:
        result.update({"passed": False, "error": str(error)})
    finally:
        api.mpv_terminate_destroy(handle)
    if log.is_file():
        result["gpu_evidence"] = [line for line in log.read_text(encoding="utf-8", errors="replace").splitlines()
                                  if any(token in line for token in ("GL_VERSION=", "GL_RENDERER=", "Selected device", "Device Name:", "Shader compile failed", "compilation errors"))][:20]
    print(json.dumps(result, ensure_ascii=False), flush=True)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--vulkan-device", default="AMD Radeon(TM) 860M Graphics")
    arguments = parser.parse_args()
    if os.name != "nt":
        raise RuntimeError("Native Windows GPU probe only")
    manifest = json.loads((ROOT / "third_party/mpv/manifest.json").read_text())
    with LIBRARY.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    if digest != manifest["library_sha256"]:
        raise RuntimeError("Pinned runtime hash mismatch")
    if not FIXTURE.is_file():
        raise RuntimeError("Run Test-Render to generate its own static HEVC fixture first")
    OUT.mkdir(parents=True, exist_ok=True)
    api = c.CDLL(str(LIBRARY), winmode=0x100 | 0x1000)
    definitions = {
        "mpv_create": (c.c_void_p, []),
        "mpv_initialize": (c.c_int, [c.c_void_p]),
        "mpv_set_option_string": (c.c_int, [c.c_void_p, c.c_char_p, c.c_char_p]),
        "mpv_set_property_string": (c.c_int, [c.c_void_p, c.c_char_p, c.c_char_p]),
        "mpv_get_property_string": (c.c_void_p, [c.c_void_p, c.c_char_p]),
        "mpv_free": (None, [c.c_void_p]),
        "mpv_command": (c.c_int, [c.c_void_p, c.POINTER(c.c_char_p)]),
        "mpv_wait_event": (c.c_void_p, [c.c_void_p, c.c_double]),
        "mpv_error_string": (c.c_char_p, [c.c_int]),
        "mpv_terminate_destroy": (None, [c.c_void_p]),
    }
    for name, (result, argument_types) in definitions.items():
        function = getattr(api, name)
        function.restype, function.argtypes = result, argument_types
    # Also probe the embedding API instead of assuming the native VO options
    # imply a Vulkan render context usable with Slint.
    class RenderParam(c.Structure):
        _fields_ = [("type", c.c_int), ("data", c.c_void_p)]

    render_create = api.mpv_render_context_create
    render_create.argtypes = [c.POINTER(c.c_void_p), c.c_void_p, c.POINTER(RenderParam)]
    render_create.restype = c.c_int
    render_free = api.mpv_render_context_free
    render_free.argtypes, render_free.restype = [c.c_void_p], None
    handle = api.mpv_create()
    if not handle:
        raise RuntimeError("mpv_create failed")
    try:
        for name, value in ((b"config", b"no"), (b"vo", b"libmpv"), (b"terminal", b"no")):
            code = api.mpv_set_option_string(handle, name, value)
            if code < 0:
                raise RuntimeError(api.mpv_error_string(code).decode())
        code = api.mpv_initialize(handle)
        if code < 0:
            raise RuntimeError(api.mpv_error_string(code).decode())
        backend_name = c.create_string_buffer(b"vulkan")
        params = (RenderParam * 2)(RenderParam(1, c.cast(backend_name, c.c_void_p)), RenderParam(0, None))
        context = c.c_void_p()
        code = render_create(c.byref(context), handle, params)
        render_api = {"api": "vulkan", "code": code, "message": api.mpv_error_string(code).decode()}
        if context:
            render_free(context)
    finally:
        api.mpv_terminate_destroy(handle)
    print(json.dumps(render_api), flush=True)
    results = [probe(api, vo, backend, deband, arguments.vulkan_device)
               for vo, backend in (("gpu", "opengl"), ("gpu", "vulkan"), ("gpu-next", "vulkan"))
               for deband in (False, True)]
    comparisons = []
    compare = ROOT / "target/debug/examples/render-compare.exe"
    if not compare.is_file():
        raise RuntimeError("Build the render-compare example first")
    if all(result["passed"] for result in results):
        pairs = [(f"{vo}-{backend}-plain", f"{vo}-{backend}-deband")
                 for vo, backend in (("gpu", "opengl"), ("gpu", "vulkan"), ("gpu-next", "vulkan"))]
        pairs += [(f"gpu-opengl-{mode}", f"gpu-vulkan-{mode}") for mode in ("plain", "deband")]
        for reference, candidate in pairs:
            report = OUT / f"{reference}-vs-{candidate}.json"
            process = subprocess.run([str(compare), str(OUT / f"{reference}.png"), str(OUT / f"{candidate}.png"), str(report)], check=False)
            metrics = json.loads(report.read_text()) if report.is_file() else {}
            comparisons.append({"reference": reference, "candidate": candidate, "exit_code": process.returncode, "metrics": metrics})
    with FIXTURE.open("rb") as source:
        fixture_hash = hashlib.file_digest(source, "sha256").hexdigest()
    summary = {"library_sha256": digest, "fixture_sha256": fixture_hash, "render_api": render_api, "results": results, "comparisons": comparisons}
    (OUT / "results.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")
    return 0 if all(result["passed"] for result in results) and comparisons and all(item["exit_code"] == 0 for item in comparisons) else 1


if __name__ == "__main__":
    raise SystemExit(main())
