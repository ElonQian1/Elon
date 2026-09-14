const test = require('node:test');
const assert = require('node:assert/strict');
const mentions = require('../server/src/assets/group_mentions.js');
const friend = { id: 'u1', display_name: '张 三' };
const ai = { id: 'ai', display_name: 'EL', isAi: true };

test('typed @ replacement keeps draft prefix and suffix', () => {
  assert.deepEqual(mentions.insert('你好 @请看看', 3, 4, [friend]), { text: '你好 @张 三 请看看', cursor: 8 });
});
test('multi-select deduplicates IDs and preserves emoji', () => {
  assert.deepEqual(mentions.insert('😀早上好世界', 5, 5, [ai, friend, ai]), { text: '😀早上好 @EL @张 三 世界', cursor: 15 });
});
test('typing at word boundaries triggers, email does not', () => {
  assert.equal(mentions.trigger('@', 0), true);
  assert.equal(mentions.trigger('你好，＠', 3), true);
  assert.equal(mentions.trigger('user@', 4), false);
});
test('search supports members, case insensitive AI and empty result', () => {
  assert.deepEqual(mentions.filter([friend, ai], '@张'), [friend]);
  assert.deepEqual(mentions.filter([friend, ai], '群ai'), [ai]);
  assert.deepEqual(mentions.filter([friend, ai], 'el'), [ai]);
  assert.deepEqual(mentions.filter([friend, ai], '不存在'), []);
});
