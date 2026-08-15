const assert = require('node:assert/strict')
const policy = require('../android/app/src/main/assets/google_web_answer_candidate_policy.js')

assert.equal(policy.accepts({
  hasQuery: true,
  textLength: 42,
  citations: 0,
  semanticBlocks: 0,
  links: 9,
  tabControls: 9,
  explicit: false,
}), false)

assert.equal(policy.accepts({
  hasQuery: true,
  textLength: 18,
  citations: 0,
  semanticBlocks: 0,
  links: 0,
  tabControls: 0,
  explicit: true,
}), true)

assert.equal(policy.accepts({
  hasQuery: true,
  textLength: 120,
  citations: 0,
  semanticBlocks: 2,
  links: 0,
  tabControls: 0,
  explicit: false,
}), true)

assert.equal(policy.accepts({
  hasQuery: false,
  textLength: 240,
  citations: 2,
  semanticBlocks: 4,
  links: 2,
  tabControls: 0,
  explicit: true,
}), false)

assert.equal(policy.navigationOnlyText(
  'AI 模式\n全部\n图片\n视频\n新闻\n地图\n购物\n图书\n航班\n财经',
), true)

assert.equal(policy.navigationOnlyText(
  'AI Mode All Images Videos News Maps Shopping Books Flights Finance',
), true)

assert.equal(policy.navigationOnlyText(
  '你可以在 AI 模式里查看新闻和地图，下面是详细回答。',
), false)

assert.equal(policy.transientStatusText('AI 模式回答已准备就绪'), true)
assert.equal(policy.transientStatusText('AI Mode answer is ready'), true)
assert.equal(policy.transientStatusText('正在生成回答…'), true)
assert.equal(policy.transientStatusText('下面是已经准备好的实际回答。'), false)

assert.equal(policy.accepts({
  hasQuery: true,
  text: 'AI 模式回答已准备就绪',
  textLength: 12,
  citations: 0,
  semanticBlocks: 0,
  links: 0,
  tabControls: 0,
  liveRegion: true,
  explicit: true,
}), false)

assert.equal(policy.accepts({
  hasQuery: true,
  text: 'The answer is four.',
  textLength: 19,
  citations: 0,
  semanticBlocks: 1,
  links: 0,
  tabControls: 0,
  liveRegion: true,
  explicit: true,
}), true)

assert.equal(policy.accepts({
  hasQuery: true,
  text: 'AI 模式 全部 图片 视频 新闻 地图 购物 图书 航班 财经',
  textLength: 31,
  citations: 0,
  semanticBlocks: 3,
  links: 0,
  tabControls: 0,
  explicit: true,
}), false)

assert.ok(policy.penalty({ links: 3, tabControls: 2 }) > policy.penalty({ links: 0, tabControls: 0 }))
console.log('google web answer candidate policy passed')
