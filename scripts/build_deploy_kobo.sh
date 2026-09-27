#!/usr/bin/env bash
set -euo pipefail

# Build and install E-Ink Chess on a mounted Kobo Libra H2O.
#
# The Cobalt checkout is intentionally kept outside this repository because it
# is a pinned fork dependency. This script prepares and prebuilds everything
# before it needs the Kobo, waits for the reader only when deployment is ready,
# installs over USB, verifies the installed chess binary, and leaves the reader
# mounted for fast development/test iterations.

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COBALT_DIR="${COBALT_DIR:-${APP_DIR}/../Cobalt}"
VOLUME="${KOBO_VOLUME:-/Volumes/KOBOeReader}"
PINNED_TOOLCHAIN="${RUST_TOOLCHAIN:-1.85.1}"

fail() {
    printf 'build/deploy failed: %s\n' "$1" >&2
    exit 1
}

command -v python3 >/dev/null || fail "python3 is required"
command -v rustup >/dev/null || fail "rustup is required"

[[ -f "${COBALT_DIR}/Cargo.toml" ]] || fail "Cobalt checkout not found at ${COBALT_DIR}"

if [[ "$(git -C "${COBALT_DIR}" rev-parse HEAD)" != "6737d128b3110a79a8b25216c58995b5ddc4b37c" ]]; then
    fail "Cobalt is not at the pinned revision 6737d128b3110a79a8b25216c58995b5ddc4b37c"
fi

printf '%s\n' 'Preparing pinned Cobalt with the current E-Ink Chess sources…'
python3 "${APP_DIR}/scripts/prepare_cobalt.py" "${COBALT_DIR}"

printf '%s\n' 'Running the chess and renderer tests…'
(
    cd "${COBALT_DIR}"
    PATH="$(dirname "$(rustup which cargo --toolchain "${PINNED_TOOLCHAIN}")"):${PATH}" \
        RUSTUP_TOOLCHAIN="${PINNED_TOOLCHAIN}" \
        cargo test --locked -p kobo-ui -p kobo-eink-chess
)

printf '%s\n' 'Installing the prepared Cobalt CLI…'
(
    cd "${COBALT_DIR}"
    PATH="$(dirname "$(rustup which cargo --toolchain "${PINNED_TOOLCHAIN}")"):${PATH}" \
        RUSTUP_TOOLCHAIN="${PINNED_TOOLCHAIN}" \
        cargo install --path crates/kobo-cli --force
)

KOBO_BIN="${KOBO_BIN:-${HOME}/.cargo/bin/kobo}"
[[ -x "${KOBO_BIN}" ]] || fail "prepared kobo CLI not found at ${KOBO_BIN}"

# Build the complete device package before requiring the Kobo to be mounted.
# `kobo setup --source` below performs the same package build internally, but
# Cargo will then hit the warm target directory instead of doing the expensive
# ARM compilation while the USB volume has to remain connected.
PREBUILT_PACKAGE="${COBALT_DIR}/target/eink-chess-KoboRoot.tgz"
printf '%s\n' 'Prebuilding the complete Kobo package before USB deployment…'
(
    cd "${COBALT_DIR}"
    PATH="$(dirname "$(rustup which cargo --toolchain "${PINNED_TOOLCHAIN}")"):${PATH}" \
        RUSTUP_TOOLCHAIN="${PINNED_TOOLCHAIN}" \
        "${KOBO_BIN}" package --out "${PREBUILT_PACKAGE}"
)
[[ -f "${PREBUILT_PACKAGE}" ]] || fail "prebuilt Kobo package was not produced"

printf 'Build complete. Connect the Kobo and tap Connect; waiting for %s…\n' "${VOLUME}"
WAIT_SECONDS="${KOBO_WAIT_SECONDS:-600}"
WAIT_DEADLINE=$((SECONDS + WAIT_SECONDS))
while [[ ! -f "${VOLUME}/.kobo/version" ]]; do
    if (( SECONDS >= WAIT_DEADLINE )); then
        fail "Kobo did not mount at ${VOLUME} within ${WAIT_SECONDS} seconds"
    fi
    sleep 2
done
printf '%s\n' 'Kobo mounted; starting the short USB deployment step…'

printf '%s\n' 'Installing the ARM package on the Kobo…'
(
    cd "${COBALT_DIR}"
    PATH="$(dirname "$(rustup which cargo --toolchain "${PINNED_TOOLCHAIN}")"):${PATH}" \
        RUSTUP_TOOLCHAIN="${PINNED_TOOLCHAIN}" \
        "${KOBO_BIN}" setup \
            --volume "${VOLUME}" \
            --source \
            --no-eject \
            --wait-for-reader \
            --no-sample \
            --yes
)

ARM_CHESS="${COBALT_DIR}/target/armv7-unknown-linux-musleabihf/release/kobo-eink-chess"
INSTALLED_CHESS="${VOLUME}/.adds/cobalt/bin/kobo-eink-chess"
MENU_FILE="${VOLUME}/.adds/nm/cobalt"
START_FILE="${VOLUME}/.adds/cobalt/start.sh"
[[ -f "${ARM_CHESS}" ]] || fail "ARM chess binary was not produced"
[[ -f "${INSTALLED_CHESS}" ]] || fail "ARM chess binary was not copied to the Kobo"
[[ -f "${MENU_FILE}" ]] || fail "NickelMenu entry was not written"
[[ -f "${START_FILE}" ]] || fail "Cobalt start script was not copied"

EXPECTED_HASH="$(shasum -a 256 "${ARM_CHESS}" | awk '{print $1}')"
INSTALLED_HASH="$(shasum -a 256 "${INSTALLED_CHESS}" | awk '{print $1}')"
[[ "${EXPECTED_HASH}" == "${INSTALLED_HASH}" ]] || fail "installed chess binary checksum does not match the build"
grep -q 'E-Ink Chess' "${MENU_FILE}" || fail "NickelMenu entry is not E-Ink Chess"
grep -q 'kobo-eink-chess' "${START_FILE}" || fail "Cobalt is not configured to launch E-Ink Chess"

printf 'Installed ARM chess binary (%s)\n' "${EXPECTED_HASH}"
printf '%s\n' 'E-Ink Chess menu and direct-launch checks passed.'
printf 'Kobo left mounted at %s for inspection/repeated deploys.\n' "${VOLUME}"
printf '%s\n' 'Eject it manually when you are ready to disconnect and test on-device.'
