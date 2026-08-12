'use strict';

const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_attachment_policy.js');

[
  '移除文件1：fixture.txt',
  '删除附件 2: photo.png',
  'Remove file 1: fixture.txt',
  'Delete attachment 2 - photo.png'
].forEach((label) => assert.equal(policy.isRemoveActionLabel(label), true, label));

[
  '关闭搜索',
  '删除聊天',
  'Remove search',
  'Download file'
].forEach((label) => assert.equal(policy.isRemoveActionLabel(label), false, label));

assert.equal(policy.withoutRemoveAction('移除文件1：fixture.txt'), 'fixture.txt');
assert.equal(policy.withoutRemoveAction('Remove file 2: fixture.txt'), 'fixture.txt');

let clickCount = 0;
const connected = { isConnected: true, click() { clickCount += 1; } };
assert.equal(policy.invokeRemoveAction(connected, '移除文件1：fixture.txt'), true);
assert.equal(clickCount, 1);
assert.equal(policy.invokeRemoveAction(connected, '删除聊天'), false);
assert.equal(clickCount, 1);
assert.equal(policy.invokeRemoveAction({ isConnected: false, click() {} }, 'Remove file 1'), false);

process.stdout.write('CHATGPT_ATTACHMENT_POLICY=passed\n');
