#!/usr/bin/env bash
# ================================================================
#  elon cli 服务端 — 本地交叉编译 → 部署 (Linux/macOS 版)
#  等效于 publish-server.ps1，支持 Ubuntu / macOS 开发机
#
#  依赖（首次运行前安装一次）：
#    1. zig:           https://ziglang.org/download/ 或 snap install zig --classic
#    2. cargo-zigbuild: cargo install cargo-zigbuild
#    3. musl target:   rustup target add x86_64-unknown-linux-musl
#    4. ssh + scp（系统自带）
#
#  用法：
#    ./scripts/publish-server.sh                 # 正常流程
#    ./scripts/publish-server.sh --skip-build    # 跳过编译，仅重试已有产物；产物版本必须已匹配本次 claim
#    ./scripts/publish-server.sh --skip-upload   # 只本地编译，不部署
#    ./scripts/publish-server.sh --force         # 强制部署，即使服务器已有更新版本
#
#  机器级 server-musl 缓存目录可在仓库根 .env.local 中设置：
#    RUST_SERVER_MUSL_TARGET_DIR=/var/tmp/server-musl-target
#  兼容旧名 RUST_MUSL_TARGET_DIR。
#  旧的 ELON_BUILD_TARGET_DIR 仍兼容，脚本会在其下创建 elon-server-musl/
#
#  发布构建会强制使用 CARGO_ENCODED_RUSTFLAGS="-C target-cpu=x86-64"，
#  防止全局 Cargo config 中的 target-cpu=native 污染服务器产物。
#
#  发布顺序：
#    1. fetch origin/main 并只做 fast-forward，同步到远端最新主线
#    2. 调 /api/release/claim 原子申请版本号
#    3. 用 ELON_BUILD_VERSION 注入版本号后编译
#    4. 上传/本机 staging + flock/CAS 部署
#    5. 调 /api/release/finish 登记成功或释放失败槽位
# ================================================================
set -euo pipefail

# ── ANSI 颜色 ──────────────────────────────────────────────────
RED='\033[0;31m'; YELLOW='\033[1;33m'; GREEN='\033[0;32m'
CYAN='\033[0;36m'; GRAY='\033[0;37m'; NC='\033[0m'

# ── 参数解析 ──────────────────────────────────────────────────
SKIP_BUILD=0; SKIP_UPLOAD=0; FORCE=0
for arg in "$@"; do
  case "$arg" in
    --skip-build)   SKIP_BUILD=1 ;;
    --skip-upload)  SKIP_UPLOAD=1 ;;
    --force)        FORCE=1 ;;
    *) echo -e "${RED}未知参数: $arg${NC}" >&2; exit 1 ;;
  esac
done

# ── 配置 ───────────────────────────────────────────────────────
TARGET="x86_64-unknown-linux-musl"
SERVER="root@43.139.149.158"
REMOTE_DIR="/root/Elon"
REMOTE_BIN="$REMOTE_DIR/server/target/release/elon-server"
SSH_OPTS="-o ProxyCommand=none"
PUBLIC_SERVER_HTTP="http://43.139.149.158:8080"
LOCAL_SERVER_HTTP="http://127.0.0.1:8080"
RELEASE_TOKEN=""
RELEASE_FINISHED=0

is_local_server_deploy() {
  case "${ELON_DEPLOY_LOCAL:-auto}" in
    1|true|TRUE|local|LOCAL) return 0 ;;
    0|false|FALSE|remote|REMOTE) return 1 ;;
  esac
  [ -d "$REMOTE_DIR" ] && [ -d "$REMOTE_DIR/server" ] && [ -w "$REMOTE_DIR" ] && command -v systemctl >/dev/null 2>&1
}

json_field() {
  local json="$1"
  local field="$2"
  JSON_INPUT="$json" python3 - "$field" <<'PY'
import json
import os
import sys

try:
    data = json.loads(os.environ.get("JSON_INPUT", ""))
except Exception:
    print("")
    sys.exit(0)

value = data
for part in sys.argv[1].split("."):
    if isinstance(value, dict):
        value = value.get(part)
    else:
        value = None
        break
if value is None:
    print("")
elif isinstance(value, bool):
    print("true" if value else "false")
else:
    print(value)
PY
}

release_post() {
  local endpoint="$1"
  local payload="$2"
  local url="${RELEASE_API_BASE:?release api base not set}/$endpoint"
  curl --noproxy '*' -sS --fail --max-time 30 \
    -H 'Content-Type: application/json' \
    -X POST \
    -d "$payload" \
    "$url"
}

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
    if output=$(git -C "$REPO_ROOT" fetch origin main 2>&1); then
      if [[ "$i" -gt 1 ]]; then
        echo -e "${GREEN}   ✅ git fetch 重试成功（第 $i 次）${NC}"
      fi
      return 0
    fi
    hint="$(git_fetch_hint "$output")"
    echo -e "${YELLOW}   ⚠️  git fetch 失败（第 $i/$attempts 次）：$hint${NC}" >&2
    if [[ "$i" -lt "$attempts" ]]; then
      sleep "$delay"
    fi
  done
  hint="$(git_fetch_hint "$output")"
  echo "CODE_SYNC_STATUS=unknown_fetch_failed"
  echo "SERVER_RELEASE_STATUS=not_attempted"
  echo "APK_RELEASE_STATUS=not_attempted"
  echo -e "${RED}❌ 后端发布未开始：git fetch origin main 连续失败 $attempts 次。$hint${NC}" >&2
  echo -e "${YELLOW}   原始输出：$output${NC}" >&2
  return 1
}

print_publish_status() {
  local server_status="$1"
  local code_status="${2:-synced}"
  local apk_status="${3:-not_attempted}"
  local message="${4:-}"
  [[ -n "$message" ]] && echo -e "${CYAN}   $message${NC}"
  echo -e "${GRAY}   CODE_SYNC_STATUS=$code_status${NC}"
  echo -e "${GRAY}   SERVER_RELEASE_STATUS=$server_status${NC}"
  echo -e "${GRAY}   APK_RELEASE_STATUS=$apk_status${NC}"
}

