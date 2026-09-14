const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');

const page = fs.readFileSync(path.join(__dirname, '../server/src/assets/web_page.html'), 'utf8');
const start = page.indexOf('  function recommendationName(');
const end = page.indexOf('  async function loadFriendRecommendations()', start);
assert(start > 0 && end > start);

function fixture() {
  const pending = new Map();
  const requests = [];
  let timer = 0;
  const context = vm.createContext({
    AbortController, encodeURIComponent,
    friendSearchInput: { value: '' },
    friendRecommendationList: { innerHTML: '' },
    friendRecommendationTitle: { textContent: '' },
    addFriendResult: { textContent: '', style: {} },
    escapeHtml: (s) => String(s).replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('"', '&quot;'),
    setTimeout: (fn) => { pending.set(++timer, fn); return timer; },
    clearTimeout: (id) => pending.delete(id),
    api: (url, options) => new Promise((resolve) => requests.push({ url, options, resolve }))
  });
  vm.runInContext(`
    let friendRecommendations = [];
    let friendLookup = { query: '', user: null, message: '' };
    let friendLookupRevision = 0, friendLookupTimer = null, friendLookupAbort = null;
    let friendRecommendationsLoading = false, friendRecommendationsError = '';
    ${page.slice(start, end)}
  `, context);
  return {
    context, requests,
    search(query, immediate = true) {
      context.friendSearchInput.value = query;
      vm.runInContext(`searchRegisteredFriend(${immediate})`, context);
    },
    runTimers() { const jobs = [...pending.values()]; pending.clear(); jobs.forEach((fn) => fn()); },
    async reply(index, data, ok = true) {
      requests[index].resolve({ ok, json: async () => data });
      await new Promise(setImmediate);
    }
  };
}

test('finds a user outside recommendations through the authenticated search API', async () => {
  const f = fixture();
  f.search('13900000052', false);
  assert.equal(f.requests.length, 0);
  f.runTimers();
  assert.equal(f.requests[0].url, '/api/me/friends/search?search_type=auto&query=13900000052');
  await f.reply(0, { found: true, user: { id: 'usr_target', nickname: 'Test User', account: '13900000052' } });
  assert.match(f.context.friendRecommendationList.innerHTML, /Test User/);
  assert.match(f.context.friendRecommendationList.innerHTML, /data-add-friend-id="usr_target"/);
});

test('debounces typing and ignores stale responses after a newer query or clear', async () => {
  const f = fixture();
  f.search('fir', false);
  f.search('first', false);
  f.runTimers();
  f.search('second');
  assert.equal(f.requests.length, 2);
  assert.equal(f.requests[0].options.signal.aborted, true);
  await f.reply(1, { found: true, user: { id: 'usr_second', nickname: 'Second' } });
  await f.reply(0, { found: true, user: { id: 'usr_first', nickname: 'First' } });
  assert.match(f.context.friendRecommendationList.innerHTML, /Second/);
  assert.doesNotMatch(f.context.friendRecommendationList.innerHTML, /First/);
  f.search('third');
  f.search('');
  await f.reply(2, { found: true, user: { id: 'usr_third', nickname: 'Third' } });
  assert.equal(f.context.friendRecommendationList.innerHTML, '');
});

test('same-name errors and not-found results are visible and retryable', async () => {
  const f = fixture();
  f.search('Same Name');
  assert.match(f.requests[0].url, /Same%20Name/);
  await f.reply(0, { error: '找到多个同名用户，请改用手机号' }, false);
  assert.match(f.context.addFriendResult.textContent, /多个同名用户/);
  f.search('13900000052');
  await f.reply(1, { found: false });
  assert.match(f.context.addFriendResult.textContent, /未找到用户/);
});

test('self and existing friends cannot be added again', async () => {
  const f = fixture();
  f.search('self');
  await f.reply(0, { found: true, is_self: true, user: { id: 'usr_self' } });
  assert.match(f.context.friendRecommendationList.innerHTML, /disabled>你自己/);
  assert.match(f.context.addFriendResult.textContent, /不能添加自己/);
  f.search('friend');
  await f.reply(1, { found: true, already_friend: true, user: { id: 'usr_friend' } });
  assert.match(f.context.friendRecommendationList.innerHTML, /disabled>已添加/);
});

test('closing the dialog invalidates an in-flight search', async () => {
  const f = fixture();
  f.search('target');
  vm.runInContext('cancelFriendLookup()', f.context);
  await f.reply(0, { found: true, user: { id: 'usr_target' } });
  assert.equal(f.context.friendRecommendationList.innerHTML, '');
});
