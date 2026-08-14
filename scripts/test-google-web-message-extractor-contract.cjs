const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const source = fs.readFileSync(
  path.join(root, 'android/app/src/main/assets/google_web_message_extractor.js'),
  'utf8',
)

assert.match(source, /const genericSelectors = \['body div'\]/)
assert.match(source, /'\[role="article"\]'/)
assert.match(source, /'body \[aria-live="polite"\]'/)
assert.match(source, /node\.closest\(/)
assert.match(source, /\[role="navigation"\]/)
assert.match(source, /\[role="dialog"\]/)
assert.match(source, /containsComposer\(node, composer\)/)
assert.match(source, /candidatePolicy\.accepts\(metrics\)/)
assert.match(source, /\[role=\"tablist\"\]/)
assert.match(source, /query \|\| location\.pathname === '\/search'/)
assert.doesNotMatch(source, /document\.cookie|Authorization|fetch\(|outerHTML|sessionStorage|localStorage/)

console.log('google web message extractor contract passed')
