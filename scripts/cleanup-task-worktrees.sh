#!/usr/bin/env bash
# 清理 ai-task-preflight 留下的孤儿 worktree
# 与 cleanup-task-worktrees.ps1 行为等价。
#
# 用法：
#   bash scripts/cleanup-task-worktrees.sh                # 预览
#   bash scripts/cleanup-task-worktrees.sh --apply        # 执行
#   bash scripts/cleanup-task-worktrees.sh --apply --force --delete-remote
#   bash scripts/cleanup-task-worktrees.sh --keep-last 3

set -euo pipefail

APPLY=0
FORCE=0
KEEP_LAST=0
DELETE_REMOTE=0
EXCLUDE_PATHS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply) APPLY=1; shift ;;
    --force) FORCE=1; shift ;;
    --delete-remote) DELETE_REMOTE=1; shift ;;
    --keep-last=*) KEEP_LAST="${1#*=}"; shift ;;
    --keep-last) KEEP_LAST="${2:-0}"; shift 2 ;;
    --exclude-path) EXCLUDE_PATHS+=("${2:?missing value for --exclude-path}"); shift 2 ;;
    -h|--help)
      sed -n '1,15p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
repo_leaf="$(basename "$repo_root")"
current_wt="$(pwd -P)"
declare -A exclude_set
for exclude_path in "${EXCLUDE_PATHS[@]}"; do
  if [[ -n "$exclude_path" ]]; then
    exclude_set["${exclude_path%/}"]=1
    if resolved_parent="$(cd "$(dirname "$exclude_path")" 2>/dev/null && pwd -P)"; then
      exclude_set["$resolved_parent/$(basename "$exclude_path")"]=1
    fi
  fi
done

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

git_fetch_with_retry >/dev/null

# 解析 worktree（采集任务命名和 codex/* 分支）
mapfile -t lines < <(git worktree list --porcelain)
declare -a wt_paths wt_branches
path=""; branch=""
flush() {
  if [[ -n "$path" ]]; then
    leaf="$(basename "$path")"
    if [[ "$leaf" =~ ^${repo_leaf}-task-[0-9]{8}-[0-9]{6}(-[A-Za-z0-9]+(-[A-Fa-f0-9]+)?)?(-task-[0-9]{8}-[0-9]{6}(-[A-Za-z0-9]+(-[A-Fa-f0-9]+)?)?)?$ || "$branch" == codex/* ]]; then
      wt_paths+=("$path"); wt_branches+=("$branch")
    fi
  fi
  path=""; branch=""
}
for line in "${lines[@]}"; do
  if [[ -z "$line" ]]; then flush; continue; fi
  key="${line%% *}"; val="${line#* }"
  case "$key" in
    worktree) path="$val" ;;
    branch)   branch="${val#refs/heads/}" ;;
  esac
done
flush

# 排序后保留最近 N 个
if [[ "$KEEP_LAST" -gt 0 && "${#wt_paths[@]}" -gt "$KEEP_LAST" ]]; then
  # 排序：把 (path,branch) 配对，按 path 倒序，取前 N
  mapfile -t sorted < <(for i in "${!wt_paths[@]}"; do printf '%s\t%s\n' "${wt_paths[$i]}" "${wt_branches[$i]}"; done | sort -r)
  declare -A keep_set
  for ((i=0;i<KEEP_LAST && i<${#sorted[@]};i++)); do
    keep_set["${sorted[$i]%%	*}"]=1
  done
else
  declare -A keep_set
fi

to_remove_paths=(); to_remove_branches=()
kept_lines=()

for i in "${!wt_paths[@]}"; do
  wt="${wt_paths[$i]}"; br="${wt_branches[$i]}"
  reasons=()

  [[ "${wt%/}" == "${current_wt%/}" ]] && reasons+=("当前正在使用")
  [[ -n "${keep_set[$wt]:-}" ]] && reasons+=("在 --keep-last 保留范围内")
  [[ -n "${exclude_set[$wt]:-}" ]] && reasons+=("在 --exclude-path 保护范围内")

  if [[ ! -d "$wt" ]]; then
    reasons+=("目录已不存在（可 prune）")
  else
    if ! st=$(git -C "$wt" status --short 2>&1); then
      reasons+=("git status 失败: $st")
    elif [[ -n "$st" ]]; then
      reasons+=("有未提交/未跟踪改动")
    fi
    if [[ "$FORCE" -eq 0 && -n "$br" ]]; then
      if ! git merge-base --is-ancestor "$br" origin/main >/dev/null 2>&1; then
        reasons+=("分支 $br 尚未合并进 origin/main（用 --force 跳过）")
      fi
    fi
  fi

  if [[ "${#reasons[@]}" -eq 0 ]]; then
    to_remove_paths+=("$wt"); to_remove_branches+=("$br")
  else
    kept_lines+=("$wt ($br)")
    for r in "${reasons[@]}"; do kept_lines+=("    - $r"); done
  fi
done

echo "=== 扫描结果 ==="
echo "可清理: ${#to_remove_paths[@]} 个"
echo "保留:   $(( ${#wt_paths[@]} - ${#to_remove_paths[@]} )) 个"
echo

if [[ "${#to_remove_paths[@]}" -gt 0 ]]; then
  echo "[将被删除]"
  for i in "${!to_remove_paths[@]}"; do
    echo "  ${to_remove_paths[$i]}  (${to_remove_branches[$i]})"
  done
  echo
fi

if [[ "${#kept_lines[@]}" -gt 0 ]]; then
  echo "[保留]"
  printf '  %s\n' "${kept_lines[@]}"
  echo
fi

if [[ "$APPLY" -eq 0 ]]; then
  echo "预览模式。如要执行，请加 --apply。"
  exit 0
fi

if [[ "${#to_remove_paths[@]}" -eq 0 ]]; then
  echo "无需清理。"
  git worktree prune
  exit 0
fi

removed=0; failed=0
for i in "${!to_remove_paths[@]}"; do
  wt="${to_remove_paths[$i]}"; br="${to_remove_branches[$i]}"
  echo "removing $wt"
  if git worktree remove --force "$wt"; then
    if [[ -n "$br" ]]; then
      if [[ "$FORCE" -eq 1 ]]; then
        git branch -D "$br" || true
      else
        git branch -d "$br" || true
      fi
      [[ "$DELETE_REMOTE" -eq 1 ]] && git push origin --delete "$br" || true
    fi
    removed=$((removed+1))
  else
    echo "  失败"; failed=$((failed+1))
  fi
done

git worktree prune
echo
echo "完成：清理 $removed 个，失败 $failed 个。"
