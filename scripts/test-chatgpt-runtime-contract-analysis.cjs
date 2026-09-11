'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { parseSource, compareSymbol, roleFiles } = require('./analyze-chatgpt-runtime-contracts.cjs');

test('public source is parsed, not executed; local rename is only an advisory match', () => {
  const before = parseSource('function f(a){return a.done===true?1:0} export {f as A};');
  const after = parseSource('throw Error("must_not_execute");function g(b){return b.done===true?1:0} export {g as B};');
  assert.deepEqual(compareSymbol(before, after, 'A').candidates, [{ local: 'g', exports: ['B'] }]);
});

test('property keys, literals and operators are not erased', () => {
  const before = parseSource('function f(a){return a.done===true?1:0} export {f as A};');
  for (const expression of ['b.ready===true?1:0', 'b.done===false?1:0', 'b.done!==true?1:0', 'b.done===true?2:0']) {
    assert.equal(compareSymbol(before, parseSource(`function g(b){return ${expression}} export {g as B};`), 'A').candidates.length, 0);
  }
});

test('ambiguous stores and repeated assignments are reported, never chosen', () => {
  const before = parseSource('var f=makeStore(null); export {f as A};');
  const after = parseSource('var g=makeStore(null),h=makeStore(null); export {g as B,h as C};');
  assert.equal(compareSymbol(before, after, 'A').candidates.length, 2);
  const assigned = parseSource('var f;function initialize(){f=makeStore(null);f=makeStore(1)} export {f as A};');
  const result = compareSymbol(assigned, after, 'A');
  assert.equal(result.definitions, 2);
  assert.deepEqual(result.candidates, []);
});

test('unexported owner matching is explicit and export names remain separate', () => {
  const before = parseSource('function owner(){return "temporary"}');
  const after = parseSource('function next(){return "temporary"}');
  assert.equal(compareSymbol(before, after, 'owner').candidates.length, 0);
  assert.deepEqual(compareSymbol(before, after, 'owner', true).candidates, [{ local: 'next', exports: [] }]);
});

test('role imports require one exact local asset per role', () => {
  const source = ['./4813494d-one.js', './conversation-small-two.js', './8b34dbc2-three.js'];
  assert.deepEqual(roleFiles(source), { shared: '4813494d-one.js', conversation: 'conversation-small-two.js', composer: '8b34dbc2-three.js' });
  assert.throws(() => roleFiles([...source, './4813494d-duplicate.js']), /ambiguous/);
  assert.throws(() => roleFiles(['https://example.org/4813494d-one.js', ...source.slice(1)]), /ambiguous/);
  assert.throws(() => roleFiles(['../4813494d-one.js', ...source.slice(1)]), /ambiguous/);
});
