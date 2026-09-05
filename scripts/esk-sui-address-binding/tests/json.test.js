const test = require('node:test')
const assert = require('node:assert/strict')
const { parseUniqueJson } = require('../json')

test('unique JSON parser accepts nested protocol-compatible JSON values', () => {
  assert.deepEqual(parseUniqueJson('{"a":[1,true,null,{"b":"value"}],"c":-2.5e3}'), {
    a: [1, true, null, { b: 'value' }], c: -2500,
  })
})

test('unique JSON parser rejects duplicate keys at every object depth', () => {
  for (const text of [
    '{"network":"mainnet","network":"testnet"}',
    '{"network":"mainnet","\\u006eetwork":"testnet"}',
    '{"outer":{"schema":"v1","schema":"v2"}}',
    '[{"challenge_id":"one","challenge_id":"two"}]',
  ]) assert.throws(() => parseUniqueJson(text), /INVALID_INPUT/)
})

test('unique JSON parser rejects malformed delimiters, numbers, escapes and trailing data', () => {
  for (const text of [
    '', '{"a":1,}', '[1,]', '{"a":01}', '{"a":true false}',
    '{"a":"\\x"}', '{"a":1}{"b":2}', '{"a" 1}',
  ]) assert.throws(() => parseUniqueJson(text), /INVALID_INPUT/)
})
