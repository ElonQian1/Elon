const assert = require('node:assert/strict')
const policy = require('../android/app/src/main/assets/google_web_send_policy.js')

assert.deepEqual(policy.reconcile('', '', 'hello'), {
  allowed: true,
  write: true,
  staged: false,
})
assert.deepEqual(policy.reconcile('hello', '', 'hello'), {
  allowed: true,
  write: false,
  staged: true,
})
assert.deepEqual(policy.reconcile('private draft', '', 'hello'), {
  allowed: false,
  write: false,
  staged: false,
})
assert.equal(policy.confirmed({ currentDraft: 'hello', prompt: 'hello' }), false)
assert.equal(policy.confirmed({ currentDraft: '', prompt: 'hello' }), true)
assert.equal(policy.confirmed({ currentDraft: 'hello', prompt: 'hello', streaming: true }), true)
assert.equal(policy.confirmed({ currentDraft: 'hello', prompt: 'hello', queryMatches: true }), true)
assert.equal(policy.version, 2)
assert.equal(policy.latestUserQueryMatches([
  { role: 'user', content: [{ type: 'text', text: 'first prompt' }] },
  { role: 'assistant', content: [{ type: 'text', text: 'first answer' }] },
  { role: 'user', content: [{ type: 'text', text: 'second prompt' }] },
], 'second prompt'), true)
assert.equal(policy.latestUserQueryMatches([
  { role: 'user', content: [{ type: 'text', text: 'first prompt' }] },
  { role: 'assistant', content: [{ type: 'text', text: 'first answer' }] },
  { role: 'user', content: [{ type: 'text', text: 'second prompt' }] },
], 'first prompt'), false)
assert.equal(policy.latestUserQueryMatches([
  { role: 'user', content: [{ type: 'text', text: '  multi\u00a0space  ' }] },
], 'multi space'), true)
assert.equal(policy.latestUserQueryMatches([], 'missing'), false)
console.log('google web send policy passed')
