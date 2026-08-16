#!/usr/bin/env bash

# Shared Git-hook bridge. On Windows, PowerShell must stay attached to the
# calling terminal without creating a separate visible console window.

ELON_HOOK_POWERSHELL=""

elon_resolve_hook_powershell() {
  if [ -n "$ELON_HOOK_POWERSHELL" ]; then
    return 0
  fi
  if command -v powershell >/dev/null 2>&1; then
    ELON_HOOK_POWERSHELL="powershell"
    return 0
  fi
  if command -v pwsh >/dev/null 2>&1; then
    ELON_HOOK_POWERSHELL="pwsh"
    return 0
  fi
  return 1
}

elon_hook_runs_on_windows() {
  if [ "${OS:-}" = "Windows_NT" ]; then
    return 0
  fi
  case "$(uname -s 2>/dev/null || true)" in
    MINGW*|MSYS*|CYGWIN*) return 0 ;;
    *) return 1 ;;
  esac
}

elon_run_hook_powershell() {
  if ! elon_resolve_hook_powershell; then
    return 127
  fi

  local host_args=(-NoProfile -ExecutionPolicy Bypass)
  if elon_hook_runs_on_windows; then
    host_args=(-WindowStyle Hidden "${host_args[@]}")
  fi
  "$ELON_HOOK_POWERSHELL" "${host_args[@]}" "$@"
}
