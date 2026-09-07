#!/usr/bin/env bash
# Offline checks: only synthetic certificates in an owned temporary directory.
set -euo pipefail
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
CONFIGURE="$SCRIPT_DIR/configure-account-https.sh"
export PYTHONDONTWRITEBYTECODE=1

# Git Bash can use the bundled Windows Python without rewriting POSIX target paths.
if [[ -n "${ELON_TEST_PYTHON:-}" ]]; then
  python3() {
    local script="$1"
    shift
    if [[ "$script" == /* ]]; then script="$(cygpath -w "$script")"; fi
    MSYS2_ARG_CONV_EXCL='*' "$ELON_TEST_PYTHON" "$script" "$@" | sed 's/\r$//'
    return "${PIPESTATUS[0]}"
  }
  export -f python3
fi

test_root="$SCRIPT_DIR/../.ai-tmp"
mkdir -p "$test_root"
temporary="$(mktemp -d "$test_root/account-https-check.XXXXXXXX")"
cleanup() {
  # All generated files are flat and owned by this test; never recurse into workspaces.
  rm -f -- "$temporary"/*.pem "$temporary"/*.csr "$temporary"/*.srl \
    "$temporary"/*.ext "$temporary"/*.json "$temporary"/*.log
  rmdir -- "$temporary"
}
trap cleanup EXIT
passed=0
expect_failure() {
  if "$@" >"$temporary/failure.log" 2>&1; then
    printf 'unexpected success: %s\n' "$1" >&2
    exit 1
  fi
  passed=$((passed + 1))
}

bash -n "$CONFIGURE"
bash "$CONFIGURE" plan 43.139.149.158 >"$temporary/plan.json"
python3 -c 'import json,sys; p=json.load(sys.stdin); assert p["origin"]=="https://43.139.149.158"; assert p["validation_port"]==80; assert p["deployment_mutated"] is False' <"$temporary/plan.json"
passed=$((passed + 1))
bash "$CONFIGURE" plan https://grid.example.com:9443 \
  --certificate-path /etc/elon/chain.pem --private-key-path /etc/elon/key.pem >"$temporary/plan.json"
python3 -c 'import json,sys; p=json.load(sys.stdin); assert p["port"]==9443; assert p["certificate_source"]=="external"; assert p["validation_port"] is None; assert p["certificate_issuance_performed"] is False' <"$temporary/plan.json"
passed=$((passed + 1))
expect_failure bash "$CONFIGURE" plan https://grid.example.com
expect_failure bash "$CONFIGURE" plan http://43.139.149.158:8080
expect_failure bash "$CONFIGURE" plan https://grid.example.com/path \
  --certificate-path /etc/elon/chain.pem --private-key-path /etc/elon/key.pem
expect_failure bash "$CONFIGURE" activate https://43.139.149.158:8080
grep -q 'port 8080 is reserved' "$temporary/failure.log"

export MSYS2_ARG_CONV_EXCL='/CN='
openssl req -x509 -newkey rsa:2048 -nodes -days 3 -subj /CN=Elon-Offline-Test-CA \
  -addext basicConstraints=critical,CA:TRUE -addext keyUsage=critical,keyCertSign,cRLSign \
  -keyout "$temporary/ca-key.pem" -out "$temporary/ca.pem" >/dev/null 2>&1
openssl req -new -newkey rsa:2048 -nodes -subj /CN=grid.example.com \
  -keyout "$temporary/key.pem" -out "$temporary/leaf.csr" >/dev/null 2>&1
cat >"$temporary/leaf.ext" <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:grid.example.com,IP:43.139.149.158
EOF
openssl x509 -req -in "$temporary/leaf.csr" -CA "$temporary/ca.pem" \
  -CAkey "$temporary/ca-key.pem" -CAcreateserial -days 2 -extfile "$temporary/leaf.ext" \
  -out "$temporary/leaf.pem" >/dev/null 2>&1
cat "$temporary/leaf.pem" "$temporary/ca.pem" >"$temporary/chain.pem"

# This trust root exists only inside this offline test process.
export SSL_CERT_FILE="$temporary/ca.pem"
if [[ -n "${ELON_TEST_PYTHON:-}" ]]; then SSL_CERT_FILE="$(cygpath -m "$SSL_CERT_FILE")"; fi
bash "$CONFIGURE" check https://grid.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/key.pem"
passed=$((passed + 1))
bash "$CONFIGURE" check https://43.139.149.158:8443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/key.pem"
passed=$((passed + 1))
expect_failure bash "$CONFIGURE" check https://wrong.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/key.pem"
expect_failure bash "$CONFIGURE" check https://grid.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/ca-key.pem"
openssl pkcs8 -topk8 -in "$temporary/key.pem" -out "$temporary/encrypted.pem" \
  -passout pass: -v2 aes-256-cbc >/dev/null 2>&1
expect_failure bash "$CONFIGURE" check https://grid.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/encrypted.pem"
grep -q 'requires an unencrypted private key PEM' "$temporary/failure.log"
openssl rsa -in "$temporary/key.pem" -traditional -aes256 -passout pass:offline-test \
  -out "$temporary/traditional-encrypted.pem" >/dev/null 2>&1
expect_failure bash "$CONFIGURE" check https://grid.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/traditional-encrypted.pem"
grep -q 'requires an unencrypted private key PEM' "$temporary/failure.log"
expect_failure bash "$CONFIGURE" prepare https://grid.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/key.pem"
grep -q 'external certificate issuance and renewal belong to its provider' "$temporary/failure.log"

openssl x509 -req -in "$temporary/leaf.csr" -CA "$temporary/ca.pem" \
  -CAkey "$temporary/ca-key.pem" -CAcreateserial -days 0 -extfile "$temporary/leaf.ext" \
  -out "$temporary/expired.pem" >/dev/null 2>&1
expect_failure bash "$CONFIGURE" check https://grid.example.com:9443 \
  --certificate-path "$temporary/expired.pem" --private-key-path "$temporary/key.pem"
unset SSL_CERT_FILE
expect_failure bash "$CONFIGURE" check https://grid.example.com:9443 \
  --certificate-path "$temporary/chain.pem" --private-key-path "$temporary/key.pem"
printf 'ACCOUNT_HTTPS_OFFLINE_CHECKS=%s passed; deployment_mutated=false\n' "$passed"
