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

unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
export NO_PROXY="*"
export no_proxy="*"

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

set_direct_git_ssh() {
  local origin_url
  origin_url="$(git remote get-url origin 2>/dev/null || true)"
  if [[ "$origin_url" =~ github\.com[:/] ]]; then
    export GIT_SSH_COMMAND="ssh -o ProxyCommand=none -o ProxyJump=none -o HostName=ssh.github.com -p 443"
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

write_ai_workflow_guard() {
  local edit_root="$1"
  local state="$2"

  echo "AI_WORKFLOW_GUARD_BEGIN"
  echo "EDIT_ROOT=$edit_root"
  echo "EDIT_STATE=$state"
  echo "RULE_MAIN_BASELINE=main checkout is sync-only; do not edit business files in main."
  echo "RULE_BEFORE_EDIT=cd to EDIT_ROOT/WORKTREE_PATH and run git status --short --branch before editing."
  echo "RULE_PUSH=after commit run git push origin HEAD:main, then run the CodePushed ancestor check for this platform."
  echo "RULE_FINISH=after push sync the main baseline with git pull --ff-only and run scripts/cleanup-task-worktrees.sh --apply."
  echo "AI_WORKFLOW_GUARD_END"
}

sync_local_main_baseline() {
  if ! git rev-parse --verify origin/main >/dev/null 2>&1; then
    echo "MAIN_BASELINE_SYNC=skipped_no_origin_main"
    return 0
  fi

  git worktree prune >/dev/null 2>&1 || true

  local main_path="" path="" branch_ref="" line key val
  while IFS= read -r line || [[ -n "$line" ]]; do
    if [[ -z "$line" ]]; then
      if [[ "$branch_ref" == "refs/heads/main" && -n "$path" ]]; then
        main_path="$path"
        break
      fi
      path=""
      branch_ref=""
      continue
    fi
    key="${line%% *}"
    val="${line#* }"
    case "$key" in
      worktree) path="$val" ;;
      branch) branch_ref="$val" ;;
    esac
  done < <(git worktree list --porcelain)
  if [[ "$branch_ref" == "refs/heads/main" && -n "$path" && -z "$main_path" ]]; then
    main_path="$path"
  fi

  if [[ -n "$main_path" && -e "$main_path/.git" ]]; then
    local status
    status="$(git -C "$main_path" status --porcelain=v1 --untracked-files=normal)"
    if [[ -n "$status" ]]; then
      echo "MAIN_BASELINE_SYNC=blocked_dirty:$main_path"
      return 0
    fi
    git -C "$main_path" merge --ff-only origin/main >/dev/null
    echo "MAIN_BASELINE_SYNC=synced_worktree:$main_path"
    return 0
  fi

  if git show-ref --verify --quiet refs/heads/main; then
    git branch --force main origin/main >/dev/null
    echo "MAIN_BASELINE_SYNC=synced_ref"
  else
    git branch main origin/main >/dev/null
    echo "MAIN_BASELINE_SYNC=created_ref"
  fi
}

branch="$(git branch --show-current || true)"
has_origin=0
if git remote get-url origin >/dev/null 2>&1; then
  has_origin=1
  set_direct_git_ssh
  git_fetch_with_retry
  sync_local_main_baseline
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
if [[ "$always_create_worktree" -eq 1 || "$dirty" -eq 1 || "$behind" -gt 0 || "$branch" == "main" ]]; then
  needs_worktree=1
fi

created_worktree=0
created_worktree_path=""
if [[ "$create_worktree" -eq 1 && "$needs_worktree" -eq 1 ]]; then
  if [[ "$has_origin" -ne 1 ]]; then
    echo "Cannot create isolated worktree: origin remote is missing" >&2
    exit 1
  fi

  stamp="$(date +%Y%m%d-%H%M%S)"
  safe_prefix="${branch_prefix%/}"
  if command -v uuidgen >/dev/null 2>&1; then
    short_id="$(uuidgen | tr -d '-' | cut -c1-8)"
  else
    short_id="$(printf '%s%s' "$$" "$RANDOM" | cut -c1-8)"
  fi
  unique_suffix="$$-$short_id"
  new_branch="$safe_prefix-$stamp-$unique_suffix"
  if [[ -z "$worktree_parent" ]]; then
    worktree_parent="$(dirname "$repo_root")"
  fi
  leaf="$(basename "$repo_root")-task-$stamp-$unique_suffix"
  worktree_path="$worktree_parent/$leaf"

  git worktree add -b "$new_branch" "$worktree_path" origin/main
  echo "WORKTREE_CREATED=true"
  echo "WORKTREE_BRANCH=$new_branch"
  echo "WORKTREE_PATH=$worktree_path"
  echo "WORKTREE_BASE=$(git rev-parse --short origin/main)"
  echo "NEXT=cd \"$worktree_path\""
  write_ai_workflow_guard "$worktree_path" "created_worktree"
  created_worktree=1
  created_worktree_path="$worktree_path"
elif [[ "$needs_worktree" -eq 1 ]]; then
  echo "WORKTREE_CREATED=false"
  echo "NEXT=Run bash scripts/ai-task-preflight.sh --create-worktree before editing."
  write_ai_workflow_guard "BLOCKED_CREATE_WORKTREE_FIRST" "blocked_needs_worktree"
else
  echo "WORKTREE_CREATED=false"
  echo "NEXT=Workspace is already isolated and current enough for direct edits."
  write_ai_workflow_guard "$repo_root" "current_worktree_ok"
fi

# 自动清理已合并、工作树干净的孤儿 task worktree。要禁用：--skip-auto-cleanup
if [[ "$skip_auto_cleanup" -ne 1 && -x "$repo_root/scripts/cleanup-task-worktrees.sh" ]]; then
  cleanup_args=(--apply)
  if [[ "$created_worktree" -eq 1 && -n "$created_worktree_path" ]]; then
    cleanup_args+=(--exclude-path "$created_worktree_path")
  fi
  cleanup_out="$(bash "$repo_root/scripts/cleanup-task-worktrees.sh" "${cleanup_args[@]}" 2>&1 || true)"
  removed_line="$(printf '%s\n' "$cleanup_out" | grep -E '^完成：清理' | tail -n 1 || true)"
  if [[ -n "$removed_line" ]]; then
    echo "AUTO_CLEANUP=$removed_line"
  else
    echo "AUTO_CLEANUP=skipped"
  fi
fi
