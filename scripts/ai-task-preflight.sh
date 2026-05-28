#!/usr/bin/env bash
set -euo pipefail

create_worktree=0
always_create_worktree=0
branch_prefix="codex/task"
worktree_parent=""
skip_auto_cleanup=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --create-worktree)
      create_worktree=1
      shift
      ;;
    --always-create-worktree)
      always_create_worktree=1
      create_worktree=1
      shift
      ;;
    --branch-prefix)
      branch_prefix="${2:?missing value for --branch-prefix}"
      shift 2
      ;;
    --worktree-parent)
      worktree_parent="${2:?missing value for --worktree-parent}"
      shift 2
      ;;
    --skip-auto-cleanup)
      skip_auto_cleanup=1
      shift
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

branch="$(git branch --show-current || true)"
has_origin=0
if git remote get-url origin >/dev/null 2>&1; then
  has_origin=1
  git fetch origin
fi

status_short="$(git status --short)"
dirty=0
if [[ -n "$status_short" ]]; then
  dirty=1
fi

ahead=0
behind=0
if [[ "$has_origin" -eq 1 ]] && git rev-parse --verify origin/main >/dev/null 2>&1; then
  read -r ahead behind < <(git rev-list --left-right --count HEAD...origin/main)
fi

echo "REPO_ROOT=$repo_root"
echo "BRANCH=$branch"
echo "DIRTY=$([[ "$dirty" -eq 1 ]] && echo true || echo false)"
echo "AHEAD=$ahead"
echo "BEHIND=$behind"

if [[ "$dirty" -eq 1 ]]; then
  echo "Changed files:"
  printf '%s\n' "$status_short" | sed 's/^/  /'
fi

needs_worktree=0
if [[ "$always_create_worktree" -eq 1 || "$dirty" -eq 1 || "$behind" -gt 0 ]]; then
  needs_worktree=1
fi

if [[ "$create_worktree" -eq 1 && "$needs_worktree" -eq 1 ]]; then
  if [[ "$has_origin" -ne 1 ]]; then
    echo "Cannot create isolated worktree: origin remote is missing" >&2
    exit 1
  fi

  stamp="$(date +%Y%m%d-%H%M%S)"
  safe_prefix="${branch_prefix%/}"
  new_branch="$safe_prefix-$stamp"
  if [[ -z "$worktree_parent" ]]; then
    worktree_parent="$(dirname "$repo_root")"
  fi
  leaf="$(basename "$repo_root")-task-$stamp"
  worktree_path="$worktree_parent/$leaf"

  git worktree add -b "$new_branch" "$worktree_path" origin/main
  echo "WORKTREE_CREATED=true"
  echo "WORKTREE_BRANCH=$new_branch"
  echo "WORKTREE_PATH=$worktree_path"
  echo "NEXT=cd \"$worktree_path\""
elif [[ "$needs_worktree" -eq 1 ]]; then
  echo "WORKTREE_CREATED=false"
  echo "NEXT=Run bash scripts/ai-task-preflight.sh --create-worktree before editing."
else
  echo "WORKTREE_CREATED=false"
  echo "NEXT=Workspace is clean and current enough for direct edits."
fi

# 自动清理已合并、工作树干净的孤儿 task worktree。要禁用：--skip-auto-cleanup
if [[ "$skip_auto_cleanup" -ne 1 && -x "$repo_root/scripts/cleanup-task-worktrees.sh" ]]; then
  cleanup_out="$(bash "$repo_root/scripts/cleanup-task-worktrees.sh" --apply 2>&1 || true)"
  removed_line="$(printf '%s\n' "$cleanup_out" | grep -E '^完成：清理' | tail -n 1 || true)"
  if [[ -n "$removed_line" ]]; then
    echo "AUTO_CLEANUP=$removed_line"
  else
    echo "AUTO_CLEANUP=skipped"
  fi
fi
