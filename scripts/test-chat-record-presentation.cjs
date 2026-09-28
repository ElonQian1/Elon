const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const root = path.resolve(__dirname, '..');
const context = vm.createContext({});
vm.runInContext(fs.readFileSync(path.join(root, 'server/src/assets/chat_record_presentation.js'), 'utf8'), context);
const api = context.ElonRecordPresentation;
const cases = JSON.parse(fs.readFileSync(path.join(root, 'android/app/src/test/resources/chat-record-presentation.json'), 'utf8'));
for (const [i, row] of cases.entries()) test(`presentation ${i}: ${row.kind}`, () => {
  const raw = JSON.stringify(row); assert.equal(api.text(row, row.cards), row.expected); assert.equal(JSON.stringify(row), raw);
});
test('stable avatar and duration', () => {
  assert.equal(api.identity(' 示例 ').color, api.identity('示例').color);
  assert.equal(api.identity('😀示例').initial, '😀');
  assert.equal(api.duration(56), '0:56'); assert.equal(api.duration(Infinity), '');
});
