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

assert.ok(policy.penalty({ links: 3, tabControls: 2 }) > policy.penalty({ links: 0, tabControls: 0 }))
console.log('google web answer candidate policy passed')
