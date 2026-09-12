'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { webcrypto } = require('node:crypto');
const base = '../android/app/src/main/assets/';
const images = require(base + 'chatgpt_web_private_message_image.js');
const downloads = require(base + 'chatgpt_web_private_file_download.js');
const pointer = require(base + 'chatgpt_web_private_image_pointer.js');
const json = require(base + 'chatgpt_web_private_json_request.js');

function fixture() {
  const calls = [], packets = [], receipts = [];
  let account = 'Bearer synthetic-image-account';
  class Image {
    isConnected = true;
    currentSrc = 'https://files.oaiusercontent.com/preview?sig=synthetic';
    src = this.currentSrc;
    naturalWidth = 512;
    naturalHeight = 512;
    closest() { return this.isMessage === false ? null : {}; }
    getAttribute() { return this.src; }
  }
  const node = new Image();
  const props = { asset: { content_type: 'image_asset_pointer', asset_pointer: 'sediment://file-original?variant=final' },
    checkContextScopesForConversationId: 'owned-conversation', downloadFileName: 'fixture-original.png' };
  function Den() { throw new Error('component must never execute'); }
  const host = { stateNode: node };
  const owner = { type: Den, memoizedProps: props, child: host };
  const rootFiber = { child: owner };
  rootFiber.stateNode = { current: rootFiber };
  host.return = owner;
  owner.return = rootFiber;
  node.__reactFiber$synthetic = host;
  const bridge = { postMessage(raw) {
    const packet = JSON.parse(raw);
    packets.push(packet);
    if (!packet.cancel) queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: packet.leaseId, state: 'queued' }) }));
  } };
  const page = { location: new URL('https://chatgpt.com/c/owned-conversation'), HTMLImageElement: Image,
    __elonChatGptPrivateRuntimeBindings: { observed: () => true },
    __elonChatGptDocumentToken: 'doc_message_image',
    __elonChatGptPrivateImagePointer: pointer,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    __elonChatGptPrivateJsonRequest: json,
    elonChatGptFileDownload: bridge, crypto: webcrypto, AbortController, setTimeout, clearTimeout,
    fetch: async (url, init) => {
      calls.push({ url: new URL(url), init });
      return Response.json({ status: 'success', file_id: 'file-original',
        download_url: 'https://files.oaiusercontent.com/original?sig=synthetic' });
    },
  };
  const download = downloads.create(page);
  page.__elonChatGptPrivateFileDownload = download;
  const image = images.create(page);
  page.__elonChatGptPrivateMessageImage = image;
  const run = selection => download.start(JSON.stringify({ ...selection, downloadHandle: selection.handle,
    version: 1, documentToken: 'doc_message_image', href: page.location.href,
    leaseId: '00000000-0000-4000-8000-000000000001' }), (...args) => receipts.push(args));
  return { page, node, props, owner, host, rootFiber, image, download, run, calls, packets, receipts,
    switchAccount: () => { account = 'Bearer different-synthetic-account'; } };
}

test('selected message image registers without fetching and reuses original download ownership', async () => {
  const f = fixture(), selected = f.image.describe(f.node);
  assert.match(selected.handle, /^download_[a-f0-9]{32}$/);
  assert.equal(f.calls.length, 0);
  assert.deepEqual(f.image.describe(f.node), selected, 'stable snapshot handle, no repeated registration churn');
  assert.doesNotMatch(JSON.stringify(selected), /sediment|file-original|sig=|Bearer|oaiusercontent/);
  await f.run(selected);
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/file-original');
  assert.deepEqual([...f.calls[0].url.searchParams], [['variant', 'final'],
    ['check_context_scopes_for_conversation_id', 'owned-conversation'], ['download_intent', 'true']]);
  assert.equal(f.calls[0].init.method, 'GET');
  assert.equal(f.packets[0].url, 'https://files.oaiusercontent.com/original?sig=synthetic');
  assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
});

test('message metadata reaches preview descriptions without using preview bytes as originals', () => {
  const f = fixture();
  vm.runInNewContext(fs.readFileSync(require.resolve(base + 'chatgpt_web_image_assets.js'), 'utf8'), {
    window: f.page, location: f.page.location, HTMLImageElement: f.page.HTMLImageElement, URL, URLSearchParams,
  });
  const value = f.page.__elonChatGptImageAssets.describe(f.node);
  assert.equal(value.original.name, 'fixture-original.png');
  assert.match(value.assetHandle, /^image_/);
  assert.doesNotMatch(JSON.stringify(value), /sediment|sig=|Bearer|oaiusercontent/);
  assert.equal(f.calls.length, 0);
});

