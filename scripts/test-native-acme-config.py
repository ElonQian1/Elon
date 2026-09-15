"""Exercise the deployment script's real rollback against isolated environment files."""
import pathlib
import subprocess
import sys
import tempfile

source = (pathlib.Path(__file__).parent / 'configure-account-native-acme.sh').read_text(encoding='utf-8')
marker = 'if ! python3 - "$ENV_FILE" "$BACKUP" "$MODE" "$IP" "$ROOT" <<\'PY\'\n'
rollback = source.split(marker, 1)[1].split('\nPY\n', 1)[0]
with tempfile.TemporaryDirectory(prefix='native-acme-config-') as root:
    directory = pathlib.Path(root)
    env, backup = directory / '.env', directory / 'backup'
    backup.write_text('SECRET_KEY=fixture-only\nOTHER=old\nACCOUNT_ACME_ENABLED=false\n', encoding='utf-8')
    env.write_text('SECRET_KEY=fixture-only\nOTHER=new-concurrent-value\nACCOUNT_ACME_ENABLED=true\nACCOUNT_ACME_STAGING=true\nACCOUNT_ACME_IP=43.139.149.158\nACCOUNT_ACME_DATA_DIR=concurrently-changed-directory\n', encoding='utf-8')
    result = subprocess.run([sys.executable, '-', str(env), str(backup), 'staging', '43.139.149.158', '/fixture'], input=rollback, text=True, capture_output=True)
    assert result.returncode == 0, result.stderr
    content = env.read_text(encoding='utf-8')
    assert 'SECRET_KEY=fixture-only' in content
    assert 'OTHER=new-concurrent-value' in content
    assert 'ACCOUNT_ACME_ENABLED=false' in content
    assert 'ACCOUNT_ACME_STAGING=' not in content
    assert 'ACCOUNT_ACME_IP=' not in content
    assert 'ACCOUNT_ACME_DATA_DIR=concurrently-changed-directory' in content
    print('NATIVE_ACME_ROLLBACK=passed (targeted settings, concurrent edits, absent original keys)')
