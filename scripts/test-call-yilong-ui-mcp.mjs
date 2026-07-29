#!/usr/bin/env node

import assert from 'node:assert/strict'
import {
  shouldPollUiVerification,
  withAndroidResume,
} from './call-yilong-ui-mcp.mjs'

const inProgress = {
  status: 'ANDROID_FALLBACK_IN_PROGRESS',
  next: 'POLL_UI_VERIFY_WITH_RESUME_ANDROID_TRUE',
  android: { retryAfterMs: 1000 },
}

assert.equal(
  shouldPollUiVerification('ui_verify_with_fallback', inProgress, 1000, 30000, 2000),
  true,
)
assert.equal(
  shouldPollUiVerification('ui_verify_with_fallback', inProgress, 1000, 30000, 31000),
  false,
)
assert.equal(
  shouldPollUiVerification(
    'ui_verify_with_fallback',
    { status: 'REAL_DEVICE_VERIFICATION_DEFERRED', next: 'STOP_AND_REQUEST_RUNTIME_RECOVERY' },
    1000,
    30000,
    2000,
  ),
  false,
)
assert.equal(
  shouldPollUiVerification('ui_check_workflow_completion', inProgress, 1000, 30000, 2000),
  false,
)

const original = { pwaSuitable: false, android: { taskId: 'task-1' } }
const resumed = withAndroidResume(original)
assert.equal(resumed.resumeAndroid, true)
assert.equal(resumed.android.taskId, 'task-1')
assert.equal(original.resumeAndroid, undefined)

process.stdout.write('PASS call-yilong-ui-mcp session polling\n')
