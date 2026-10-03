#!/usr/bin/env python3
"""Make the supplied source kit buildable offline from its verified crate archives."""
import hashlib
import json
from pathlib import Path
import tarfile

ROOT = Path(__file__).resolve().parents[1]
CRATES = ROOT.parent / "rust-crates"

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    records = json.loads((CRATES / "manifest.json").read_text())
    vendor = ROOT / "vendor"
    vendor.mkdir(exist_ok=True)
    for entry in records:
        archive = CRATES / Path(entry["archive"]).name
        if sha(archive) != entry["sha256"]:
            raise RuntimeError(f"Source archive digest differs: {archive.name}")
        name = entry["name"] + "-" + entry["version"]
        destination = vendor / name
        if destination.exists():
            raise RuntimeError(f"Vendor destination already exists; use a fresh source-kit directory: {destination}")
        with tarfile.open(archive) as file:
            if any(Path(member.name).parts[0] != name for member in file.getmembers()):
                raise RuntimeError(f"Unexpected crate archive root: {archive.name}")
            file.extractall(vendor, filter="data")
        checksums = {str(path.relative_to(destination)): sha(path) for path in destination.rglob("*") if path.is_file()}
        (destination / ".cargo-checksum.json").write_text(json.dumps({"files": checksums, "package": entry["sha256"]}))
    config = ROOT / ".cargo/config.toml"
    config.parent.mkdir(exist_ok=True)
    if config.exists():
        raise RuntimeError("Existing Cargo config is preserved; add source replacement manually")
    config.write_text('[source.crates-io]\nreplace-with = "source-kit"\n[source.source-kit]\ndirectory = "vendor"\n')
    print("Verified crate sources prepared. Run cargo build --locked --offline --release -p yyplayer-app --bin yyplayer")

if __name__ == "__main__":
    main()
