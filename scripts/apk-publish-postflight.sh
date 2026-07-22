#!/usr/bin/env bash

invoke_elon_apk_worktree_cleanup() {
  local repo_root="$1" cleanup_out removed_line
  [[ -x "$repo_root/scripts/cleanup-task-worktrees.sh" ]] || return 0
  cleanup_out="$(bash "$repo_root/scripts/cleanup-task-worktrees.sh" --apply 2>&1 || true)"
  removed_line="$(printf '%s\n' "$cleanup_out" | grep -E '^完成：清理' | tail -n 1 || true)"
  [[ -n "$removed_line" ]] && echo "   $removed_line（自动）"
}

invoke_elon_apk_publish_postflight() {
  local repo_root="$1" apk_path="$2" expected_version_code="$3"
  invoke_elon_apk_worktree_cleanup "$repo_root"
  . "$SCRIPT_DIR/apk-adb-autodeploy.sh"
  invoke_elon_apk_adb_autodeploy "$apk_path" "$expected_version_code"
}
