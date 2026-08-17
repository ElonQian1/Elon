const assert = require('node:assert/strict')
const policy = require('../android/app/src/main/assets/google_web_answer_candidate_policy.js')

assert.equal(policy.version, 12)

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
  text: '5',
  textLength: 1,
  citations: 0,
  semanticBlocks: 0,
  controls: 0,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: false,
  explicit: false,
}), true)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '5',
  textLength: 1,
  citations: 0,
  semanticBlocks: 0,
  controls: 5,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: false,
  explicit: true,
}), true)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '5',
  textLength: 1,
  citations: 0,
  semanticBlocks: 0,
  controls: 5,
  links: 0,
  tabControls: 0,
  liveRegion: true,
  afterQuery: false,
  trustedAnswerContainer: true,
  interactive: false,
  explicit: true,
}), true)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '5',
  textLength: 1,
  citations: 0,
  semanticBlocks: 0,
  controls: 5,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: false,
  trustedAnswerContainer: true,
  interactive: false,
  explicit: true,
}), true)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '5',
  textLength: 1,
  citations: 0,
  semanticBlocks: 0,
  controls: 1,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: false,
  explicit: false,
}), false)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '5',
  textLength: 1,
  citations: 0,
  semanticBlocks: 0,
  controls: 0,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: false,
  interactive: false,
  explicit: true,
}), false)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '复制',
  textLength: 2,
  citations: 0,
  semanticBlocks: 0,
  controls: 0,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: true,
  explicit: true,
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
  hasQuery: true,
  text: '彰化縣- 縣市預報 | 交通部中央氣象署 天氣...',
  textLength: 38,
  citations: 0,
  semanticBlocks: 1,
  links: 1,
  tabControls: 0,
  afterQuery: true,
  resultListItem: true,
  explicit: true,
}), false, 'organic result cards must not be rendered as the AI answer')

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
assert.equal(policy.transientStatusText('Searching...'), true)
assert.equal(policy.transientStatusText('正在生成回答…'), true)
assert.equal(policy.transientStatusText('正在搜索'), true)
assert.equal(policy.transientStatusText('下面是已经准备好的实际回答。'), false)
assert.equal(policy.shareSurfaceText(
  '分享公开链接 此公开链接用于分享消息串。复制链接 Facebook Gmail X Reddit WhatsApp',
), true)
assert.equal(policy.shareSurfaceText('下面是关于公开链接安全性的完整回答。'), false)
assert.equal(policy.disclosureOnlyText('收起全部显示'), true)
assert.equal(policy.disclosureOnlyText('全部显示'), true)
assert.equal(policy.disclosureOnlyText('Show all'), true)
assert.equal(policy.disclosureOnlyText('下面给出完整回答，并说明如何展开全部内容。'), false)

const signedOutHistoryChrome =
  '打开边栏 新话题 管理 AI 模式 共享的公开链接 查看我的 AI 模式历史记录 ' +
  '设置 新对话 搜索消息串 关闭边栏 AI 模式历史记录 您已退出账号 ' +
  '若要访问历史记录和共享其他好处，请登录您的账号'
assert.equal(policy.pageChromeText(signedOutHistoryChrome), true)
assert.equal(policy.accepts({
  hasQuery: true,
  text: signedOutHistoryChrome,
  textLength: signedOutHistoryChrome.length,
  citations: 0,
  semanticBlocks: 1,
  controls: 3,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: false,
  explicit: true,
}), false)

assert.equal(policy.accepts({
  hasQuery: true,
  text: '收起全部显示',
  textLength: 6,
  citations: 0,
  semanticBlocks: 0,
  controls: 0,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: false,
  explicit: true,
}), false)

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
  text: '正在搜索',
  textLength: 4,
  citations: 0,
  semanticBlocks: 0,
  controls: 0,
  links: 0,
  tabControls: 0,
  liveRegion: false,
  afterQuery: true,
  interactive: false,
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
  text: 'Share a public link. Copy link Facebook Gmail Reddit WhatsApp',
  textLength: 61,
  citations: 5,
  semanticBlocks: 2,
  links: 5,
  tabControls: 0,
  liveRegion: false,
  explicit: true,
}), false)

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

assert.equal(policy.select([
  { id: 'complete-answer', afterQuery: true, trustedAnswerContainer: true, domOrder: 2, score: 9000, textLength: 900 },
  { id: 'last-sentence', afterQuery: true, trustedAnswerContainer: true, domOrder: 5, score: 20, textLength: 22 },
  { id: 'organic-result', afterQuery: true, trustedAnswerContainer: true, domOrder: 9, score: 400, textLength: 80 },
]).id, 'complete-answer', 'the complete AI response must win over later leaf text and result cards')
assert.equal(policy.select([
  { id: 'later-unrelated', domOrder: 9, score: 9000, textLength: 900 },
  { id: 'current-answer', afterQuery: true, domOrder: 4, score: 20, textLength: 2 },
]).id, 'current-answer')
console.log('google web answer candidate policy passed')
