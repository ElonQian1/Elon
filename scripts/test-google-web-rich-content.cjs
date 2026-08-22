const assert = require('node:assert/strict')
const richContent = require('../android/app/src/main/assets/google_web_rich_content.js')

assert.equal(richContent.version, 2)

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
