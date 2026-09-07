#!/usr/bin/env bash
# Install a checksum-pinned ACME client and its operator scripts; never issue or activate TLS.
set -euo pipefail
umask 077
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
VERSION=5.4.1
ARCHIVE_SHA256=ebb33f1bead5a7c99dd46f1c5734b44cf1eab5b5c12faf397cd14d50a5916419
BASE=/opt/elon-account-acme
STATE_ROOT=/var/lib/elon-account-acme
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
fail() { printf 'ACCOUNT_ACME_INSTALL_ERROR=%s\n' "$*" >&2; exit 1; }
[[ $EUID -eq 0 && "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] ||
  fail "Linux x86_64 root required"
[[ $# -eq 0 || ( $# -eq 2 && "$1" == --archive ) ]] || fail "usage: $0 [--archive verified-local-tar-gz]"
for source in configure-account-acme.sh configure-account-https.sh account_https_target.py; do
  [[ -f "$SCRIPT_DIR/$source" ]] || fail "operator script missing: $source"
done
temporary="$(mktemp -d /tmp/elon-account-acme-install.XXXXXXXX)"
cleanup() { rm -f -- "$temporary/lego.tar.gz" "$temporary/lego"; rmdir -- "$temporary"; }
trap cleanup EXIT
if [[ $# -eq 2 ]]; then
  [[ "$2" == /* && -f "$2" ]] || fail "archive must be an absolute regular file path"
  cp -- "$2" "$temporary/lego.tar.gz"
else
  curl --noproxy '*' --proto '=https' --proto-redir '=https' --fail --location \
    --connect-timeout 15 --max-time 300 --retry 1 \
    "https://github.com/go-acme/lego/releases/download/v$VERSION/lego_v${VERSION}_linux_amd64.tar.gz" \
    -o "$temporary/lego.tar.gz"
fi
actual="$(sha256sum "$temporary/lego.tar.gz" | cut -d ' ' -f 1)"
[[ "$actual" == "$ARCHIVE_SHA256" ]] || fail "archive checksum mismatch"
tar -xzf "$temporary/lego.tar.gz" -C "$temporary" --no-same-owner --no-same-permissions lego
[[ -f "$temporary/lego" && ! -L "$temporary/lego" ]] || fail "archive did not contain a regular lego executable"
chmod 0700 "$temporary/lego"
"$temporary/lego" --version | grep -qF "lego version $VERSION" || fail "client version mismatch"
for directory in "$BASE" "$BASE/lego-v$VERSION" "$BASE/scripts" "$STATE_ROOT"; do
  [[ ! -L "$directory" ]] || fail "installation directory cannot be a symlink"
  if [[ -e "$directory" ]]; then
    [[ -d "$directory" && "$(stat -c %u "$directory")" == 0 ]] || fail "installation directory must be root-owned"
  fi
  install -d -m 0700 -o root -g root "$directory"
done
lock="$STATE_ROOT/operation.lock"
[[ ! -L "$lock" ]] || fail "operation lock cannot be a symlink"
if [[ -e "$lock" ]]; then
  [[ -f "$lock" && "$(stat -c %u "$lock")" == 0 ]] || fail "invalid operation lock"
fi
exec 9>>"$lock"
chmod 0600 "$lock"
flock -w 10 9 || fail "another certificate operation or installation is active"
if [[ -e "$BASE/lego-v$VERSION/lego" ]]; then
  [[ ! -L "$BASE/lego-v$VERSION/lego" ]] || fail "installed executable cannot be a symlink"
  cmp -s "$temporary/lego" "$BASE/lego-v$VERSION/lego" || fail "existing client differs; preserve and investigate"
else
  install -m 0700 "$temporary/lego" "$BASE/lego-v$VERSION/lego"
fi
for source in configure-account-acme.sh configure-account-https.sh account_https_target.py; do
  [[ ! -L "$BASE/scripts/$source" ]] || fail "installed operator script cannot be a symlink"
  install -m 0700 "$SCRIPT_DIR/$source" "$BASE/scripts/$source"
done
printf 'ACCOUNT_ACME_INSTALL_STATUS=ready version=%s sha256=%s issuance_performed=false\n' "$VERSION" "$ARCHIVE_SHA256"
