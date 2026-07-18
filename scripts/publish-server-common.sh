#!/usr/bin/env bash

is_local_server_deploy() {
  case "${ELON_DEPLOY_LOCAL:-auto}" in
    1|true|TRUE|local|LOCAL) return 0 ;;
    0|false|FALSE|remote|REMOTE) return 1 ;;
  esac
  [ -d "$REMOTE_DIR" ] && [ -d "$REMOTE_DIR/server" ] && [ -w "$REMOTE_DIR" ] && command -v systemctl >/dev/null 2>&1
}
