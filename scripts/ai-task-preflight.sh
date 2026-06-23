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

git_fetch_hint() {
  local output="${1:-}"
  if [[ "$output" =~ (Could\ not\ resolve\ host|Name\ or\ service\ not\ known|Temporary\ failure\ in\ name\ resolution) ]]; then
    printf '%s\n' "网络/DNS 无法解析 GitHub，请检查网络、DNS 或代理后重试。"
  elif [[ "$output" =~ (Failed\ to\ connect|Connection\ timed\ out|Connection\ reset|Connection\ refused|Operation\ timed\ out|HTTP/2\ stream|early\ EOF|The\ remote\ end\ hung\ up\ unexpectedly) ]]; then
    printf '%s\n' "网络连接到 GitHub 不稳定或超时，通常是临时抖动；脚本已短重试但仍失败。"
  elif [[ "$output" =~ (Permission\ denied|Authentication\ failed|Repository\ not\ found|Could\ not\ read\ from\ remote\ repository|Host\ key\ verification\ failed|publickey) ]]; then
    printf '%s\n' "Git 远端认证或仓库权限异常，请检查 SSH key、GitHub 权限和 origin 地址。"
  else
    printf '%s\n' "Git fetch 失败，原因未能自动分类；请查看原始输出。"
  fi
}

git_fetch_with_retry() {
  local attempts=3 delay=2 i output hint
  for ((i=1; i<=attempts; i++)); do
    if output=$(git fetch origin 2>&1); then
      if [[ "$i" -gt 1 ]]; then
        echo "GIT_FETCH_RETRY=success_after_$i"
      fi
      return 0
    fi
    hint="$(git_fetch_hint "$output")"
    echo "GIT_FETCH_RETRY=attempt_$i/$attempts failed: $hint" >&2
    if [[ "$i" -lt "$attempts" ]]; then
      sleep "$delay"
    fi
  done
  hint="$(git_fetch_hint "$output")"
  echo "git fetch origin 连续失败 $attempts 次。$hint" >&2
  echo "原始输出：$output" >&2
  return 1
}

branch="$(git branch --show-current || true)"
has_origin=0
if git remote get-url origin >/dev/null 2>&1; then
  has_origin=1
  git_fetch_with_retry
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

created_worktree=0
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
  created_worktree=1
elif [[ "$needs_worktree" -eq 1 ]]; then
  echo "WORKTREE_CREATED=false"
  echo "NEXT=Run bash scripts/ai-task-preflight.sh --create-worktree before editing."
else
  echo "WORKTREE_CREATED=false"
  echo "NEXT=Workspace is clean and current enough for direct edits."
fi

# 自动清理已合并、工作树干净的孤儿 task worktree。要禁用：--skip-auto-cleanup
if [[ "$created_worktree" -eq 1 && "$skip_auto_cleanup" -ne 1 ]]; then
  echo "AUTO_CLEANUP=skipped_after_worktree_create"
elif [[ "$skip_auto_cleanup" -ne 1 && -x "$repo_root/scripts/cleanup-task-worktrees.sh" ]]; then
  cleanup_out="$(bash "$repo_root/scripts/cleanup-task-worktrees.sh" --apply 2>&1 || true)"
  removed_line="$(printf '%s\n' "$cleanup_out" | grep -E '^完成：清理' | tail -n 1 || true)"
  if [[ -n "$removed_line" ]]; then
    echo "AUTO_CLEANUP=$removed_line"
  else
    echo "AUTO_CLEANUP=skipped"
  fi
fi