complete_release() {
  local success="$1"
  local version_name="${2:-}"
  local sha="${3:-}"
  local error_message="${4:-}"

  [ -n "$RELEASE_TOKEN" ] || return 0
  [ "$RELEASE_FINISHED" -eq 0 ] || return 0

  local payload
  payload=$(python3 - "$RELEASE_TOKEN" "$success" "$version_name" "$sha" "$error_message" <<'PY'
import json
import sys

token, success, version_name, sha, error_message = sys.argv[1:6]
ok = success.lower() == "true"
payload = {
    "kind": "server",
    "token": token,
    "success": ok,
}
if ok:
    if version_name:
        payload["versionName"] = version_name
    if sha:
        payload["sha"] = sha
elif error_message:
    payload["errorMessage"] = error_message
print(json.dumps(payload, separators=(",", ":")))
PY
)

  set +e
  release_post finish "$payload" >/dev/null
  local rc=$?
  set -e
  RELEASE_FINISHED=1
  if [ "$rc" -ne 0 ]; then
    echo -e "${YELLOW}   ⚠️  release/finish 调用失败（不影响脚本退出）：exit=$rc${NC}" >&2
  fi
}

read_live_server_version_name() {
  curl --noproxy '*' -s --max-time 5 "${SERVER_HTTP_BASE:-$PUBLIC_SERVER_HTTP}/api/server/version" 2>/dev/null | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('versionName',''))" 2>/dev/null || true
}

version_gt() {
  python3 - "$1" "$2" <<'PY'
import sys

def key(value: str):
    parts = []
    for part in value.split("."):
        try:
            parts.append(int(part))
        except ValueError:
            parts.append(0)
    while len(parts) < 3:
        parts.append(0)
    return parts[:3]

sys.exit(0 if key(sys.argv[1]) > key(sys.argv[2]) else 1)
PY
}

resolve_server_version_baseline() {
  local best_name="" best_source="" found=0
  local status_json status_name live_json live_name

  if status_json=$(curl --noproxy '*' -sS --fail --max-time 10 "$RELEASE_API_BASE/status?kind=server" 2>/dev/null); then
    status_name=$(json_field "$status_json" lastPublishedVersionName)
    if [ -n "$status_name" ]; then
      best_name="$status_name"
      best_source="/api/release/status"
      found=1
    fi
  else
    echo -e "${YELLOW}   ⚠️  后端版本基线读取失败：/api/release/status?kind=server${NC}" >&2
  fi

  if live_json=$(curl --noproxy '*' -sS --fail --max-time 10 "$SERVER_HTTP_BASE/api/server/version" 2>/dev/null); then
    live_name=$(json_field "$live_json" versionName)
    if [ -n "$live_name" ]; then
      if [ "$found" -eq 1 ] && [ "$live_name" != "$best_name" ]; then
        echo -e "${YELLOW}   ⚠️  服务器后端版本来源不一致：/api/server/version=v${live_name}，release/status=v${best_name}，采用较高版本${NC}" >&2
      fi
      if [ "$found" -eq 0 ] || version_gt "$live_name" "$best_name"; then
        best_name="$live_name"
        best_source="/api/server/version"
      fi
      found=1
    fi
  else
    echo -e "${YELLOW}   ⚠️  后端版本基线读取失败：/api/server/version${NC}" >&2
  fi

  if [ "$found" -eq 0 ]; then
    echo -e "${RED}❌ 无法读取服务器后端版本基线；发布已停止，避免用 Cargo.toml 兜底版本发布。${NC}" >&2
    return 1
  fi

  printf '%s|%s\n' "$best_name" "$best_source"
}

read_deployed_server_sha() {
  local deployed_sha_file="$REMOTE_DIR/.deployed-sha"
  local live_sha file_sha
  live_sha=$(curl --noproxy '*' -s --max-time 5 "${SERVER_HTTP_BASE:-$PUBLIC_SERVER_HTTP}/api/server/version" 2>/dev/null | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('gitSha',''))" 2>/dev/null || true)
  if [[ "$live_sha" =~ ^[0-9a-f]{40}$ ]]; then
    echo "$live_sha"
    return
  fi
  if [ "${LOCAL_DEPLOY:-0}" -eq 1 ]; then
    file_sha=$(cat "$deployed_sha_file" 2>/dev/null || true)
  else
    # shellcheck disable=SC2086
    file_sha=$(ssh $SSH_OPTS "$SERVER" "cat $deployed_sha_file 2>/dev/null || true" 2>/dev/null | tr -d '[:space:]' || true)
  fi
  [[ "$file_sha" =~ ^[0-9a-f]{40}$ ]] && echo "$file_sha" || true
}

server_runtime_unchanged_since() {
  local base_sha="$1"
  [[ "$base_sha" =~ ^[0-9a-f]{40}$ ]] || return 1
  git -C "$REPO_ROOT" merge-base --is-ancestor "$base_sha" "$SHA_BIG" 2>/dev/null || return 1
  git -C "$REPO_ROOT" diff --quiet "$base_sha" "$SHA_BIG" -- \
    server/src server/Cargo.toml server/homecli-proto
}

cargo_config_candidates() {
  local dir="$REPO_ROOT"
  while [ -n "$dir" ] && [ "$dir" != "/" ]; do
    [ -f "$dir/.cargo/config.toml" ] && printf '%s\n' "$dir/.cargo/config.toml"
    [ -f "$dir/.cargo/config" ] && printf '%s\n' "$dir/.cargo/config"
    dir="$(dirname "$dir")"
  done

  local cargo_home="${CARGO_HOME:-$HOME/.cargo}"
  [ -f "$cargo_home/config.toml" ] && printf '%s\n' "$cargo_home/config.toml"
  [ -f "$cargo_home/config" ] && printf '%s\n' "$cargo_home/config"
}

