const assert = require('node:assert/strict')
const bridge = require('../android/app/src/main/assets/google_web_composer_bridge.js')

const base = {
  visible: true,
  disabled: false,
  inNavigation: false,
  contentEditable: false,
  role: '',
  type: '',
  formOwned: false,
}

const aiComposer = bridge.scoreMeta({
  ...base,
  tag: 'textarea',
  positiveLabel: true,
  bottomHalf: true,
})
const topSearch = bridge.scoreMeta({
  ...base,
  tag: 'input',
  type: 'search',
  role: 'searchbox',
  positiveLabel: false,
  bottomHalf: false,
})

assert.ok(aiComposer > topSearch)
assert.equal(bridge.scoreMeta({ ...base, tag: 'textarea', inNavigation: true }), -1000)
assert.equal(bridge.scoreMeta({ ...base, tag: 'textarea', visible: false }), -1000)

const adjacentSubmit = bridge.scoreSubmitAction({
  visible: true,
  disabled: false,
  negativeLabel: false,
  positiveLabel: true,
  submitType: true,
  sameForm: false,
  nearComposer: true,
})
const microphone = bridge.scoreSubmitAction({
  visible: true,
  disabled: false,
  negativeLabel: true,
  positiveLabel: false,
  submitType: false,
  sameForm: false,
  nearComposer: true,
})
assert.ok(adjacentSubmit > 0)
assert.equal(microphone, -1000)
console.log('google web composer bridge passed')
