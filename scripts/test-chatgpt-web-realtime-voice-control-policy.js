'use strict';

const assert = require('node:assert/strict');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_realtime_voice_policy.js'
);

assert.equal(policy.classify('Start voice mode'), 'voice_mode');
assert.equal(policy.classify('Mute microphone'), 'voice_mute');
assert.equal(policy.classify('Unmute microphone'), 'voice_unmute');
assert.equal(policy.classify('关闭麦克风'), 'voice_mute');
assert.equal(policy.classify('打开麦克风'), 'voice_unmute');
assert.equal(policy.classify('启动语音功能'), 'voice_mode');
assert.equal(policy.classify('Exit voice mode'), 'close');
assert.equal(policy.classify('Close voice call'), 'close');
assert.equal(policy.classify('结束语音'), 'close');
assert.equal(policy.classify('挂断通话'), 'close');
assert.equal(policy.classify('Stop generating'), '');

process.stdout.write('CHATGPT_REALTIME_VOICE_CONTROL_POLICY=passed\n');