test('unconfirmed runtime, renderer, image, conversation and alternate scope never invent an original', () => {
  for (const change of [f => { f.page.__elonChatGptPrivateRuntimeBindings.observed = () => false; },
    f => { f.owner.type = function Other() {}; }, f => { f.props.isOptimisticPlaceholder = true; },
    f => { f.node.isMessage = false; }, f => { f.rootFiber.stateNode.current = {}; },
    f => { f.props.checkContextScopesForConversationId = 'other'; }, f => { f.props.libraryFileId = 'bad/id'; },
    f => { f.props.asset.context_scopes = ['unknown']; }, f => { f.props.asset.asset_pointer = 'https://example.test/x.png'; },
    f => { f.node.src = 'https://files.oaiusercontent.com/not-yet-loaded'; },
    f => { f.props.downloadFileName = 'bad\nname'; }]) {
    const f = fixture(); change(f);
    assert.equal(f.image.describe(f.node), null);
    assert.equal(f.calls.length, 0);
  }
});

test('account, document, route, node and image changes reject a stale original before any request', async () => {
  for (const change of [f => f.switchAccount(), f => { f.page.__elonChatGptDocumentToken = 'doc_other'; },
    f => { f.page.location = new URL('https://chatgpt.com/c/other'); }, f => { f.node.isConnected = false; },
    f => { f.props.asset.asset_pointer = 'sediment://file-other'; },
    f => { f.node.src = 'https://files.oaiusercontent.com/not-yet-loaded'; }]) {
    const f = fixture(), selected = f.image.describe(f.node);
    change(f);
    await f.run(selected);
    assert.equal(f.calls.length, 0);
    assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
  }
});

test('rotating preview authorization does not invalidate the same original file identity', async () => {
  const f = fixture(), selected = f.image.describe(f.node);
  f.node.currentSrc = f.node.src = 'https://files.oaiusercontent.com/preview?sig=renewed';
  assert.deepEqual(f.image.describe(f.node), selected);
  await f.run(selected);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/file-original');
  assert.equal(f.receipts.at(-1)[2], 'download_queued');
});

test('image replacement while authorizing cannot enqueue an unrelated or stale original', async () => {
  const f = fixture(), selected = f.image.describe(f.node);
  f.page.fetch = async () => {
    f.props.asset.asset_pointer = 'sediment://file-next';
    return Response.json({ status: 'success', download_url: 'https://files.oaiusercontent.com/stale' });
  };
  await f.run(selected);
  assert.equal(f.packets.filter(value => value.url).length, 0);
  assert.equal(f.receipts.at(-1)[2], 'download_cancelled');
});

test('refreshing the private file index does not expire a selected message image', async () => {
  const f = fixture(), selected = f.image.describe(f.node);
  f.download.register(selected.path, {}, { files: [] });
  await f.run(selected);
  assert.equal(f.receipts.at(-1)[2], 'download_queued');
});

test('a still-current message image survives idle time but requests fresh authorization on download', async t => {
  t.mock.timers.enable({ apis: ['Date'], now: 1_000_000 });
  const f = fixture(), selected = f.image.describe(f.node);
  t.mock.timers.tick(15 * 60 * 1000);
  assert.deepEqual(f.image.describe(f.node), selected);
  await f.run(selected);
  assert.equal(f.calls.length, 1);
  assert.equal(f.receipts.at(-1)[2], 'download_queued');
});

test('library image retains scoped metadata validation and native original byte saving', async () => {
  const f = fixture(); f.props.libraryFileId = 'libfile_original';
  f.props.asset.asset_pointer = 'file-service://file-original';
  const urls = [], bytes = [];
  f.page.__elonChatGptPrivateLibraryDownload = {
    contentUrl: () => true,
    runContent: async (page, job, current, url) => {
      assert.equal(current(job), true); bytes.push(url); return 'download_saved';
    },
  };
  f.page.fetch = async url => {
    const parsed = new URL(url); urls.push(parsed);
    return parsed.pathname.endsWith('/simple')
      ? Response.json({ file_id: 'file-original', is_library_file: true, library_file_id: 'libfile_original', is_project: false })
      : Response.json({ status: 'success', download_url: '/backend-api/estuary/content?id=original' });
  };
  await f.run(f.image.describe(f.node));
  assert.equal(urls[0].pathname, '/backend-api/files/file-original/simple');
  assert.equal(urls[0].searchParams.get('conversation_id'), 'owned-conversation');
  assert.equal(urls[1].searchParams.get('check_context_scopes_for_conversation_id'), 'owned-conversation');
  assert.deepEqual(bytes, ['/backend-api/estuary/content?id=original']);
  assert.equal(f.receipts.at(-1)[2], 'download_saved');
});
