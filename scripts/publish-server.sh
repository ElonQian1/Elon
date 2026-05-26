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
#    ./scripts/publish-server.sh --skip-build    # 跳过编译，用上次产物重新部署
#    ./scripts/publish-server.sh --skip-upload   # 只本地编译，不部署
#    ./scripts/publish-server.sh --force         # 强制部署，即使服务器已有更新版本
#
#  机器级 server-musl 缓存目录可在仓库根 .env.local 中设置：
#    RUST_SERVER_MUSL_TARGET_DIR=/var/tmp/server-musl-target
#  兼容旧名 RUST_MUSL_TARGET_DIR。
#  旧的 ELON_BUILD_TARGET_DIR 仍兼容，脚本会在其下创建 elon-server-musl/
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

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${CYAN}   elon cli 服务端  交叉编译 + 部署${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${GRAY}  仓库根: $REPO_ROOT${NC}"
echo -e "${GRAY}  目标:   $TARGET${NC}"
echo -e "${GRAY}  服务器: $SERVER${NC}"
echo ""

# ── cleanup worktree ──────────────────────────────────────────
TMP_WORKTREE=""
cleanup_worktree() {
  if [ -n "$TMP_WORKTREE" ] && [ -d "$TMP_WORKTREE" ]; then
    echo -e "${GRAY}   🧹 清理临时工作树...${NC}"
    git -C "$REPO_ROOT" worktree remove "$TMP_WORKTREE" --force 2>/dev/null || true
  fi
}
trap cleanup_worktree EXIT

# ── 1. git pull --rebase ──────────────────────────────────────
echo -e "${YELLOW}1⃣  同步最新代码...${NC}"
git -C "$REPO_ROOT" pull --rebase origin main
SHA=$(git -C "$REPO_ROOT" rev-parse --short HEAD)
SHA_BIG=$(git -C "$REPO_ROOT" rev-parse HEAD)
SERVER_VERSION=$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)"/\1/p' "$SERVER_DIR/Cargo.toml" | head -1)
echo -e "${GREEN}   ✅ 最新 SHA: $SHA${NC}"
echo -e "${GREEN}   ✅ 后端版本: v$SERVER_VERSION${NC}"

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
  (
    cd "$TMP_SERVER_DIR"
    ELON_SERVER_GIT_SHA="$SHA_BIG" cargo zigbuild --release --target "$TARGET"
  )
  unset CARGO_TARGET_DIR

  if [ ! -f "$BINARY" ]; then
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
  exit 0
fi

# ── 4. 上传到服务器（staging 路径含 SHA，避免并发覆盖）────────
echo -e "${YELLOW}4⃣  上传 binary 到服务器...${NC}"
STAGING_PATH="/tmp/elon-server-$SHA"
# shellcheck disable=SC2086
scp $SSH_OPTS "$BINARY" "${SERVER}:${STAGING_PATH}"
echo -e "${GREEN}   ✅ 上传完成${NC}"

# ── 4.5 SHA 顺序检查（防止旧版编译慢覆盖新版）───────────────
if [ "$FORCE" -eq 0 ]; then
  DEPLOYED_SHA_FILE="$REMOTE_DIR/.deployed-sha"
  # shellcheck disable=SC2086
  SERVER_SHA=$(ssh $SSH_OPTS "$SERVER" "cat $DEPLOYED_SHA_FILE 2>/dev/null || echo ''" | tr -d '[:space:]')
  if [ -n "$SERVER_SHA" ] && [ "$SERVER_SHA" != "$SHA_BIG" ]; then
    # 检查服务器 SHA 是否是我们的祖先（是祖先 = 我们更新）
    if ! git -C "$REPO_ROOT" merge-base --is-ancestor "$SERVER_SHA" "$SHA_BIG" 2>/dev/null; then
      # 服务器已有更新版本，拒绝回退
      # shellcheck disable=SC2086
      ssh $SSH_OPTS "$SERVER" "rm -f $STAGING_PATH" 2>/dev/null || true
      SHORT_SERVER="${SERVER_SHA:0:8}"
      echo ""
      echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
      echo -e "${YELLOW}   ⚠️  部署已中止：服务器版本更新${NC}"
      echo -e "${YELLOW}   服务器当前: $SHORT_SERVER（比本次 $SHA 更新）${NC}"
      echo -e "${YELLOW}   原因：另一个开发者已部署了更新版本，本次编译基于旧 commit。${NC}"
      echo -e "${YELLOW}   解决：git pull --rebase 后重新编译部署，或用 --force 强制覆盖。${NC}"
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
# shellcheck disable=SC2086
LOCK_OUT=$(echo "$LOCK_SCRIPT" | ssh $SSH_OPTS "$SERVER" "flock -x -w 120 /tmp/elon-deploy.lock bash -s" 2>&1)
LOCK_EXIT=$?
set -e
if [ "$LOCK_EXIT" -eq 42 ]; then
  echo ""
  echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
  echo -e "${YELLOW}   ⚠️  部署已中止：CAS 冲突（锁内检测到并发部署）${NC}"
  echo -e "${YELLOW}   $LOCK_OUT${NC}"
  echo -e "${YELLOW}   解决：git pull --rebase 后重新部署，或用 --force 强制覆盖。${NC}"
  echo -e "${YELLOW}═══════════════════════════════════════════════════${NC}"
  exit 0
elif [ "$LOCK_EXIT" -ne 0 ]; then
  echo -e "${RED}❌ 锁内部署失败（exit=$LOCK_EXIT）: $LOCK_OUT${NC}" >&2
  exit 1
fi

echo -e "${GREEN}   ✅ 服务重启指令已发送（锁内完成 mv + restart + 写 SHA）${NC}"
echo -e "${GREEN}   ✅ SHA 记录已写入服务器 (.deployed-sha = $SHA)${NC}"

# ── 6. 验证 ──────────────────────────────────────────────────
echo -e "${YELLOW}6⃣  等待服务启动（3 秒）...${NC}"
sleep 3

HEALTH=$(curl --noproxy '*' -s --max-time 10 "http://43.139.149.158:8080/health" 2>&1 || true)
if [ -n "$HEALTH" ]; then
  echo -e "${GREEN}   ✅ 健康检查: $HEALTH${NC}"
else
  echo -e "${YELLOW}   ⚠️  健康检查无响应（服务可能还在启动中）${NC}"
  echo -e "${YELLOW}      手动确认：curl --noproxy '*' http://43.139.149.158:8080/health${NC}"
fi

SERVER_VERSION_JSON=$(curl --noproxy '*' -s --max-time 10 "http://43.139.149.158:8080/api/server/version" 2>&1 || true)
if [ -n "$SERVER_VERSION_JSON" ]; then
  echo -e "${GREEN}   ✅ 后端版本接口: $SERVER_VERSION_JSON${NC}"
else
  echo -e "${YELLOW}   ⚠️  后端版本接口无响应${NC}"
  echo -e "${YELLOW}      手动确认：curl --noproxy '*' http://43.139.149.158:8080/api/server/version${NC}"
fi

# ── 7. 清理工作树（由 trap EXIT 自动执行）────────────────────
echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   ✅ 部署完成！${NC}"
echo -e "${GRAY}   SHA:    $SHA${NC}"
echo -e "${GRAY}   服务:   http://43.139.149.158:8080/health${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo ""
