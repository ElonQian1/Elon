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
console.log('google web send policy passed')
