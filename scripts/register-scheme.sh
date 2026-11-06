#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# SZZ Launcher - Local Deep Link Scheme Handler Helper (Linux xdg-open)
#
# Usage:
#   ./scripts/register-scheme.sh register    Register local debug binary for szz://
#   ./scripts/register-scheme.sh unregister  Remove local registration
#   ./scripts/register-scheme.sh status      Query current default x-scheme-handler/szz
#   ./scripts/register-scheme.sh test [url]  Test triggering deep link via xdg-open
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN_PATH="${REPO_ROOT}/target/debug/szz-desktop"
APPS_DIR="${HOME}/.local/share/applications"
DESKTOP_FILE="${APPS_DIR}/szz.desktop"

COMMAND="${1:-status}"

case "${COMMAND}" in
  register)
    echo "[szz] Registering local debug launcher for szz:// custom protocol..."
    if [[ ! -f "${BIN_PATH}" ]]; then
      echo "[szz] Binary not found at ${BIN_PATH}. Building debug binary first..."
      cargo build --bin szz-desktop
    fi

    mkdir -p "${APPS_DIR}"
    cat << EOF > "${DESKTOP_FILE}"
[Desktop Entry]
Name=SZZ Launcher (Dev)
Comment=Lightweight Minecraft Launcher & Platform
Exec="${BIN_PATH}" %u
Type=Application
Terminal=false
MimeType=x-scheme-handler/szz;
Categories=Game;
StartupNotify=true
EOF

    chmod +x "${DESKTOP_FILE}"
    xdg-mime default szz.desktop x-scheme-handler/szz

    if command -v update-desktop-database >/dev/null 2>&1; then
      update-desktop-database "${APPS_DIR}" >/dev/null 2>&1 || true
    fi

    echo "[SUCCESS] Successfully registered szz.desktop as the handler for 'szz://'."
    echo "  Handler: ${DESKTOP_FILE}"
    echo "  Binary:  ${BIN_PATH}"
    ;;

  unregister)
    echo "[szz] Unregistering local szz:// scheme handler..."
    if [[ -f "${DESKTOP_FILE}" ]]; then
      rm -f "${DESKTOP_FILE}"
      echo "[szz] Removed ${DESKTOP_FILE}"
    fi

    if command -v update-desktop-database >/dev/null 2>&1; then
      update-desktop-database "${APPS_DIR}" >/dev/null 2>&1 || true
    fi
    echo "[SUCCESS] Unregistered szz:// scheme handler."
    ;;

  status)
    CURRENT="$(xdg-mime query default x-scheme-handler/szz 2>/dev/null || echo '(none)')"
    echo "[szz] Current handler for 'x-scheme-handler/szz': ${CURRENT}"
    if [[ -f "${DESKTOP_FILE}" ]]; then
      echo "[szz] Dev desktop entry exists: ${DESKTOP_FILE}"
    else
      echo "[szz] Dev desktop entry does not exist in ${APPS_DIR}"
    fi
    ;;

  test)
    TEST_URL="${2:-szz://launch/cobblemon-1-21-1}"
    echo "[szz] Testing deep-link trigger with URL: ${TEST_URL}"
    xdg-open "${TEST_URL}"
    ;;

  *)
    echo "Usage: $0 {register|unregister|status|test [url]}"
    exit 1
    ;;
esac