warn_native_rustflags() {
  local found=0
  local target_env="CARGO_TARGET_${TARGET^^}_RUSTFLAGS"
  target_env="${target_env//-/_}"
  local env_name env_value config_path

  for env_name in RUSTFLAGS CARGO_ENCODED_RUSTFLAGS CARGO_BUILD_RUSTFLAGS "$target_env"; do
    env_value="${!env_name-}"
    if [[ "$env_value" =~ target-cpu[[:space:]]*=?[[:space:]]*\"?native ]]; then
      if [ "$found" -eq 0 ]; then
        echo -e "${YELLOW}   ⚠️  检测到 target-cpu=native，发布脚本将忽略这些机器级 rustflags：${NC}"
      fi
      found=1
      echo -e "${YELLOW}      env:$env_name${NC}"
    fi
  done

  while IFS= read -r config_path; do
    if grep -Eq 'target-cpu[[:space:]]*=?[[:space:]]*"?native' "$config_path"; then
      if [ "$found" -eq 0 ]; then
        echo -e "${YELLOW}   ⚠️  检测到 target-cpu=native，发布脚本将忽略这些机器级 rustflags：${NC}"
      fi
      found=1
      echo -e "${YELLOW}      $config_path${NC}"
    fi
  done < <(cargo_config_candidates | awk '!seen[$0]++')
}

enable_portable_release_rustflags() {
  local target_env="CARGO_TARGET_${TARGET^^}_RUSTFLAGS"
  target_env="${target_env//-/_}"

  SAVED_RUSTFLAGS_SET="${RUSTFLAGS+x}"
  SAVED_RUSTFLAGS="${RUSTFLAGS-}"
  SAVED_CARGO_ENCODED_RUSTFLAGS_SET="${CARGO_ENCODED_RUSTFLAGS+x}"
  SAVED_CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS-}"
  SAVED_CARGO_BUILD_RUSTFLAGS_SET="${CARGO_BUILD_RUSTFLAGS+x}"
  SAVED_CARGO_BUILD_RUSTFLAGS="${CARGO_BUILD_RUSTFLAGS-}"
  SAVED_TARGET_RUSTFLAGS_NAME="$target_env"
  SAVED_TARGET_RUSTFLAGS_SET="${!target_env+x}"
  SAVED_TARGET_RUSTFLAGS="${!target_env-}"

  warn_native_rustflags

  # Cargo prioritizes CARGO_ENCODED_RUSTFLAGS over RUSTFLAGS and config files.
  # Force release builds to a portable x86-64 baseline so artifacts built on a
  # different CPU cannot SIGILL on the deployment server.
  export CARGO_ENCODED_RUSTFLAGS=$'-C\x1ftarget-cpu=x86-64'
  unset RUSTFLAGS CARGO_BUILD_RUSTFLAGS "$target_env"
  echo -e "${GREEN}   ✅ Release rustflags: -C target-cpu=x86-64（屏蔽全局 target-cpu=native）${NC}"
}

restore_release_rustflags() {
  if [ "${SAVED_RUSTFLAGS_SET:-}" = "x" ]; then export RUSTFLAGS="$SAVED_RUSTFLAGS"; else unset RUSTFLAGS; fi
  if [ "${SAVED_CARGO_ENCODED_RUSTFLAGS_SET:-}" = "x" ]; then export CARGO_ENCODED_RUSTFLAGS="$SAVED_CARGO_ENCODED_RUSTFLAGS"; else unset CARGO_ENCODED_RUSTFLAGS; fi
  if [ "${SAVED_CARGO_BUILD_RUSTFLAGS_SET:-}" = "x" ]; then export CARGO_BUILD_RUSTFLAGS="$SAVED_CARGO_BUILD_RUSTFLAGS"; else unset CARGO_BUILD_RUSTFLAGS; fi
  if [ -n "${SAVED_TARGET_RUSTFLAGS_NAME:-}" ]; then
    if [ "${SAVED_TARGET_RUSTFLAGS_SET:-}" = "x" ]; then
      export "$SAVED_TARGET_RUSTFLAGS_NAME=$SAVED_TARGET_RUSTFLAGS"
    else
      unset "$SAVED_TARGET_RUSTFLAGS_NAME"
    fi
  fi
}

# ── 路径推导（兼容任意 PC、任意路径）──────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || git rev-parse --show-toplevel)"
if [ -z "$REPO_ROOT" ]; then
  echo -e "${RED}❌ 当前目录不在 git 仓库中${NC}" >&2; exit 1
fi
SERVER_DIR="$REPO_ROOT/server"
if [ ! -f "$SERVER_DIR/Cargo.toml" ]; then
  echo -e "${RED}❌ 找不到 $SERVER_DIR/Cargo.toml${NC}" >&2; exit 1
fi

load_local_env_file() {
  local env_file="$1"
  [ -f "$env_file" ] || return 0

  while IFS= read -r line || [ -n "$line" ]; do
    line="${line%$'\r'}"
    [[ "$line" =~ ^[[:space:]]*$ ]] && continue
    [[ "$line" =~ ^[[:space:]]*# ]] && continue

    if [[ "$line" =~ ^[[:space:]]*([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*=[[:space:]]*(.*)$ ]]; then
      name="${BASH_REMATCH[1]}"
      value="${BASH_REMATCH[2]}"
      value="${value#"${value%%[![:space:]]*}"}"
      value="${value%"${value##*[![:space:]]}"}"
      if [[ "$value" == \"*\" && "$value" == *\" ]]; then
        value="${value:1:${#value}-2}"
      elif [[ "$value" == \'*\' && "$value" == *\' ]]; then
        value="${value:1:${#value}-2}"
      fi
      if [ -z "${!name+x}" ]; then
        export "$name=$value"
      fi
    fi
  done < "$env_file"
}

load_local_env_file "$REPO_ROOT/.env.local"

if is_local_server_deploy; then
  LOCAL_DEPLOY=1
  SERVER_HTTP_BASE="$LOCAL_SERVER_HTTP"
else
  LOCAL_DEPLOY=0
  SERVER_HTTP_BASE="$PUBLIC_SERVER_HTTP"
fi
RELEASE_API_BASE="$SERVER_HTTP_BASE/api/release"

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${CYAN}   elon cli 服务端  交叉编译 + 部署${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${GRAY}  仓库根: $REPO_ROOT${NC}"
echo -e "${GRAY}  目标:   $TARGET${NC}"
echo -e "${GRAY}  服务器: $SERVER${NC}"
if [ "$LOCAL_DEPLOY" -eq 1 ]; then
  echo -e "${GRAY}  部署模式: 本机部署（跳过 SSH/SCP）${NC}"
else
  echo -e "${GRAY}  部署模式: 远程 SSH/SCP${NC}"
fi
echo -e "${GRAY}  发布 API: $RELEASE_API_BASE${NC}"
echo ""

# ── cleanup worktree ──────────────────────────────────────────
TMP_WORKTREE=""
cleanup_worktree() {
  if [ -n "$TMP_WORKTREE" ] && [ -d "$TMP_WORKTREE" ]; then
    echo -e "${GRAY}   🧹 清理临时工作树...${NC}"
    git -C "$REPO_ROOT" worktree remove "$TMP_WORKTREE" --force 2>/dev/null || true
  fi
}

on_exit() {
  local code=$?
  cleanup_worktree
  if [ -n "$RELEASE_TOKEN" ] && [ "$RELEASE_FINISHED" -eq 0 ]; then
    complete_release false "" "" "script exited before release finish (exit=$code)"
  fi
}
trap on_exit EXIT

# ── 1. git fetch + fast-forward ───────────────────────────────
echo -e "${YELLOW}1⃣  同步最新代码...${NC}"
git_fetch_with_retry
DIRTY=$(git -C "$REPO_ROOT" status --porcelain)
if [ -n "$DIRTY" ]; then
  echo ""
  echo -e "${RED}❌ 工作区不干净，请先 commit + push 业务改动再运行部署脚本：${NC}" >&2
  echo "$DIRTY" >&2
  exit 1
fi

LOCAL_HEAD=$(git -C "$REPO_ROOT" rev-parse HEAD)
REMOTE_MAIN=$(git -C "$REPO_ROOT" rev-parse origin/main)
if [ "$LOCAL_HEAD" != "$REMOTE_MAIN" ]; then
  if git -C "$REPO_ROOT" merge-base --is-ancestor "$LOCAL_HEAD" "$REMOTE_MAIN" 2>/dev/null; then
    echo -e "${CYAN}   ℹ️  本地 HEAD 已包含在 origin/main 中，快进到最新 main：${REMOTE_MAIN:0:7}${NC}"
    git -C "$REPO_ROOT" merge --ff-only origin/main
  elif git -C "$REPO_ROOT" merge-base --is-ancestor "$REMOTE_MAIN" "$LOCAL_HEAD" 2>/dev/null; then
    echo ""
    echo -e "${RED}❌ 当前 HEAD 尚未进入 origin/main，禁止基于未推送提交编译部署。${NC}" >&2
    echo -e "${YELLOW}   当前 HEAD:  $LOCAL_HEAD${NC}" >&2
    echo -e "${YELLOW}   origin/main: $REMOTE_MAIN${NC}" >&2
    echo -e "${YELLOW}   请先执行：git push origin HEAD:main${NC}" >&2
    exit 1
  else
    echo ""
    echo -e "${RED}❌ 当前 HEAD 与 origin/main 已分叉，发布脚本不会自动 rebase。请先完成代码合并并 push 后再运行。${NC}" >&2
    echo -e "${YELLOW}   当前 HEAD:  $LOCAL_HEAD${NC}" >&2
    echo -e "${YELLOW}   origin/main: $REMOTE_MAIN${NC}" >&2
    exit 1
  fi
fi

SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
SHA_BIG=$(git -C "$REPO_ROOT" rev-parse HEAD)
REMOTE_MAIN=$(git -C "$REPO_ROOT" rev-parse origin/main)
if [ "$SHA_BIG" != "$REMOTE_MAIN" ]; then
  echo ""
  echo -e "${RED}❌ 同步后 HEAD 仍不等于 origin/main，发布脚本停止。${NC}" >&2
  echo -e "${YELLOW}   当前 HEAD:  $SHA_BIG${NC}" >&2
  echo -e "${YELLOW}   origin/main: $REMOTE_MAIN${NC}" >&2
  exit 1
fi

FALLBACK_VERSION=$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)"/\1/p' "$SERVER_DIR/Cargo.toml" | head -1)
SERVER_VERSION_BASELINE=$(resolve_server_version_baseline) || exit 1
CLAIM_CURRENT_VERSION="${SERVER_VERSION_BASELINE%%|*}"
CLAIM_CURRENT_SOURCE="${SERVER_VERSION_BASELINE#*|}"
echo -e "${GREEN}   ✅ 最新 SHA: $SHA${NC}"
DEPLOYED_SERVER_SHA=$(read_deployed_server_sha)
if [ "$FORCE" -eq 0 ] && [ -n "$DEPLOYED_SERVER_SHA" ] && [ "$DEPLOYED_SERVER_SHA" != "$SHA_BIG" ]; then
  if server_runtime_unchanged_since "$DEPLOYED_SERVER_SHA"; then
    echo -e "${GREEN}   ✅ 后端运行代码未变化，复用线上 binary（deployed ${DEPLOYED_SERVER_SHA:0:7}）${NC}"
    echo -e "${GRAY}      如需强制重编译/重启：bash scripts/publish-server.sh --force${NC}"
    exit 0
  fi
fi
echo -e "${GREEN}   ✅ Cargo.toml 兜底版本: v$FALLBACK_VERSION${NC}"
echo -e "${GREEN}   ✅ 服务器后端版本基线: v$CLAIM_CURRENT_VERSION [$CLAIM_CURRENT_SOURCE]${NC}"

# ── 1.5 从服务器原子分配新版本号（claim）─────────────────────
echo -e "${YELLOW}1.5⃣  向服务器申请新版本号...${NC}"
BUILDER_HOST="${HOSTNAME:-$(hostname 2>/dev/null || echo unknown-host)}"
BUILDER_USER="${USER:-${LOGNAME:-unknown-user}}"
BUILDER_ID="$BUILDER_HOST-$BUILDER_USER"
BUILDER_LABEL="publish-server.sh @ $BUILDER_ID"
CLAIM_PAYLOAD=$(python3 - "$SHA_BIG" "$BUILDER_ID" "$BUILDER_LABEL" "$CLAIM_CURRENT_VERSION" <<'PY'
import json
import sys

sha, builder_id, builder_label, current_version = sys.argv[1:5]
payload = {
    "kind": "server",
    "sha": sha,
    "builderId": builder_id,
    "builderLabel": builder_label,
    "bump": "patch",
}
if current_version:
    payload["currentVersionName"] = current_version
print(json.dumps(payload, separators=(",", ":")))
PY
)
if ! CLAIM_JSON=$(release_post claim "$CLAIM_PAYLOAD"); then
  echo -e "${RED}❌ /api/release/claim 失败，未开始编译。${NC}" >&2
  exit 1
fi

CLAIM_ACTION=$(json_field "$CLAIM_JSON" action)
RELEASE_TOKEN=$(json_field "$CLAIM_JSON" token)
ASSIGNED_VERSION=$(json_field "$CLAIM_JSON" assignedVersionName)
IN_FLIGHT_COUNT=$(json_field "$CLAIM_JSON" inFlightCount)
if [ "$CLAIM_ACTION" != "build" ] || [ -z "$RELEASE_TOKEN" ] || [ -z "$ASSIGNED_VERSION" ]; then
  echo -e "${RED}❌ release/claim 返回非预期响应：$CLAIM_JSON${NC}" >&2
  exit 1
fi
echo -e "${GREEN}   ✅ 已分配版本号: v$ASSIGNED_VERSION (token=${RELEASE_TOKEN:0:8}..., in-flight=${IN_FLIGHT_COUNT:-1})${NC}"

# ── 2. 环境检查 ───────────────────────────────────────────────
if [ "$SKIP_BUILD" -eq 0 ]; then
  if ! command -v zig &>/dev/null; then
    echo -e "${RED}❌ 未找到 zig！请先安装：${NC}"
    echo -e "${YELLOW}   Ubuntu: sudo snap install zig --classic --channel=latest/stable${NC}"
    echo -e "${YELLOW}   或访问: https://ziglang.org/download/${NC}"
    exit 1
  fi
  ZIG_VER=$(zig version 2>&1 | head -1)
  echo -e "${GRAY}   zig: $ZIG_VER${NC}"

  if ! cargo zigbuild --version &>/dev/null 2>&1; then
    echo -e "${YELLOW}📦 安装 cargo-zigbuild...${NC}"
    cargo install cargo-zigbuild
  fi

  if ! rustup target list --installed 2>/dev/null | grep -q "$TARGET"; then
    echo -e "${YELLOW}📦 添加 rustup target $TARGET...${NC}"
    rustup target add "$TARGET"
  fi

fi

# ── 3. 编译（临时工作树）─────────────────────────────────────
TMP_WORKTREE="$(dirname "$REPO_ROOT")/elon-build-$SHA"
if [ -n "${RUST_SERVER_MUSL_TARGET_DIR:-}" ]; then
  case "$RUST_SERVER_MUSL_TARGET_DIR" in
    /*) ;;
    *) echo -e "${RED}❌ RUST_SERVER_MUSL_TARGET_DIR 必须是绝对路径，当前值: $RUST_SERVER_MUSL_TARGET_DIR${NC}" >&2; exit 1 ;;
  esac
  BUILD_TARGET_DIR="$RUST_SERVER_MUSL_TARGET_DIR"
elif [ -n "${RUST_MUSL_TARGET_DIR:-}" ]; then
  case "$RUST_MUSL_TARGET_DIR" in
    /*) ;;
    *) echo -e "${RED}❌ RUST_MUSL_TARGET_DIR 必须是绝对路径，当前值: $RUST_MUSL_TARGET_DIR${NC}" >&2; exit 1 ;;
  esac
  BUILD_TARGET_DIR="$RUST_MUSL_TARGET_DIR"
elif [ -n "${ELON_BUILD_TARGET_DIR:-}" ]; then
  case "$ELON_BUILD_TARGET_DIR" in
    /*) ;;
    *) echo -e "${RED}❌ ELON_BUILD_TARGET_DIR 必须是绝对路径，当前值: $ELON_BUILD_TARGET_DIR${NC}" >&2; exit 1 ;;
  esac
  mkdir -p "$ELON_BUILD_TARGET_DIR"
  # 固定子目录名，让所有 builder 共享同一份增量编译缓存
  BUILD_TARGET_DIR="$ELON_BUILD_TARGET_DIR/elon-server-musl"
else
  # 无自定义路径时，用 XDG 标准缓存目录（Ubuntu/macOS 通用，无需手动配置）
  BUILD_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/elon/build/elon-server-musl"
fi
mkdir -p "$BUILD_TARGET_DIR"
BUILD_BIN="$BUILD_TARGET_DIR/$TARGET/release/elon-server"
BINARY="$BUILD_BIN"
echo -e "${GRAY}  构建缓存: $BUILD_TARGET_DIR${NC}"

if [ "$SKIP_BUILD" -eq 0 ]; then
  # 清理残留工作树
  if [ -d "$TMP_WORKTREE" ]; then
    git -C "$REPO_ROOT" worktree remove "$TMP_WORKTREE" --force 2>/dev/null || true
  fi

  echo -e "${YELLOW}2⃣  创建临时工作树（$SHA）...${NC}"
  git -C "$REPO_ROOT" worktree add --detach "$TMP_WORKTREE" HEAD

  echo -e "${YELLOW}3⃣  交叉编译 → $TARGET...${NC}"
  TMP_SERVER_DIR="$TMP_WORKTREE/server"
  export CARGO_TARGET_DIR="$BUILD_TARGET_DIR"
  enable_portable_release_rustflags
  if ! (
    cd "$TMP_SERVER_DIR"
    ELON_SERVER_GIT_SHA="$SHA_BIG" ELON_BUILD_VERSION="$ASSIGNED_VERSION" cargo zigbuild --release --target "$TARGET"
  ); then
    restore_release_rustflags
    unset CARGO_TARGET_DIR
    complete_release false "" "" "cargo zigbuild failed"
    echo -e "${RED}❌ 编译失败${NC}" >&2
    exit 1
  fi
  restore_release_rustflags
  unset CARGO_TARGET_DIR

  if [ ! -f "$BINARY" ]; then
    complete_release false "" "" "binary missing after build"
    echo -e "${RED}❌ 编译产物不存在: $BINARY${NC}" >&2; exit 1
  fi

  SIZE_MB=$(awk "BEGIN {printf \"%.1f\", $(stat -c%s "$BINARY" 2>/dev/null || stat -f%z "$BINARY") / 1048576}")
  echo -e "${GREEN}   ✅ 编译成功！产物 ${SIZE_MB} MB${NC}"

else
  echo -e "${YELLOW}2⃣  ⏩ 跳过编译（--skip-build）${NC}"
  if [ ! -f "$BINARY" ]; then
    FALLBACK_TARGET_DIR="${CARGO_TARGET_DIR:-$SERVER_DIR/target}"
    BINARY="$FALLBACK_TARGET_DIR/$TARGET/release/elon-server"
    if [ ! -f "$BINARY" ]; then
      echo -e "${RED}❌ 找不到编译产物，请先不带 --skip-build 运行一次${NC}" >&2; exit 1
    fi
    echo -e "${GRAY}   使用已有产物: $BINARY${NC}"
  fi
fi

if [ "$SKIP_UPLOAD" -eq 1 ]; then
  echo ""
  echo -e "${GREEN}✅ 本地编译完成（--skip-upload，未部署）${NC}"
  echo -e "${GRAY}   产物: $BINARY${NC}"
  complete_release false "" "" "skip upload (local build only)"
  exit 0
fi

# ── 3.5 构建/上传 PC 新版与旧版对照快照 ─────────────────────
PC_FRONTEND_DIR="$REPO_ROOT/pc-frontend"
PC_DIST_DIR="$PC_FRONTEND_DIR/dist"
PC_LEGACY_BASE_COMMIT="d1f89950eb09d1911aae601f7cdedc583101e1d2"
PC_LEGACY_DIST_DIR="$REPO_ROOT/target/pc-legacy-dist"
REMOTE_DATA_DIR="/opt/elon/data"
REMOTE_PC_DIST="$REMOTE_DATA_DIR/pc-next-dist"
REMOTE_PC_LEGACY_DIST="$REMOTE_DATA_DIR/pc-legacy-dist"

upload_static_dist() {
  local local_dir="$1"
  local remote_dir="$2"
  local label="$3"
  local staging_dir="${remote_dir}-staging-$SHA"

  if [ -z "$local_dir" ] || [ ! -f "$local_dir/index.html" ]; then
    echo -e "${YELLOW}3.5⃣  ⚠️  $label 不存在，跳过上传${NC}"
    return 0
  fi

  echo -e "${YELLOW}3.5⃣  上传 $label 到 $remote_dir ...${NC}"
  if [ "$LOCAL_DEPLOY" -eq 1 ]; then
    rm -rf "$staging_dir"
    mkdir -p "$staging_dir"
    cp -a "$local_dir"/. "$staging_dir"/
    rm -rf "$remote_dir"
    mv "$staging_dir" "$remote_dir"
  else
    # shellcheck disable=SC2086
    if ! ssh $SSH_OPTS "$SERVER" "mkdir -p '$staging_dir'"; then
      echo -e "${YELLOW}   ⚠️  $label staging 目录创建失败（不中止后端部署）${NC}"
      return 0
    fi
    if ! scp $SSH_OPTS -r "$local_dir/." "${SERVER}:${staging_dir}"; then
      # shellcheck disable=SC2086
      ssh $SSH_OPTS "$SERVER" "rm -rf '$staging_dir'" 2>/dev/null || true
      echo -e "${YELLOW}   ⚠️  $label 上传失败（不中止后端部署）${NC}"
      return 0
    fi
    if ! ssh $SSH_OPTS "$SERVER" "rm -rf '$remote_dir' && mv '$staging_dir' '$remote_dir'"; then
      # shellcheck disable=SC2086
      ssh $SSH_OPTS "$SERVER" "rm -rf '$staging_dir'" 2>/dev/null || true
      echo -e "${YELLOW}   ⚠️  $label 目录替换失败（staging 已清理）${NC}"
      return 0
    fi
  fi
  echo -e "${GREEN}   ✅ $label 上传并替换完成 → $remote_dir${NC}"
}

export_pc_legacy_dist() {
  local commit="$1"
  local out_dir="$2"
  local assets_dir="$out_dir/assets"
  local asset_path name tmp_html brand_file

  rm -rf "$out_dir"
  mkdir -p "$assets_dir"

  while IFS= read -r asset_path; do
    name="$(basename "$asset_path")"
    [ "$name" = "pc_app.html" ] && continue
    if [[ "$name" == pc_* || "$name" == "voice_tts_sdk.js" ]]; then
      git -C "$REPO_ROOT" show "${commit}:${asset_path}" > "$assets_dir/$name"
    fi
  done < <(git -C "$REPO_ROOT" ls-tree -r --name-only "$commit" -- server/src/assets)

  tmp_html="$out_dir/.pc_app.html"
  git -C "$REPO_ROOT" show "${commit}:server/src/assets/pc_app.html" > "$tmp_html"
  brand_file="$REPO_ROOT/server/src/assets/ic_app_brand.b64"
  python3 - "$tmp_html" "$brand_file" "$out_dir/index.html" <<'PY'
from pathlib import Path
import sys

source = Path(sys.argv[1])
brand_file = Path(sys.argv[2])
target = Path(sys.argv[3])

html = source.read_text(encoding="utf-8")
brand = brand_file.read_text(encoding="utf-8").strip() if brand_file.exists() else ""
html = html.replace("__BRAND_PNG_B64__", brand)
html = html.replace('"/assets/', '"/pc-legacy/assets/')
html = html.replace("'/assets/", "'/pc-legacy/assets/")
target.write_text(html, encoding="utf-8")
source.unlink(missing_ok=True)
PY
}

if [ -f "$PC_FRONTEND_DIR/package.json" ]; then
  if ! command -v npm >/dev/null 2>&1; then
    echo -e "${YELLOW}3.5⃣  ⚠️  npm 不在 PATH，跳过新版 PC 前端构建${NC}"
    PC_DIST_DIR=""
  elif [ "$SKIP_BUILD" -eq 1 ] && [ -f "$PC_DIST_DIR/index.html" ]; then
    echo -e "${YELLOW}3.5⃣  ⏩ 跳过新版 PC 前端构建（--skip-build），使用已有 dist${NC}"
  elif ! (
    cd "$PC_FRONTEND_DIR"
    if [ ! -d node_modules ]; then
      echo -e "${GRAY}   📦 安装前端依赖（npm ci）...${NC}"
      npm ci
    fi
    npm run build
  ); then
    if [ -x "$PC_FRONTEND_DIR/node_modules/.bin/tsc" ] && [ -x "$PC_FRONTEND_DIR/node_modules/.bin/vite" ]; then
      echo -e "${GRAY}   🔁 npm 构建失败，尝试直接使用 node_modules/.bin/tsc + vite ...${NC}"
      if ! (
        cd "$PC_FRONTEND_DIR"
        node_modules/.bin/tsc --noEmit
        node_modules/.bin/vite build
      ); then
        echo -e "${YELLOW}   ⚠️  新版 PC 前端构建失败（不中止后端部署）${NC}"
        PC_DIST_DIR=""
      fi
    else
      echo -e "${YELLOW}   ⚠️  新版 PC 前端构建失败（不中止后端部署）${NC}"
      PC_DIST_DIR=""
    fi
  fi
  upload_static_dist "$PC_DIST_DIR" "$REMOTE_PC_DIST" "新版 PC 前端 dist"
else
  echo -e "${GRAY}3.5⃣  ℹ️  pc-frontend/ 不存在，跳过新版 PC 前端构建${NC}"
fi

echo -e "${YELLOW}3.5⃣  生成旧版 PC 对照快照（${PC_LEGACY_BASE_COMMIT:0:8}）...${NC}"
if export_pc_legacy_dist "$PC_LEGACY_BASE_COMMIT" "$PC_LEGACY_DIST_DIR"; then
  upload_static_dist "$PC_LEGACY_DIST_DIR" "$REMOTE_PC_LEGACY_DIST" "旧版 PC 对照快照"
else
  echo -e "${YELLOW}   ⚠️  旧版 PC 对照快照生成失败（不中止后端部署）${NC}"
fi

# ── 4. 上传到服务器（staging 路径含 SHA，避免并发覆盖）────────
echo -e "${YELLOW}4⃣  上传 binary 到服务器...${NC}"
STAGING_PATH="/tmp/elon-server-$SHA"
if [ "$LOCAL_DEPLOY" -eq 1 ]; then
  cp "$BINARY" "$STAGING_PATH"
else
  # shellcheck disable=SC2086
  scp $SSH_OPTS "$BINARY" "${SERVER}:${STAGING_PATH}"
fi
echo -e "${GREEN}   ✅ 上传完成${NC}"

# ── 4.5 SHA 顺序检查（防止旧版编译慢覆盖新版）───────────────
if [ "$FORCE" -eq 0 ]; then
  SERVER_SHA=$(read_deployed_server_sha)
  if [ -n "$SERVER_SHA" ] && [ "$SERVER_SHA" != "$SHA_BIG" ]; then
    # 检查服务器 SHA 是否是我们的祖先（是祖先 = 我们更新）
    if ! git -C "$REPO_ROOT" merge-base --is-ancestor "$SERVER_SHA" "$SHA_BIG" 2>/dev/null; then
      # 服务器已有更新版本，拒绝回退
      if [ "$LOCAL_DEPLOY" -eq 1 ]; then
        rm -f "$STAGING_PATH" 2>/dev/null || true
      else
        # shellcheck disable=SC2086
        ssh $SSH_OPTS "$SERVER" "rm -f $STAGING_PATH" 2>/dev/null || true
      fi
      SHORT_SERVER="${SERVER_SHA:0:8}"
      echo ""
      echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
      echo -e "${YELLOW}   ⚠️  部署已中止：服务器版本更新${NC}"
      echo -e "${YELLOW}   服务器当前: $SHORT_SERVER（比本次 $SHA 更新）${NC}"
      echo -e "${YELLOW}   原因：另一个开发者已部署了更新版本，本次编译基于旧 commit。${NC}"
      echo -e "${YELLOW}   处理：代码已合并，发布交给最新主线；明确发布协调任务可重新运行，或用 --force 强制覆盖。${NC}"
      complete_release false "" "" "superseded by server sha $SERVER_SHA"
      echo -e "${YELLOW}   release/finish 已调用 (success=false)，分配的 v$ASSIGNED_VERSION 已释放。${NC}"
      print_publish_status "superseded_by_newer_main" "synced" "not_attempted" "代码已合并，发布交给最新主线。"
      echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
      echo ""
      exit 0
    fi
  fi
  echo -e "${GREEN}   ✅ SHA 顺序检查通过（本次 $SHA 是最新版本）${NC}"
fi

# ── 5. 替换 binary + 重启服务（flock 互斥锁 + CAS 原子化）─────
# 锁保护范围：CAS 校验 .deployed-sha + mv + restart + 写新 SHA。
# 即使两台 PC 都通过了步骤 4.5 祖先检查，锁内仍会重新比对
# .deployed-sha == EXPECTED，任何中途被别人抢先 → 退出码 42 → 拒绝覆盖。
echo -e "${YELLOW}5⃣  替换 binary 并重启服务（flock 互斥锁保护）...${NC}"
REMOTE_BIN_DIR=$(dirname "$REMOTE_BIN")
if [ "$FORCE" -eq 1 ]; then
  EXPECTED_SHA='__FORCE__'
else
  EXPECTED_SHA="${SERVER_SHA:-}"
fi

LOCK_SCRIPT=$(cat <<EOF
set -e
EXPECTED='${EXPECTED_SHA}'
NEW='${SHA_BIG}'
STAGING='${STAGING_PATH}'
DEST='${REMOTE_BIN}'
DEST_DIR='${REMOTE_BIN_DIR}'
SHA_FILE='${REMOTE_DIR}/.deployed-sha'
REMOTE_DIR_INNER='${REMOTE_DIR}'
CURRENT=\$(cat "\$SHA_FILE" 2>/dev/null || echo '')
if [ "\$EXPECTED" != "__FORCE__" ] && [ -n "\$CURRENT" ] && [ "\$CURRENT" != "\$EXPECTED" ]; then
  echo "CAS_CONFLICT current=\$CURRENT expected=\$EXPECTED" >&2
  rm -f "\$STAGING" 2>/dev/null || true
  exit 42
fi
mkdir -p "\$DEST_DIR"
mv "\$STAGING" "\$DEST"
chmod +x "\$DEST"
if systemctl is-enabled elon-server >/dev/null 2>&1; then
  systemctl restart elon-server
else
  pkill -f elon-server 2>/dev/null || true
  sleep 1
  fuser -k 8080/tcp 2>/dev/null || true
  sleep 1
  cd "\$REMOTE_DIR_INNER" && nohup "\$DEST" </dev/null >> /root/elon-server.log 2>&1 & disown
  sleep 2
fi
echo "\$NEW" > "\$SHA_FILE"
echo OK
EOF
)

set +e
if [ "$LOCAL_DEPLOY" -eq 1 ]; then
  LOCK_OUT=$(echo "$LOCK_SCRIPT" | flock -x -w 120 /tmp/elon-deploy.lock bash -s 2>&1)
else
  # shellcheck disable=SC2086
  LOCK_OUT=$(echo "$LOCK_SCRIPT" | ssh $SSH_OPTS "$SERVER" "flock -x -w 120 /tmp/elon-deploy.lock bash -s" 2>&1)
fi
LOCK_EXIT=$?
set -e
if [ "$LOCK_EXIT" -eq 42 ]; then
  echo ""
  echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
  echo -e "${YELLOW}   ⚠️  部署已中止：CAS 冲突（锁内检测到并发部署）${NC}"
  echo -e "${YELLOW}   $LOCK_OUT${NC}"
  echo -e "${YELLOW}   处理：代码已合并，发布交给最新主线；明确发布协调任务可重新运行，或用 --force 强制覆盖。${NC}"
  complete_release false "" "" "cas conflict inside flock: $LOCK_OUT"
  echo -e "${YELLOW}   release/finish 已调用 (success=false)，分配的 v$ASSIGNED_VERSION 已释放。${NC}"
  print_publish_status "superseded_by_newer_main" "synced" "not_attempted" "代码已合并，发布交给最新主线。"
  echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
  exit 0
elif [ "$LOCK_EXIT" -ne 0 ]; then
  complete_release false "" "" "deploy script failed: exit=$LOCK_EXIT"
  echo -e "${RED}❌ 锁内部署失败（exit=$LOCK_EXIT）: $LOCK_OUT${NC}" >&2
  exit 1
fi

echo -e "${GREEN}   ✅ 服务重启指令已发送（锁内完成 mv + restart + 写 SHA）${NC}"
echo -e "${GREEN}   ✅ SHA 记录已写入服务器 (.deployed-sha = $SHA)${NC}"

# ── 6. 验证 ──────────────────────────────────────────────────
echo -e "${YELLOW}6⃣  等待服务启动（3 秒）...${NC}"
sleep 3

HEALTH=$(curl --noproxy '*' -s --max-time 10 "$SERVER_HTTP_BASE/health" 2>&1 || true)
if [ -n "$HEALTH" ]; then
  echo -e "${GREEN}   ✅ 健康检查: $HEALTH${NC}"
else
  echo -e "${YELLOW}   ⚠️  健康检查无响应（服务可能还在启动中）${NC}"
  echo -e "${YELLOW}      手动确认：curl --noproxy '*' $SERVER_HTTP_BASE/health${NC}"
fi

SERVER_VERSION_JSON=$(curl --noproxy '*' -s --max-time 10 "$SERVER_HTTP_BASE/api/server/version" 2>&1 || true)
if [ -n "$SERVER_VERSION_JSON" ]; then
  echo -e "${GREEN}   ✅ 后端版本接口: $SERVER_VERSION_JSON${NC}"
else
  echo -e "${YELLOW}   ⚠️  后端版本接口无响应${NC}"
  echo -e "${YELLOW}      手动确认：curl --noproxy '*' $SERVER_HTTP_BASE/api/server/version${NC}"
  complete_release false "" "" "server version endpoint empty after deploy"
  exit 1
fi

DEPLOYED_VERSION_NAME=$(json_field "$SERVER_VERSION_JSON" versionName)
DEPLOYED_GIT_SHA=$(json_field "$SERVER_VERSION_JSON" gitSha)
if [ "$DEPLOYED_VERSION_NAME" != "$ASSIGNED_VERSION" ] || [ "$DEPLOYED_GIT_SHA" != "$SHA_BIG" ]; then
  complete_release false "" "" "server version mismatch after deploy: version=$DEPLOYED_VERSION_NAME sha=$DEPLOYED_GIT_SHA"
  echo -e "${RED}❌ 后端版本校验失败，未登记发布成功。${NC}" >&2
  echo -e "${YELLOW}   期望版本: v$ASSIGNED_VERSION${NC}" >&2
  echo -e "${YELLOW}   实际版本: v${DEPLOYED_VERSION_NAME:-unknown}${NC}" >&2
  echo -e "${YELLOW}   期望 SHA: $SHA_BIG${NC}" >&2
  echo -e "${YELLOW}   实际 SHA: ${DEPLOYED_GIT_SHA:-unknown}${NC}" >&2
  exit 1
fi

# ── 7. 清理工作树（由 trap EXIT 自动执行）────────────────────
complete_release true "$ASSIGNED_VERSION" "$SHA_BIG" ""

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   ✅ 部署完成！${NC}"
echo -e "${GRAY}   版本:   v$ASSIGNED_VERSION（服务器分配，未写入 git）${NC}"
echo -e "${GRAY}   SHA:    $SHA${NC}"
echo -e "${GRAY}   服务:   $SERVER_HTTP_BASE/health${NC}"
print_publish_status "published"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo ""

# 自动清理已合并、工作树干净的孤儿 task worktree
if [[ -x "$REPO_ROOT/scripts/cleanup-task-worktrees.sh" ]]; then
  cleanup_out="$(bash "$REPO_ROOT/scripts/cleanup-task-worktrees.sh" --apply 2>&1 || true)"
  removed_line="$(printf '%s\n' "$cleanup_out" | grep -E '^完成：清理' | tail -n 1 || true)"
  [[ -n "$removed_line" ]] && echo -e "${GRAY}   $removed_line（自动）${NC}"
fi
