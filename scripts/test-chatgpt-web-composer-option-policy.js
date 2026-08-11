'use strict';

const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_composer_option_policy.js');

function option(label, role = 'menuitem', selectable = false) {
  return { label, role, selectable };
}

assert.deepEqual(policy.filter('model', [
  option('下载 ChatGPT 桌面版'),
  option('下载 ChatGPT 移动版')
]), []);
assert.deepEqual(policy.filter('tools', [
  option('ELon Qian Pro'),
  option('个性化'),
  option('个人资料'),
  option('设置'),
  option('帮助'),
  option('退出登录')
]), []);

assert.deepEqual(policy.filter('model', [
  option('自动', 'menuitemradio'),
  option('5.6 Sol轻度', 'menuitemradio'),
  option('GPT-5 Thinking', 'option'),
  option('能力'),
  option('推理强度 中'),
  option('速度 标准')
]).map((value) => value.label), [
  '自动', '5.6 Sol轻度', 'GPT-5 Thinking', '能力', '推理强度 中', '速度 标准'
]);

assert.deepEqual(policy.filter('tools', [
  option('相机'),
  option('照片'),
  option('文件'),
  option('创建图片'),
  option('网页搜索'),
  option('创建任务'),
  option('Figma'),
  option('GitHub'),
  option('OpenAI Platform')
]).map((value) => value.label), [
  '相机', '照片', '文件', '创建图片', '网页搜索', '创建任务', 'Figma', 'GitHub', 'OpenAI Platform'
]);

assert.equal(policy.accepts('model', option('帮助')), false);
assert.equal(policy.accepts('tools', option('', 'menuitem')), false);

process.stdout.write('CHATGPT_COMPOSER_OPTION_POLICY=passed\n');
