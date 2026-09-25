#!/usr/bin/env bash
set -euo pipefail

# Build and install E-Ink Chess on a mounted Kobo Libra H2O.
#
# The Cobalt checkout is intentionally kept outside this repository because it
# is a pinned fork dependency. This script prepares it from the app source,
# builds the ARM package, installs it over USB, verifies the installed chess
# binary, and ejects the reader only after every check succeeds.

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
command -v diskutil >/dev/null || fail "diskutil is required on macOS"

[[ -d "${VOLUME}" ]] || fail "Kobo volume is not mounted at ${VOLUME}"
[[ -f "${VOLUME}/.kobo/version" ]] || fail "${VOLUME} is not a Kobo volume"
[[ -f "${COBALT_DIR}/Cargo.toml" ]] || fail "Cobalt checkout not found at ${COBALT_DIR}"

if [[ "$(git -C "${COBALT_DIR}" rev-parse HEAD)" != "b0c1160d87fe438f297e21a917867baad6316b55" ]]; then
    fail "Cobalt is not at the pinned revision b0c1160d87fe438f297e21a917867baad6316b55"
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

printf '%s\n' 'Building and installing the ARM package on the Kobo…'
(
    cd "${COBALT_DIR}"
    PATH="$(dirname "$(rustup which cargo --toolchain "${PINNED_TOOLCHAIN}")"):${PATH}" \
        RUSTUP_TOOLCHAIN="${PINNED_TOOLCHAIN}" \
        "${KOBO_BIN}" setup \
            --volume "${VOLUME}" \
            --source \
            --no-eject \
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
diskutil eject "${VOLUME}"
printf '%s\n' 'Kobo safely ejected. Disconnect USB and restart the reader.'
