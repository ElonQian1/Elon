const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const path = require('node:path');

test('Windows Gradle wrapper propagates a real Java launch failure', { skip: process.platform !== 'win32' }, () => {
  // The deliberately unsupported JVM option fails before Gradle downloads, compiles or tests anything.
  const result = spawnSync(process.env.ComSpec || 'cmd.exe', ['/d', '/s', '/c', 'gradlew.bat --version'], {
    cwd: path.join(__dirname, '..', 'android'), encoding: 'utf8', timeout: 30000, windowsHide: true,
    env: { ...process.env, GRADLE_EXIT_CONSOLE: '', JAVA_TOOL_OPTIONS: '-XX:MobileDesignWrapperIntentionalInvalidOption' },
  });
  assert.ifError(result.error);
  assert.match(result.stderr, /Unrecognized VM option.*MobileDesignWrapperIntentionalInvalidOption/);
  assert.notEqual(result.status, 0, 'A failed JVM must fail the CI step, not fall through endlocal as success');
});
