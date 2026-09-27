'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const file = path.join(__dirname, '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_win_directory.js');
const { decodePins, mergeRows } = require(file);
const conversation = { item_type: 'conversation', pinned_at: '2026-09-28T00:00:00Z', item: { id:'one', title:'One' } };
const project = { item_type: 'project', item: { gizmo: { id:'g-p-one', display: { name:'Project' } } } };

test('typed upstream pins normalize conversations and projects, including explicit empty', () => {
  assert.equal(decodePins([conversation], 'conversation')[0].pinnedAt, Date.parse(conversation.pinned_at));
  assert.equal(decodePins([project], 'project')[0].path, '/g/g-p-one/project');
  assert.deepEqual(decodePins([], 'conversation'), []);
  assert.throws(() => decodePins({ items:[] }, 'conversation'), /schema_changed/);
  assert.throws(() => decodePins([project], 'conversation'), /schema_changed/);
  assert.throws(() => decodePins([{ ...conversation, item: { id:'../bad', title:'Bad' } }], 'conversation'));
});
test('unknown pins preserve cached status; authoritative empty explicitly unpins', () => {
  const rows = [{ id:'one', title:'Old', pinned:true }];
  assert.equal(mergeRows(rows, undefined)[0].pinned, true);
  assert.equal(mergeRows(rows, [])[0].pinned, false);
  assert.equal(mergeRows([], decodePins([conversation], 'conversation'))[0].id, 'one');
});
test('Win refresh adds same-origin pin reads, preserves last good pins on failure, invalidates account', async () => {
  let account = 'fixture-a', fail = false;
  const calls = [], modes = [];
  const base = { snapshot: () => ({ conversations:[{ id:'one', title:'One' }], projects:[] }),
    refresh: async () => { modes.push(root.__elonWinDirectoryRecentOnly); return { ok:true }; },
    refreshScope: async scope => ({ ok:true, scope }), setListener() {} };
  const root = { location: { origin:'https://chatgpt.com' }, fetch() {}, AbortController, setTimeout, clearTimeout,
    __elonChatGptDocumentToken:'doc_fixture_123', __elonChatGptPrivateConversationDirectory:base,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ 'chatgpt-account-id':account }),
      acquireSameOriginRequestHeaders: async () => ({ 'chatgpt-account-id':account }) },
    __elonChatGptPrivateJsonRequest: { request: async (_, url, init) => {
      calls.push([url, init.credentials]); if (fail) throw Error('timeout');
      return { text:JSON.stringify(url.endsWith('conversation') ? [conversation] : [project]) };
    } } };
  vm.runInNewContext(fs.readFileSync(file,'utf8'), { window:root });
  const adapter = root.__elonChatGptPrivateConversationDirectory;
  assert.equal((await adapter.refresh()).pinsReady, true);
  assert.equal(calls.length, 2);
  assert.ok(calls.every(([url, credentials]) => url.startsWith('/backend-api/pins?') && credentials === 'same-origin'));
  assert.equal(adapter.snapshot().conversations[0].pinned, true);
  fail = true;
  assert.equal((await adapter.refresh('history')).pinsReady, false);
  assert.equal(adapter.snapshot().conversations[0].pinned, true);
  assert.deepEqual(modes, [true, false]);
  assert.equal(root.__elonWinDirectoryRecentOnly, true);
  account = 'fixture-b';
  assert.equal(adapter.snapshot().conversations[0].pinned, undefined);
});
test('late conversation prefetch cannot navigate back or publish into a newer target', () => {
  const rust = fs.readFileSync(path.join(__dirname,'../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_cached_conversation_navigation.rs'),'utf8');
  const template = rust.split('r#"')[1].split('"#')[0].replaceAll('{{','{').replaceAll('}}','}');
  const pending = [], navigated = [], events = [];
  const window = { __elonChatGptDocumentToken:'doc_fixture_123', __elonChatGptAdapterVersion:216,
    elonChatGptNative: { postMessage: event => events.push(event) },
    __elonChatGptPrivateTransport: { conversationPrefetchEnabled:true, prefetchConversation: (...args) => { pending.push(args); return true; } } };
  const context = { window, URL, location:{ origin:'https://chatgpt.com', pathname:'/c/start', assign: url => navigated.push(url) } };
  vm.runInNewContext(template.replace('{encoded_path}', JSON.stringify('/c/first')), context);
  vm.runInNewContext(template.replace('{encoded_path}', JSON.stringify('/c/second')), context);
  pending[0][1]({ type:'message_snapshot' }); pending[0][2]();
  assert.equal(events.length,0); assert.equal(navigated.length,0);
  pending[1][1]({ type:'message_snapshot' }); pending[1][2]();
  assert.equal(events.length,1); assert.equal(navigated[0],'https://chatgpt.com/c/second');
});

test('production Win dependency graph performs private directory reads and emits typed pins', async () => {
  const { fixture, response, assets } = require('./fixtures/chatgpt-directory-refresh');
  const { root } = fixture();
  const calls = [], events = [];
  root.fetch = async url => {
    calls.push(url);
    const payload = url.includes('/pins?') ? (url.endsWith('conversation') ? [conversation] : [project]) :
      url.includes('/snorlax/') ? { items:[{ gizmo:{ gizmo:project.item.gizmo }, conversations:[] }], cursor:null } :
        { items:[conversation.item], offset:0, limit:28, total:100 };
    const result = response(payload); result.clone = () => response(payload); return result;
  };
  const context = vm.createContext({ window:root, location:root.location, URL, TextDecoder, TextEncoder,
    setTimeout, clearTimeout, console });
  const bootstrap = fs.readFileSync(path.join(__dirname,
    '../desktop-shell/src-tauri/src/local_ai_browser/chatgpt_adapter_bootstrap.rs'),'utf8');
  const modules = ['chatgpt_web_private_conversation_directory.js','chatgpt_web_private_directory_pages.js',
    'chatgpt_web_private_directory_refresh.js','chatgpt_web_adapter_conversation_directory_requests.js'];
  for (const name of modules) {
    assert.ok(bootstrap.includes(`"${name}",`), `missing production dependency ${name}`);
    vm.runInContext(fs.readFileSync(path.join(assets,name),'utf8'),context);
    if (name === modules[0]) vm.runInContext(fs.readFileSync(file,'utf8'),context);
  }
  const requests = root.__elonChatGptConversationDirectoryRequests.create({
    privateDirectory:root.__elonChatGptPrivateConversationDirectory,
    conversationAdapter:{ requestList() { throw Error('unexpected DOM fallback'); } },
    emitEvent:event => events.push(event), optional:(_, action) => action(),
  });
  requests.installListener();
  const receipt = await new Promise(resolve => requests.requestList({ requestId:'fixture' }, (...args) => resolve(args)));
  assert.equal(receipt[1],true);
  const result = events.at(-1);
  assert.equal(result.collection.source,'official_private');
  assert.equal(result.collection.refreshSettled,true);
  assert.equal(result.conversations[0].pinned,true);
  assert.equal(result.projects[0].pinned,true);
  assert.equal(calls.filter(url => url.includes('/conversations?')).length,1);
  assert.equal(calls.filter(url => url.includes('/pins?')).length,2);
});
