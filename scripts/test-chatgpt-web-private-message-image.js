'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { webcrypto } = require('node:crypto');
const base = '../android/app/src/main/assets/';
const images = require(base + 'chatgpt_web_private_message_image.js');
const generatedImages = require(base + 'chatgpt_web_private_generated_image.js');
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
  page.__elonChatGptPrivateGeneratedImage = generatedImages.create(page);
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

function generatedFixture() {
  const f = fixture();
  const props = { pointer: { content_type: 'image_asset_pointer', asset_pointer: 'sediment://file-original',
    metadata: { generation: { gen_id: 'synthetic-generation' } } },
    messageId: 'generated-message', conversation: { serverId$: () => 'owned-conversation' } };
  const rendererProps = { src: f.node.src, datadogImageContext: {
    conversation_id: 'owned-conversation', message_id: props.messageId } };
  const actionProps = { conversation: props.conversation, messageId: props.messageId,
    fullSizeImageAsset: props.pointer, imageAssetPointer: f.node.src, imageUrl: f.node.src,
    hasWatermarkedDownload: false, isRenderedImageWatermarked: false };
  function lo() { throw Error('do not render'); }
  function qa() { throw Error('do not render'); }
  function Xt() { throw Error('do not execute overlay'); }
  const renderer = { type: qa, memoizedProps: rendererProps, child: f.host, return: f.owner };
  const action = { type: Xt, memoizedProps: actionProps, return: f.owner };
  renderer.sibling = action;
  f.host.return = renderer;
  f.owner.child = renderer;
  f.owner.type = lo;
  f.owner.memoizedProps = props;
  f.node.isMessage = false;
  return { ...f, props, renderer, action, rendererProps, actionProps };
}

test('generated message original uses the observed final image and conversation download scope', async () => {
  const f = generatedFixture(), selected = f.image.describe(f.node);
  assert.equal(selected.name, 'image.png');
  assert.equal(f.calls.length, 0, 'preview description never waits for a private request');
  await f.run(selected);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/file-original');
  assert.deepEqual([...f.calls[0].url.searchParams], [['conversation_id', 'owned-conversation'],
    ['inline', 'false'], ['download_intent', 'true']]);
  assert.equal(f.packets[0].url, 'https://files.oaiusercontent.com/original?sig=synthetic');
  assert.equal(f.receipts.at(-1)[2], 'download_queued');
});

test('generated images in a project retain conversation ownership, not guessed file project ownership', async () => {
  const f = generatedFixture();
  f.page.location = new URL('https://chatgpt.com/g/g-p-' + 'a'.repeat(32) + '/c/owned-conversation');
  await f.run(f.image.describe(f.node));
  assert.deepEqual([...f.calls[0].url.searchParams], [['conversation_id', 'owned-conversation'],
    ['inline', 'false'], ['download_intent', 'true']]);
});

test('free-account default follows the official watermarked pointer without using preview bytes', async () => {
  const f = generatedFixture();
  f.props.pointer.metadata.watermarked_asset_pointer = 'sediment://file-watermarked?variant=final';
  f.actionProps.hasWatermarkedDownload = true;
  f.actionProps.isRenderedImageWatermarked = true;
  f.actionProps.imageUrl = 'https://files.oaiusercontent.com/watermarked-preview';
  f.node.src = f.node.currentSrc = f.actionProps.imageUrl;
  f.page.fetch = async (url, init) => {
    f.calls.push({ url: new URL(url), init });
    return Response.json({ status: 'success', file_id: 'file-watermarked',
      download_url: 'https://files.oaiusercontent.com/watermarked-original' });
  };
  const selected = f.image.describe(f.node);
  assert.doesNotMatch(JSON.stringify(selected), /file-watermarked|sig=|oaiusercontent/);
  await f.run(selected);
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/file-watermarked');
  assert.equal(f.calls[0].url.searchParams.get('variant'), 'final');
  assert.equal(f.packets[0].url, 'https://files.oaiusercontent.com/watermarked-original');
});

test('paid-account default does not force a metadata watermark when the official overlay does not', async () => {
  const f = generatedFixture();
  f.props.pointer.metadata.watermarked_asset_pointer = 'sediment://file-watermarked';
  await f.run(f.image.describe(f.node));
  assert.equal(f.calls[0].url.pathname, '/backend-api/files/download/file-original');
});

test('unready, preview, ambiguous, foreign and no-auth generated images have no original descriptor', () => {
  for (const change of [f => { f.props.isPreview = true; }, f => { f.props.compact = true; },
    f => { f.props.pointer.metadata.is_no_auth_placeholder = true; },
    f => { f.props.conversation.serverId$ = () => 'other'; },
    f => { f.props.pointer.context_scopes = ['unknown']; },
    f => { f.renderer.sibling = null; }, f => { f.action.type = function Unknown() {}; },
    f => { f.actionProps.conversation = { serverId$: () => 'owned-conversation' }; },
    f => { f.actionProps.messageId = 'other'; }, f => { f.actionProps.fullSizeImageAsset = { ...f.props.pointer }; },
    f => { f.actionProps.hasWatermarkedDownload = true; },
    f => { f.actionProps.isRenderedImageWatermarked = true; },
    f => { delete f.actionProps.hasWatermarkedDownload; },
    f => { f.actionProps.imageUrl = ''; }, f => { f.actionProps.imageAssetPointer = 'stale'; },
    f => { f.rendererProps.datadogImageContext.conversation_id = 'other'; },
    f => { f.rendererProps.datadogImageContext.message_id = 'other'; },
    f => { f.node.src = f.node.currentSrc = 'https://files.oaiusercontent.com/unrelated'; },
    f => { f.action.sibling = { ...f.action }; },
    f => { f.action.child = f.owner; }, f => { f.action.child = { tag: 4 }; },
    f => { f.page.__elonChatGptPrivateRuntimeBindings.observed = url => !url.includes('ecab41d6'); }]) {
    const f = generatedFixture(); change(f);
    assert.equal(f.image.describe(f.node), null);
    assert.equal(f.calls.length, 0);
  }
});

test('generated binding expires on ownership/policy changes but survives refreshed preview URLs', async () => {
  const f = generatedFixture(), selected = f.image.describe(f.node);
  f.node.src = f.node.currentSrc = f.rendererProps.src = f.actionProps.imageAssetPointer = f.actionProps.imageUrl =
    'https://files.oaiusercontent.com/preview?sig=rotated';
  assert.deepEqual(f.image.describe(f.node), selected);
  f.props.pointer.metadata.watermarked_asset_pointer = 'sediment://file-watermarked';
  f.actionProps.hasWatermarkedDownload = true;
  await f.run(selected);
  assert.equal(f.calls.length, 0);
  assert.equal(f.receipts.at(-1)[2], 'download_selection_expired');
});

test('replaced generation after authorization cannot save the previous image', async () => {
  const f = generatedFixture(), selected = f.image.describe(f.node);
  f.page.fetch = async () => {
    f.props.pointer.asset_pointer = 'sediment://file-next';
    return Response.json({ status: 'success', download_url: 'https://files.oaiusercontent.com/stale' });
  };
  await f.run(selected);
  assert.equal(f.packets.filter(packet => packet.url).length, 0);
  assert.equal(f.receipts.at(-1)[2], 'download_cancelled');
});
