const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const Module = require('node:module');
const ts = require('typescript');
const file = path.resolve(__dirname, '../src/features/friends/chat-records/recordApi.ts');
let token = 'test-session';
const compiled = new Module(file, module);
compiled.require = id => id.endsWith('/client') ? { getAuthToken: () => token }
  : id.endsWith('/runtime') ? { resolveApiUrl: value => 'https://example.test' + value } : require(id);
compiled._compile(ts.transpileModule(fs.readFileSync(file, 'utf8'), { compilerOptions: { module:ts.ModuleKind.CommonJS, target:ts.ScriptTarget.ES2020 } }).outputText, file);
const { recordCard, RECORD_PREFIX, recordPath, recordRequest } = compiled.exports;
const card = { schema:'chat_record_bundle_v1', record_id:'record_test', group_id:'group_test', title:'Fixture', summary:'A: text', message_count:2, total_count:3 };
test('versioned card rejects missing identifiers and unsupported shapes', () => {
  assert.deepEqual(recordCard(RECORD_PREFIX + JSON.stringify(card)), card);
  for (const value of [{...card,record_id:undefined}, {...card,group_id:null}, {...card,record_id:'../../other'}, {...card,message_count:0}, {...card,schema:'future'}]) {
    assert.equal(recordCard(RECORD_PREFIX + JSON.stringify(value)), null);
  }
  assert.equal(recordCard('ordinary text'), null);
  assert.equal(recordPath(card), '/api/me/groups/group_test/chat-records/record_test');
});
test('authorized fetch uses request headers rather than public media query tokens', async () => {
  token = 'test-session'; const original = global.fetch;
  global.fetch = async (url, options) => {
    assert.equal(url, 'https://example.test' + recordPath(card)); assert.equal(options.cache, 'no-store');
    assert.equal(options.headers.Authorization, 'Bearer test-session');
    return new Response(JSON.stringify({ card }));
  };
  try { assert.deepEqual(await recordRequest(recordPath(card), new AbortController().signal), { card }); }
  finally { global.fetch = original; }
});
test('permission loss, oversized media and account change fail closed', async () => {
  token = 'test-session'; const original = global.fetch;
  try {
    global.fetch = async () => new Response('', { status:403 });
    await assert.rejects(recordRequest(recordPath(card), new AbortController().signal));
    global.fetch = async () => new Response('', { headers: { 'content-length':String(12 * 1024 * 1024 + 1) } });
    await assert.rejects(recordRequest(recordPath(card), new AbortController().signal, true));
    global.fetch = async () => { token = 'different-session'; return new Response('{}'); };
    await assert.rejects(recordRequest(recordPath(card), new AbortController().signal));
  } finally { global.fetch = original; token = 'test-session'; }
});
