#!/usr/bin/env bash
# Runs only inside the disposable compatibility container, never on the host.
set -euo pipefail
[[ "${YYPLAYER_COMPAT_CONTAINER:-}" == 1 && -f /.dockerenv && -f /workspace/Cargo.lock ]] || {
    echo "This script requires the dedicated Docker workflow container." >&2
    exit 2
}
: "${RUST_VERSION:?}" "${CASE_FAMILY:?}" "${CASE_CODENAME:?}" "${CASE_ARCHIVE:?}"
phase=environment
finish() { printf '%s\n' "$?" > /reports/exit-code; }
trap finish EXIT
stage() { phase=$1; printf '%s\n' "$phase" > /reports/phase; }
stage environment
cp /etc/os-release /reports/os-release
uname -a > /reports/shared-host-kernel.txt
getconf GNU_LIBC_VERSION > /reports/glibc.txt
export DEBIAN_FRONTEND=noninteractive
mkdir -p /scratch

if [[ "$CASE_ARCHIVE" == true ]]; then
    [[ "$CASE_FAMILY" == debian && "$CASE_CODENAME" =~ ^(stretch|buster)$ ]]
    # Old archived Release files have expired Valid-Until dates. Signature
    # verification remains enabled; do not use trusted=yes / unauthenticated.
    rm -f /etc/apt/sources.list.d/*.list /etc/apt/sources.list.d/*.sources
    printf 'deb [check-valid-until=no] http://archive.debian.org/debian %s main\n' "$CASE_CODENAME" > /etc/apt/sources.list
fi
cp -r /etc/apt/sources.list.d /reports/apt-sources.d
if [[ -f /etc/apt/sources.list ]]; then cp /etc/apt/sources.list /reports/apt-sources.list; fi
apt-get -o Acquire::Retries=3 -o Acquire::http::Timeout=60 update

# Only native build dependencies. libmpv is dlopen'd: an old runtime package
# must not prevent this compile-only experiment from reaching the compiler.
select_package() {
    local package
    for package in "$@"; do
        if apt-cache policy "$package" | awk '$1 == "Candidate:" && $2 != "(none)" {found=1} END {exit !found}'; then
            printf '%s' "$package"
            return
        fi
    done
    printf 'No candidate for dependency group: %s\n' "$*" >&2
    return 1
}
fontconfig_dev=$(select_package libfontconfig-dev libfontconfig1-dev)
freetype_dev=$(select_package libfreetype-dev libfreetype6-dev)
apt-get install --no-install-recommends -y build-essential pkg-config ca-certificates curl binutils "$fontconfig_dev" "$freetype_dev"
dpkg-query -W -f='${Package}\t${Version}\n' > /reports/packages.tsv
apt-cache policy libmpv1 libmpv2 > /reports/available-mpv.txt
cc --version > /reports/compiler.txt
ld --version > /reports/linker.txt

stage toolchain
export CARGO_HOME=/scratch/cargo
export RUSTUP_HOME=/scratch/rustup
export CARGO_TARGET_DIR=/scratch/target
export CARGO_BUILD_JOBS=2
export CARGO_NET_RETRY=3
export CARGO_HTTP_TIMEOUT=60
curl --proto '=https' --tlsv1.2 --fail --show-error --location --retry 3 https://sh.rustup.rs -o /scratch/rustup.sh
sha256sum /scratch/rustup.sh > /reports/rustup-installer.sha256
bash /scratch/rustup.sh -y --profile minimal --default-toolchain "$RUST_VERSION" --no-modify-path
export PATH="$CARGO_HOME/bin:$PATH"
rustc -Vv > /reports/rustc.txt
cargo --version > /reports/cargo.txt

cd /workspace
stage fetch
cargo fetch --locked --target x86_64-unknown-linux-gnu
stage compile
cargo build --locked --offline --release -p yyplayer-app --bin yyplayer
stage inspect
binary="$CARGO_TARGET_DIR/release/yyplayer"
test -s "$binary"
file_command=$(command -v file || true)
if [[ -n "$file_command" ]]; then "$file_command" "$binary" > /reports/elf.txt; fi
readelf --wide --dyn-syms "$binary" > /reports/symbols.txt
readelf --version-info "$binary" > /reports/versions.txt
ldd "$binary" > /reports/ldd.txt
sha256sum "$binary" > /reports/binary.sha256
if [[ "${KEEP_BINARY:-false}" == true ]]; then
    cp "$binary" /reports/yyplayer
fi
stage complete
