const { test } = require('node:test');
const assert = require('node:assert/strict');
const { webcrypto } = require('node:crypto');
const factory = require('../desktop-shell/src-tauri/src/local_ai_browser/win_attachment_source.js');
const nativeSource = require('../android/app/src/main/assets/chatgpt_web_native_attachment_source.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');
const batchId = '00000000-0000-4000-8000-000000000001';
const leaseId = '00000000-0000-4000-8000-000000000002';
function fixture() {
  const receipts = [], uploaded = [];
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/?temporary-chat=true' },
    __elonChatGptDocumentToken: 'doc_fixture_1', __elonChatGptAdapterVersion: 208,
    __elonChatGptPrivateAttachmentProtocol: protocol, crypto: webcrypto,
    File, Blob, atob, btoa, setTimeout, clearTimeout,
    elonChatGptNative: { postMessage: raw => receipts.push(JSON.parse(raw)) } };
  const source = factory(root), reader = nativeSource.create(root);
  root.__elonChatGptPrivateAttachmentSend = { cancel() {}, async start(raw, respond) {
    for (const descriptor of JSON.parse(raw).files) {
      assert.equal(descriptor.uploadCopy, true);
      const file = await reader.read(descriptor);
      uploaded.push(new Uint8Array(await file.arrayBuffer()));
    }
    respond('request_attachment_upload', true, 'private_attachment_associated');
  } };
  const command = value => source.command(JSON.stringify({ value: JSON.stringify({ batchId, ...value }), requestId: 'mcp_fixture' }));
  return { root, receipts, uploaded, source, command };
}
function descriptor(size, sha256) { return { leaseId, size, name: 'group_01_fixture.txt', type: 'text/plain', sha256 }; }

test('bounded chunks reach the shared uploader byte-for-byte and release the send guard', async () => {
  const f = fixture(), bytes = new Uint8Array(70001).map((_, i) => i % 251);
  const hash = Buffer.from(await webcrypto.subtle.digest('SHA-256', bytes)).toString('hex');
  await f.command({ step: 'begin', files: [descriptor(bytes.length, hash)] });
  assert.equal(f.source.guardSend('{"action":"send_prompt","requestId":"mcp_guard"}'), true);
  for (let offset = 0; offset < bytes.length; offset += 65536)
    await f.command({ step: 'chunk', leaseId, offset, data: Buffer.from(bytes.subarray(offset, offset + 65536)).toString('base64') });
  await f.command({ step: 'upload' });
  assert.equal(f.uploaded.length, 1);
  assert.deepEqual(f.uploaded[0], bytes);
  assert.equal(f.receipts.at(-1).detail, 'private_attachment_associated');
  assert.equal(f.source.guardSend('{"action":"send_prompt"}'), false);
});

test('partial bytes, duplicate chunks, wrong digest and navigation never upload', async () => {
  for (const scenario of ['partial', 'duplicate', 'digest', 'navigation']) {
    const f = fixture();
    await f.command({ step: 'begin', files: [descriptor(3, scenario === 'digest' ? '0'.repeat(64) : undefined)] });
    if (scenario === 'navigation') f.root.__elonChatGptDocumentToken = 'doc_other';
    if (scenario !== 'partial') await f.command({ step: 'chunk', leaseId, offset: 0, data: 'YWJj' });
    if (scenario === 'duplicate') await f.command({ step: 'chunk', leaseId, offset: 0, data: 'YWJj' });
    await f.command({ step: 'upload' });
    assert.equal(f.receipts.at(-1).ok, false, scenario);
    assert.equal(f.uploaded.length, 0, scenario);
  }
});

test('oversized and unsupported batches fail before allocating a file lease', async () => {
  for (const file of [{ ...descriptor(1), type: 'audio/mpeg' }, descriptor(8388609), { ...descriptor(1), name: '../bad.txt' }]) {
    const f = fixture(); await f.command({ step: 'begin', files: [file] });
    assert.equal(f.receipts.at(-1).ok, false);
    assert.equal(f.source.guardSend('{"action":"send_prompt"}'), false);
  }
});
