'use strict';

const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_model_label_policy.js');

for (const label of [
  'GPT-5.6 Sol',
  '5.6 Terra 中',
  '5.5 Pro',
  '6 Pro',
  'Pro',
  '极速',
  '自动',
  '快速',
  '思考',
  '低',
  '中',
  '高',
  '高 高',
  '思考强度 极高',
  '模型 Auto'
]) {
  assert.equal(policy.isModelLabel(label), true, `expected model label: ${label}`);
}

assert.equal(policy.isModelControl({
  label: '高',
  signal: 'composer-disclosure-trigger unrelated-node-metadata 高 高'
}), true, 'visible compact model label wins over noisy node metadata');

assert.equal(policy.isModelControl({
  label: '工具',
  signal: 'composer-tool-trigger 工具'
}), false, 'unrelated composer control remains non-model');

for (const label of [
  '',
  'Workspace',
  'Profile',
  '启动语音功能',
  '添加附件',
  '发送'
]) {
  assert.equal(policy.isModelLabel(label), false, `expected non-model label: ${label}`);
}

console.log('CHATGPT_WEB_MODEL_LABEL_POLICY=passed');
