'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const { webcrypto } = require('node:crypto');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const flush = () => new Promise(resolve => setTimeout(resolve, 20));

function fixture() {
  const calls = [], events = [], replies = [];
  const headers = { Authorization: 'Bearer synthetic-fixture-identity' };
  const root = { location: { origin: 'https://chatgpt.com', pathname: '/', href: 'https://chatgpt.com/' },
    __elonChatGptDocumentToken: 'doc_page_fixture', crypto: webcrypto, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTransport: { acquireSameOriginRequestHeaders: async () => headers, copySameOriginRequestHeaders: () => headers },
    fetch: async url => {
      calls.push(url);
      const parsed = new URL(url, 'https://chatgpt.com');
      const offset = Number(parsed.searchParams.get('offset'));
      return new Response(JSON.stringify({ offset, limit: 28, total: 250,
        items: Array.from({ length: Math.min(28, 250 - offset) }, (_, n) => ({ id: 'fixture-' + (n + offset), title: 'Fixture ' + (n + offset) })) }));
    },
  };
  const context = vm.createContext({ window: root, location: root.location, URL, Response, Uint8Array, TextDecoder, TextEncoder, setTimeout, clearTimeout });
  for (const name of ['private_json_request', 'private_directory_pages', 'private_directory_browser',
    'directory_page_requests', 'private_conversation_directory', 'adapter_conversation_directory_requests']) {
    vm.runInContext(fs.readFileSync(path.join(assets, 'chatgpt_web_' + name + '.js'), 'utf8'), context);
  }
  const directory = root.__elonChatGptPrivateConversationDirectory;
  const controller = root.__elonChatGptConversationDirectoryRequests.create({ privateDirectory: directory,
    emitEvent: event => events.push(JSON.parse(JSON.stringify(event))), optional: (_, fn) => fn(),
    conversationAdapter: { requestList() { assert.fail('DOM path must not be used'); } } });
  const command = (id, handle = '', scope = 'conversations') => controller.handleCommand({ action: 'browse_directory_page',
    requestId: id, value: JSON.stringify({ scope, handle }) }, (...reply) => replies.push(reply));
  return { root, context, calls, events, replies, directory, controller, command };
}

test('real asset chain maps complete pages into the dedicated native event, leaving recent cache unchanged', async () => {
  const f = fixture();
  let handle = '', count = 0;
  do {
    f.command('mcp_page' + count, handle); await flush();
    assert.equal(f.replies.at(-1)?.[1], true, JSON.stringify(f.replies));
    const event = f.events.at(-1);
    assert.equal(event.type, 'directory_page');
    assert.equal(event.requestId, 'mcp_page' + count);
    assert.equal(event.requestedHandle, handle);
    assert.equal(f.replies.at(-1)[1], true);
    if (count === 7) assert.equal(event.conversations[0].id, 'fixture-196');
    assert(event.conversations.every(row => row.path === '/c/' + row.id));
    handle = event.nextHandle; count++;
  } while (handle);
  assert.equal(f.events.reduce((n, event) => n + event.conversations.length, 0), 250);
  assert.equal(f.directory.snapshot().conversations.length, 0);
  assert.equal(f.calls.length, count);
});

test('tombstones are deliberately omitted without dropping the rest of a page or advancing a guessed cursor', async () => {
  const f = fixture();
  f.directory.acceptDeletedState('fixture-0');
  f.command('mcp_deleted'); await flush();
  const event = f.events[0];
  assert.equal(event.conversations.length, 27);
  assert.equal(event.conversations[0].id, 'fixture-1');
  f.command('mcp_next', event.nextHandle); await flush();
  assert.equal(f.events[1].conversations[0].id, 'fixture-28');
  const handle = f.events[1].handle;
  f.directory.acceptDeletedState('fixture-28');
  f.command('mcp_stale', handle); await flush();
  assert.equal(f.replies.at(-1)[2], 'directory_page_expired');
  assert.equal(f.events.length, 2);
});

test('cancelled reads cannot emit a page, cancellation retains successful waypoints for explicit retry', async () => {
  const f = fixture();
  f.command('mcp_one'); await flush();
  const handle = f.events[0].nextHandle;
  f.controller.handleCommand({ action: 'cancel_directory_page', value: 'mcp_one' }, () => {});
  f.command('mcp_two', handle); await flush();
  assert.equal(f.events.length, 2);
  let resolve;
  const pages = f.root.__elonChatGptDirectoryPageRequests.create({ browsePage: () => new Promise(r => { resolve = r; }), cancelBrowse() {} }, () => assert.fail('stale event'));
  pages.handle({ action: 'browse_directory_page', requestId: 'mcp_cancel', value: JSON.stringify({ scope: 'projects', handle: '' }) }, () => assert.fail('stale receipt'));
  await flush();
  pages.handle({ action: 'cancel_directory_page', value: 'mcp_cancel' }, () => {});
  resolve({ ok: true, items: [] }); await flush();
});

test('invalid scopes and opaque cursor substitutes are rejected before any private request', async () => {
  const f = fixture();
  f.command('mcp_invalid', 'raw-provider-cursor');
  f.command('mcp_invalid', '', 'https://other.example');
  await flush();
  assert.equal(f.calls.length, 0);
  assert(f.replies.every(reply => reply[1] === false));
});
