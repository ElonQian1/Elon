#!/usr/bin/env bash

invoke_elon_apk_worktree_cleanup() {
  local repo_root="$1" cleanup_out removed_line
  [[ -x "$repo_root/scripts/cleanup-task-worktrees.sh" ]] || return 0
  cleanup_out="$(bash "$repo_root/scripts/cleanup-task-worktrees.sh" --apply 2>&1 || true)"
  removed_line="$(printf '%s\n' "$cleanup_out" | grep -E '^完成：清理' | tail -n 1 || true)"
  [[ -n "$removed_line" ]] && echo "   $removed_line（自动）"
}
