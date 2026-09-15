#!/usr/bin/env bash
# Configure the server's Rust ACME lifecycle. Installs no certificate software.
set -euo pipefail
unset HTTP_PROXY HTTPS_PROXY ALL_PROXY http_proxy https_proxy all_proxy
MODE="${1:-plan}"; IP="${2:-43.139.149.158}"
ROOT=/var/lib/elon-account-native-acme
ENV_FILE=/root/Elon/server/.env
SERVICE=elon-server
fail() { printf 'NATIVE_ACME_ERROR=%s\n' "$*" >&2; exit 1; }
[[ $# -le 2 && "$MODE" =~ ^(plan|status|staging|production|disable)$ ]] || fail 'usage: configure-account-native-acme.sh plan|status|staging|production|disable [public-ipv4]'
python3 - "$IP" <<'PY'
import ipaddress,sys
ip=ipaddress.IPv4Address(sys.argv[1])
if not ip.is_global or ip.is_multicast or ip.is_reserved: raise SystemExit('NATIVE_ACME_ERROR=public IPv4 required')
PY
ORIGIN="https://$IP:8443"
if [[ "$MODE" == plan ]]; then
  printf 'NATIVE_ACME_IMPLEMENTATION=Rust instant-acme + rustls\nNATIVE_ACME_CHALLENGE=tls-alpn-01:443\nNATIVE_ACME_HTTPS=%s\nNATIVE_ACME_MUTATED=false\n' "$ORIGIN"
  exit
fi
if [[ "$MODE" == status ]]; then
  python3 - "$ROOT" "$IP" <<'PY'
import json,pathlib,sys
for mode in ('staging','production'):
    p=pathlib.Path(sys.argv[1])/mode/sys.argv[2]/'status.json'
    d=json.loads(p.read_text()) if p.exists() else {'result':'not_configured'}
    print(json.dumps({'environment':mode,**{k:d.get(k) for k in ('result','expires_at','next_attempt','failures','last_error')}},separators=(',',':')))
PY
  exit
fi
[[ $EUID -eq 0 && -f "$ENV_FILE" ]] || fail 'root and existing service environment required'
[[ "$(systemctl show "$SERVICE" -p User --value)" == root ]] || fail 'review service user permissions before activation'
[[ ! -L "$ROOT" ]] || fail 'state directory cannot be a symlink'
umask 077
install -d -m 0700 -o root -g root "$ROOT"
exec 9>"$ROOT/configure.lock"
flock -w 5 9 || fail 'another native ACME configuration is running'
for unit in elon-account-acme-renew.service elon-account-cert-renew.service; do
  if systemctl is-active --quiet "$unit"; then fail 'legacy certificate renewal is running; retry after it finishes'; fi
done
curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" >/dev/null
[[ "$(curl --silent --show-error --max-time 10 -o /dev/null -w '%{http_code}' "$ORIGIN/square")" == 200 ]] || fail 'deploy native ACME and Square server code first'
BACKUP="$(mktemp /root/Elon/server/.native-acme-env-backup.XXXXXXXX)"
cp -p "$ENV_FILE" "$BACKUP"; chmod 0600 "$BACKUP"
ACTIVE_TIMERS=(); TIMERS_CHANGED=false
for timer in elon-account-acme-renew.timer elon-account-cert-renew.timer; do
  if systemctl is-enabled --quiet "$timer"; then ACTIVE_TIMERS+=("$timer"); fi
done
rollback() {
  local code=$?; trap - EXIT
  if [[ $code -ne 0 ]]; then
    # Restore only this operation's settings; preserve concurrent unrelated edits.
    if ! python3 - "$ENV_FILE" "$BACKUP" "$MODE" "$IP" "$ROOT" <<'PY'
import os,pathlib,sys,tempfile
p,backup=map(pathlib.Path,sys.argv[1:3]);mode,ip,root=sys.argv[3:]
expected={'ACCOUNT_ACME_ENABLED':'false' if mode=='disable' else 'true','ACCOUNT_ACME_STAGING':'true' if mode=='staging' else 'false'}
if mode!='disable': expected.update(ACCOUNT_ACME_ACCEPT_TOS='true',ACCOUNT_ACME_IP=ip,ACCOUNT_ACME_LISTEN_ADDR='0.0.0.0:443',ACCOUNT_ACME_DATA_DIR=root)
def entries(lines): return {k.strip():v for s in lines for k,sep,v in [s.partition('=')] if sep}
lines=p.read_text().splitlines();current=entries(lines);before=entries(backup.read_text().splitlines())
restore={k for k,v in expected.items() if current.get(k)==v}
lines=[s for s in lines if s.partition('=')[0].strip() not in restore]
lines += [f'{k}={before[k]}' for k in sorted(restore) if k in before]
fd,tmp=tempfile.mkstemp(prefix='.native-acme-rollback-',dir=p.parent)
try:
    with os.fdopen(fd,'w') as out: out.write('\n'.join(lines)+'\n');out.flush();os.fsync(out.fileno())
    os.chmod(tmp,0o600);os.replace(tmp,p)
finally:
    if os.path.exists(tmp): os.unlink(tmp)
PY
    then
      printf 'NATIVE_ACME_STATUS=rollback_failed_private_backup_retained\n' >&2
      exit "$code"
    fi
    systemctl restart "$SERVICE" || true
    if [[ "$TIMERS_CHANGED" == true ]]; then
      for timer in elon-account-acme-renew.timer elon-account-cert-renew.timer; do
        if [[ " ${ACTIVE_TIMERS[*]} " == *" $timer "* ]]; then systemctl enable --now "$timer" || true;
        elif systemctl list-unit-files "$timer" --no-legend | grep -q "$timer"; then systemctl disable --now "$timer" || true; fi
      done
    fi
    printf 'NATIVE_ACME_STATUS=activation_failed_previous_environment_restored\n' >&2
  fi
  rm -f -- "$BACKUP"; exit "$code"
}
trap rollback EXIT
if [[ ! -e "$ROOT/legacy-timers" ]]; then
  printf '%s\n' "${ACTIVE_TIMERS[@]}" >"$ROOT/legacy-timers"
fi
python3 - "$ENV_FILE" "$MODE" "$IP" "$ROOT" <<'PY'
import os,pathlib,sys,tempfile
p=pathlib.Path(sys.argv[1]);mode,ip,root=sys.argv[2:]
updates={'ACCOUNT_ACME_ENABLED':'false' if mode=='disable' else 'true','ACCOUNT_ACME_STAGING':'true' if mode=='staging' else 'false'}
if mode!='disable': updates.update(ACCOUNT_ACME_ACCEPT_TOS='true',ACCOUNT_ACME_IP=ip,ACCOUNT_ACME_LISTEN_ADDR='0.0.0.0:443',ACCOUNT_ACME_DATA_DIR=root)
lines=[s for s in p.read_text().splitlines() if s.partition('=')[0].strip() not in updates]
lines += [f'{k}={v}' for k,v in updates.items()]
fd,tmp=tempfile.mkstemp(prefix='.native-acme-env-',dir=p.parent)
try:
    with os.fdopen(fd,'w') as out: out.write('\n'.join(lines)+'\n');out.flush();os.fsync(out.fileno())
    os.chmod(tmp,0o600);os.replace(tmp,p)
finally:
    if os.path.exists(tmp): os.unlink(tmp)
PY
systemctl restart "$SERVICE"
for attempt in {1..30}; do if curl --silent --fail --max-time 2 "$ORIGIN/health" >/dev/null; then break; fi; sleep 1; done
curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" >/dev/null
if [[ "$MODE" != disable ]]; then
  STATE="$ROOT/$MODE/$IP/status.json"
  for attempt in {1..140}; do
    if python3 - "$STATE" <<'PY'
import json,pathlib,sys,time
p=pathlib.Path(sys.argv[1]);d=json.loads(p.read_text()) if p.exists() else {}
raise SystemExit(0 if d.get('result')=='renewed' and (d.get('expires_at') or 0)>time.time()+48*3600 else 1)
PY
    then break; fi
    sleep 5
  done
  python3 - "$STATE" <<'PY'
import json,pathlib,sys,time
d=json.loads(pathlib.Path(sys.argv[1]).read_text())
if d.get('result')!='renewed' or (d.get('expires_at') or 0)<=time.time()+48*3600: raise SystemExit('NATIVE_ACME_ERROR=issuance not verified; inspect sanitized status and retry time')
print('NATIVE_ACME_ISSUANCE=verified expires_at='+str(d['expires_at']))
PY
fi
if [[ "$MODE" == production ]]; then
  # Read the peer certificate with the standard TLS verifier; never disable trust.
  python3 - "$IP" "$ROOT/production/$IP/active.pem" <<'PY'
import hashlib,pathlib,re,socket,ssl,sys,time
ip,p=sys.argv[1:];pem=re.search(r'-----BEGIN CERTIFICATE-----.*?-----END CERTIFICATE-----',pathlib.Path(p).read_text(),re.S).group()
expected=hashlib.sha256(ssl.PEM_cert_to_DER_cert(pem)).hexdigest()
context=ssl.create_default_context()
for attempt in range(18):
    with socket.create_connection((ip,8443),timeout=10) as raw:
        with context.wrap_socket(raw,server_hostname=ip) as tls: actual=hashlib.sha256(tls.getpeercert(binary_form=True)).hexdigest()
    if actual==expected: print('NATIVE_ACME_SERVED_CERTIFICATE=verified sha256='+actual);break
    time.sleep(5)
else: raise SystemExit('NATIVE_ACME_ERROR=HTTPS has not loaded the newly issued pair')
PY
  TIMERS_CHANGED=true
  for timer in elon-account-acme-renew.timer elon-account-cert-renew.timer; do
    if systemctl list-unit-files "$timer" --no-legend | grep -q "$timer"; then systemctl disable --now "$timer"; fi
  done
elif [[ "$MODE" == disable ]]; then
  TIMERS_CHANGED=true
  while IFS= read -r timer; do
    case "$timer" in elon-account-acme-renew.timer|elon-account-cert-renew.timer) systemctl enable --now "$timer";; esac
  done <"$ROOT/legacy-timers"
fi
curl --silent --show-error --fail --max-time 10 "$ORIGIN/health" >/dev/null
curl --silent --show-error --fail --max-time 10 http://127.0.0.1:8080/health >/dev/null
printf 'NATIVE_ACME_STATUS=%s_verified\n' "$MODE"
