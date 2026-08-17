const assert = require('node:assert/strict')
const richContent = require('../android/app/src/main/assets/google_web_rich_content.js')

assert.equal(richContent.version, 1)

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

console.log('google web rich content tests passed')
