"""Bounded device discovery for the shell release entry; no credential transport."""
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from datetime import datetime, timezone


def run(args, timeout=10):
    try:
        return subprocess.run(args, text=True, encoding='utf-8', errors='replace',
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
    except subprocess.TimeoutExpired:
        return subprocess.CompletedProcess(args, 124, '', 'timeout')


def registry():
    source = """import sqlite3,json
c=sqlite3.connect('file:/opt/elon/data/elon.db?mode=ro',uri=True)
r=c.execute('select hardware_serial,display_name,last_endpoint from project_android_devices where project_id=?',('elon-self',)).fetchall()
print(json.dumps({'schemaVersion':1,'enabled':True,'launchAfterInstall':False,'maxAttempts':2,'retryDelaySeconds':2,'targets':[{'hardwareSerial':v[0],'label':v[1],'serial':v[2]} for v in r]}))
"""
    encoded = base64.b64encode(source.encode()).decode()
    result = run(['ssh', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', '-o', 'ProxyCommand=none',
                  '-o', 'ProxyJump=none', 'root@43.139.149.158',
                  f'python3 -c "exec(__import__(\'base64\').b64decode(\'{encoded}\'))"'], 30)
    if result.returncode:
        raise RuntimeError('PROJECT_ADB_REGISTRY_UNAVAILABLE')
    config = json.loads(result.stdout)
    if not config.get('targets'):
        raise RuntimeError('PROJECT_ADB_REGISTRY_EMPTY')
    return config


def resolve(adb, hardware, endpoint, invoke=run):
    if not re.fullmatch(r'[A-Za-z0-9._-]+', hardware):
        raise ValueError('Invalid hardware identity')
    listed = invoke([adb, 'devices', '-l'], 15)
    if listed.returncode:
        raise RuntimeError('ADB_DEVICE_DISCOVERY_FAILED')
    online = []
    for line in listed.stdout.splitlines():
        fields = line.split()
        if len(fields) >= 2 and fields[1] in ('device', 'offline', 'unauthorized') and not fields[0].startswith('emulator-'):
            online.append(fields[0])
    candidates = [hardware] + [s for s in online if ':' not in s] + [endpoint] + online
    mdns = invoke([adb, 'mdns', 'services'], 5)
    for line in mdns.stdout.splitlines():
        match = re.fullmatch(r'adb-' + re.escape(hardware) + r'-\S+\s+_adb-tls-connect\._tcp\.?\s+(\S+)', line.strip())
        if match:
            candidates.append(match[1])
    reason = 'offline'
    for serial in dict.fromkeys(candidates):
        if not re.fullmatch(r'[A-Za-z0-9._:-]+', serial or '') or serial.startswith('emulator-'):
            continue
        if re.search(r':\d+$', serial) and serial not in online:
            invoke([adb, 'connect', serial], 8)
        state = invoke([adb, '-s', serial, 'get-state'], 5)
        if state.returncode or state.stdout.strip() != 'device':
            if 'unauthorized' in state.stderr + state.stdout:
                reason = 'unauthorized'
            elif state.returncode == 124 and serial in online:
                reason = 'probe_failed'
            continue
        identity = invoke([adb, '-s', serial, 'shell', 'getprop', 'ro.serialno'], 5)
        if identity.returncode:
            reason = 'probe_failed'
        elif identity.stdout.strip().lower() == hardware.lower():
            return 'online', serial
        elif serial == endpoint:
            reason = 'identity_mismatch'
    return reason, ''


def receipt(apk, expected, package, results):
    rows = []
    for line in Path(results).read_text().splitlines():
        label, status = line.split('\t', 1)
        rows.append({'Label': label, 'Status': status})
    target = Path.home() / '.elon' / 'apk-adb-receipts'
    target.mkdir(parents=True, exist_ok=True)
    path = target / f'{package}-{expected}-{os.getpid()}.json'
    digest = hashlib.sha256()
    with open(apk, 'rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(block)
    path.write_text(json.dumps(dict(schema='elon.apk_adb_receipt.v1',
        checkedAt=datetime.now(timezone.utc).isoformat(), packageName=package,
        expectedVersionCode=int(expected), apkSha256=digest.hexdigest(), targets=rows), ensure_ascii=False), encoding='utf-8')
    print(f'APK_ADB_RECEIPT={path}')


if __name__ == '__main__':
    try:
        mode, *args = sys.argv[1:]
        if mode == 'registry':
            print(json.dumps(registry()))
        elif mode == 'resolve':
            print('\t'.join(resolve(*args)))
        elif mode == 'receipt':
            receipt(*args)
        elif mode == 'exec':
            result = run(args[1:], int(args[0]))
            print(result.stdout, end='')
            print(result.stderr, file=sys.stderr, end='')
            sys.exit(result.returncode)
        else:
            raise ValueError('Unknown command')
    except (ValueError, OSError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
