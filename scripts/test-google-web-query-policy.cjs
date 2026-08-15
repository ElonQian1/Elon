const assert = require('node:assert/strict')
const policy = require('../android/app/src/main/assets/google_web_query_policy.js')

assert.equal(policy.select({
  explicitQuery: 'latest visible follow-up',
  rememberedQuery: 'native follow-up',
  urlQuery: 'initial query',
}), 'latest visible follow-up')
assert.equal(policy.select({
  rememberedQuery: 'native follow-up',
  urlQuery: 'initial query',
}), 'initial query')
assert.equal(policy.select({
  explicitQuery: 'stale visible query',
  rememberedQuery: 'native follow-up',
  rememberedOwned: true,
  urlQuery: 'stale url query',
}), 'native follow-up')
assert.equal(policy.select({ urlQuery: 'initial query' }), 'initial query')
assert.equal(policy.select(null), '')
assert.equal(policy.version, 2)

console.log('google web query policy passed')
