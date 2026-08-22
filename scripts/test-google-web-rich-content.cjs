const assert = require('node:assert/strict')
const richContent = require('../android/app/src/main/assets/google_web_rich_content.js')

assert.equal(richContent.version, 5)

const markdown = richContent.renderBlocks([
  { type: 'heading', level: 2, text: '天气概览' },
  { type: 'paragraph', text: '今天 **适合** 出行。' },
  { type: 'list', ordered: false, items: ['带伞', '注意防晒'] },
  {
    type: 'table',
    rows: [
      ['时段', '天气'],
      ['上午', '晴'],
      ['下午', '阵雨'],
    ],
  },
  { type: 'code', language: 'js', text: 'const ok = true;' },
])

assert.match(markdown, /^## 天气概览/m)
assert.ok(markdown.includes('今天 \\*\\*适合\\*\\* 出行。'))
assert.match(markdown, /^- 带伞/m)
assert.match(markdown, /\| 时段 \| 天气 \|/)
assert.match(markdown, /\| --- \| --- \|/)
assert.match(markdown, /```js\nconst ok = true;\n```/)

const parts = richContent.partsFromBlocks([
  { type: 'paragraph', text: '问题本身' },
  { type: 'heading', level: 3, text: '回答' },
  { type: 'paragraph', text: '完整内容' },
  { type: 'heading', level: 3, text: 'Related results' },
  { type: 'paragraph', text: '不应进入回答' },
], '回退内容', '问题本身')

assert.deepEqual(parts, [{
  type: 'markdown',
  text: '### 回答\n\n完整内容',
}])

assert.deepEqual(
  richContent.partsFromBlocks([], '普通回退内容', ''),
  [{ type: 'text', text: '普通回退内容' }],
)

const sourceRailPruned = richContent.partsFromBlocks([
  { type: 'heading', level: 2, text: '主回答' },
  {
    type: 'list',
    ordered: true,
    items: [
      { text: '正文第一点，并附少量行内引用' },
      { text: '正文第二点' },
    ],
  },
  {
    type: 'list',
    ordered: false,
    sourceCollection: true,
    items: [
      { text: 'YouTube 来源标题与摘要' },
      { text: 'Yahoo 来源标题与摘要' },
      { text: '来源网页 Table_content: | 时间 | 指数 |' },
    ],
  },
], '', '')
assert.equal(sourceRailPruned.length, 1)
assert.match(sourceRailPruned[0].text, /正文第一点/)
assert.match(sourceRailPruned[0].text, /正文第二点/)
assert.doesNotMatch(sourceRailPruned[0].text, /YouTube|Yahoo|Table_content/)

assert.equal(richContent.sourceResultCollection({
  itemCount: 3,
  sourceItemCount: 3,
  dominantSourceItemCount: 3,
  textLength: 540,
}), true)
assert.equal(richContent.sourceResultCollection({
  itemCount: 3,
  sourceItemCount: 1,
  dominantSourceItemCount: 1,
  textLength: 540,
}), false, 'a narrative list with one inline citation must remain in the answer')
assert.equal(richContent.sourceResultCollection({
  itemCount: 3,
  sourceItemCount: 0,
  dominantSourceItemCount: 0,
  textLength: 640,
  railBoundary: true,
}), true, 'Google source rail boundary must survive wrapped or indirect result links')
assert.equal(richContent.sourceResultRailBoundary({
  nextElementSibling: {
    innerText: '显示所有相关结果',
    nextElementSibling: null,
    querySelectorAll: () => [],
  },
}), true)
assert.equal(richContent.sourceResultRailBoundary({
  nextElementSibling: {
    innerText: '继续阅读正文',
    nextElementSibling: null,
    querySelectorAll: () => [],
  },
}), false)
const showAllResults = { innerText: '显示所有相关结果' }
const narrativeList = {
  compareDocumentPosition: (node) => node === showAllResults ? 4 : 0,
}
const wrappedSourceList = {
  nextElementSibling: null,
  compareDocumentPosition: (node) => node === showAllResults ? 4 : 0,
}
const answerScope = {
  parentElement: null,
  querySelectorAll: (selector) => selector.startsWith('button')
    ? [showAllResults]
    : [narrativeList, wrappedSourceList],
}
wrappedSourceList.parentElement = answerScope
assert.equal(richContent.sourceResultRailBoundary(wrappedSourceList), true)
assert.equal(richContent.sourceResultRailBoundary({
  nextElementSibling: null,
  parentElement: {
    parentElement: null,
    querySelectorAll: answerScope.querySelectorAll,
  },
  compareDocumentPosition: narrativeList.compareDocumentPosition,
}), false, 'only the nearest list before the source-rail boundary may be pruned')

const weather = richContent.partsFromBlocks([
  { type: 'heading', level: 2, text: '彰化县今晚天气' },
  { type: 'paragraph', text: '晚间逐渐转晴。' },
  {
    type: 'table',
    rows: [
      ['时间', '天气状况', '气温', '降雨概率'],
      ['17:00', '多云时阴', '33°C', '20%'],
      ['18:00', '阴天', '32°C', '10%'],
    ],
  },
], '', '')
assert.equal(weather.length, 2)
assert.equal(weather[0].type, 'markdown')
assert.equal(weather[1].type, 'rich_card')
assert.equal(weather[1].richContent.schema, 'yilong.rich-content.v1')
assert.equal(weather[1].richContent.kind, 'weather')
assert.equal(weather[1].richContent.payload.rows[0].temperature, '33°C')
assert.ok(!weather[0].text.includes('| 时间 |'), 'the native weather card must replace the duplicate markdown table')

console.log('google web rich content tests passed')
