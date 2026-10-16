#!/usr/bin/env bash
# Stable Developer ID signatures let LuLu retain debug approvals across builds
# and worktrees. Signing failure must never prevent running the executable.
set -euo pipefail

readonly DEVELOPER_ID_TEAM="974QD84WVC"

debug_identifier_for() {
  case "$(basename "$1")" in
    sunstack-rule) echo "dev.shravansunder.sunstack-rule.debug" ;;
    *) echo "dev.shravansunder.sunstack-tools-tests.debug" ;;
  esac
}

warn_unsigned() {
  echo "cargo debug runner: $(basename "$1") runs unsigned ($2); LuLu may prompt after rebuilding" >&2
}

has_debug_identity() {
  local signature
  signature="$(codesign -dv "$1" 2>&1)" || return 1
  grep -qx "Identifier=$2" <<<"${signature}" &&
    codesign --verify "$1" 2>/dev/null
}

sign_with_debug_identity() {
  local executable="$1"
  local debug_identifier="$2"
  local signing_identity

  if has_debug_identity "${executable}" "${debug_identifier}"; then
    return 0
  fi

  signing_identity="$(
    security find-identity -v -p codesigning |
      awk -v team="(${DEVELOPER_ID_TEAM})\"" 'index($0, "Developer ID Application") && index($0, team) { print $2; exit }'
  )" || signing_identity=""
  if [[ -z "${signing_identity}" ]]; then
    warn_unsigned "${executable}" "Developer ID identity unavailable"
    return 0
  fi

  if codesign --force --sign "${signing_identity}" --identifier "${debug_identifier}" --timestamp=none "${executable}" >/dev/null 2>&1; then
    return 0
  fi
  # Concurrent runs may race on codesign's temporary file. Wait for the
  # winning signature before falling back to the original executable.
  for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if has_debug_identity "${executable}" "${debug_identifier}"; then
      return 0
    fi
    sleep 0.1
  done
  warn_unsigned "${executable}" "codesign failed"
}

if [[ "${1:-}" == "--sign-only" ]]; then
  shift
  for executable in "$@"; do
    sign_with_debug_identity "${executable}" "$(debug_identifier_for "${executable}")"
  done
  exit 0
fi

if [[ $# -eq 0 ]]; then
  echo "usage: cargo-debug-signing-runner.sh <executable> [arguments...]" >&2
  exit 64
fi
executable="$1"
shift
sign_with_debug_identity "${executable}" "$(debug_identifier_for "${executable}")"
exec "${executable}" "$@"
