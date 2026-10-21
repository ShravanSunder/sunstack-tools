#!/usr/bin/env bash
# Stable Developer ID signatures let LuLu retain debug approvals across builds
# and worktrees. Signing failure must never prevent running the executable.
set -euo pipefail

# Select the local signing team without committing account metadata. Multiple
# local teams require an explicit command-scoped choice; never pick arbitrarily.
developer_id_team() {
  if [[ -n "${SUNSTACK_DEBUG_SIGNING_TEAM:-}" ]]; then
    printf '%s\n' "${SUNSTACK_DEBUG_SIGNING_TEAM}"
    return
  fi
  security find-identity -v -p codesigning 2>/dev/null |
    awk '/Developer ID Application/ {
      sub(/.*\(/, ""); sub(/\).*/, ""); teams[$0] = 1
    } END {
      for (team in teams) { selected = team; count++ }
      if (count == 1) print selected
    }'
}
readonly DEVELOPER_ID_TEAM="$(developer_id_team || true)"

debug_identifier_for() {
  case "$(basename "$1")" in
    sunstack) echo "dev.shravansunder.sunstack-rule.debug" ;;
    *) echo "dev.shravansunder.sunstack-tools-tests.debug" ;;
  esac
}

warn_unsigned() {
  echo "cargo debug runner: $(basename "$1") runs unsigned ($2); LuLu may prompt after rebuilding" >&2
}

has_debug_identity() {
  local signature
  signature="$(codesign -dvv "$1" 2>&1)" || return 1
  grep -qx "Identifier=$2" <<<"${signature}" &&
    grep -qx "TeamIdentifier=${DEVELOPER_ID_TEAM}" <<<"${signature}" &&
    codesign --verify "$1" 2>/dev/null
}

sign_with_debug_identity() (
  local executable="$1"
  local debug_identifier="$2"
  local signing_identity temporary_file="" lock_acquired=false
  local lock_directory="${executable}.signing.lock"

  cleanup_signing() {
    if [[ -n "${temporary_file}" ]]; then
      rm -f "${temporary_file}" 2>/dev/null || true
    fi
    if [[ "${lock_acquired}" == true ]]; then
      rmdir "${lock_directory}" 2>/dev/null || true
    fi
  }
  trap cleanup_signing EXIT

  if [[ -z "${DEVELOPER_ID_TEAM}" ]]; then
    warn_unsigned "${executable}" "configure SUNSTACK_DEBUG_SIGNING_TEAM when local signing teams are ambiguous or unavailable"
    return 0
  fi
  if has_debug_identity "${executable}" "${debug_identifier}"; then
    return 0
  fi

  for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
    if mkdir "${lock_directory}" 2>/dev/null; then
      lock_acquired=true
      break
    fi
    if has_debug_identity "${executable}" "${debug_identifier}"; then
      return 0
    fi
    sleep 0.1
  done
  if [[ "${lock_acquired}" != true ]]; then
    warn_unsigned "${executable}" "signing lock unavailable"
    return 0
  fi
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

  temporary_file="$(mktemp "${executable}.signing.XXXXXX")" || {
    warn_unsigned "${executable}" "temporary copy unavailable"
    return 0
  }
  if ! cp -p "${executable}" "${temporary_file}"; then
    warn_unsigned "${executable}" "executable copy failed"
    return 0
  fi
  if ! codesign --force --sign "${signing_identity}" --identifier "${debug_identifier}" --timestamp=none "${temporary_file}" >/dev/null 2>&1; then
    warn_unsigned "${executable}" "codesign failed; original unchanged"
    return 0
  fi
  if ! has_debug_identity "${temporary_file}" "${debug_identifier}"; then
    warn_unsigned "${executable}" "signature verification failed; original unchanged"
    return 0
  fi
  # Rename in the same directory is atomic. Concurrent execs see either the
  # untouched linked inode or the fully signed inode, never a partial rewrite.
  if ! mv -f "${temporary_file}" "${executable}"; then
    warn_unsigned "${executable}" "signed replacement failed; original unchanged"
    return 0
  fi
  temporary_file=""
)

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
