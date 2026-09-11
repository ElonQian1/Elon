'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const base = '../android/app/src/main/assets/';
const citation = require(base + 'chatgpt_web_private_file_citation.js');
const context = require(base + 'chatgpt_web_private_context_sources_policy.js');
const projection = require(base + 'chatgpt_web_private_history_projection.js').create({});
const sources = require(base + 'chatgpt_web_private_context_sources.js');
const downloads = require(base + 'chatgpt_web_private_file_download.js');
const file = (name = 'cloud', extra = {}) => ({ type: 'file', name,
  url: 'https://docs.example.test/document/' + name,
  cloud_doc_url: 'https://docs.example.test/document/' + name, ...extra });
const metadata = (items, status = 'complete') => ({
  conversation_context_citation_metadata_status: status,
  conversation_context_citation_metadata: items.map(citation => ({ citation }))
});
const payload = meta => ({ messages: [{ id: 'answer', author: { role: 'assistant' },
  content: { parts: ['synthetic source response'] }, metadata: meta }] });

test('only explicit official cloud URL fields yield source links, never download identities', () => {
  const item = file();
  assert.deepEqual(citation.sourceLink(item), { name: 'cloud', url: item.cloud_doc_url });
  assert.equal(citation.target(item), null);
  assert.equal(citation.sourceLink(file('extra', { cloud_doc_url: ' ',
    extra: { cloud_doc_url: 'https://docs.example.test/extra#section' } })).url,
  'https://docs.example.test/extra#section');
  assert.equal(citation.sourceLink(file('plain', { cloud_doc_url: undefined })), null);
  assert.equal(citation.sourceLink(file('wrong-category', { category: 'memory' })), null);
  assert.equal(citation.sourceLink(file('deleted', { deleted: true })), null);
  assert.equal(citation.sourceLink(file('context', { type: 'conversation_context_citation' })), null);
});

test('link validation rejects executable schemes, credentials, controls and malformed cloud values', () => {
  for (const url of ['javascript:alert(1)', 'intent://x', 'file:///sdcard/a', 'data:text/plain,a',
    'http://docs.example.test/a', 'https://user:password@docs.example.test/a',
    'https://docs.example.test:8080/a', 'https://docs.example.test\\@evil.test/a',
    'https://docs.example.test/a\nb', 'https://docs.example.test/%xy', 'https://',
    'https://docs.example.test/' + 'a'.repeat(8192)]) {
    assert.equal(citation.sourceLink(file('unsafe', { cloud_doc_url: url })), null, url.slice(0, 80));
  }
  assert.equal(citation.sourceLink(file('bad-primary', { cloud_doc_url: 'javascript:alert(1)',
    extra: { cloud_doc_url: 'https://docs.example.test/valid' } })), null);
});

test('distinct URL-only inline files remain distinct and use a source row without a file handle', () => {
  const data = payload(metadata([file('first'), file('second')]));
  const rows = projection.files(data);
  assert.equal(rows.truncated, false);
  assert.deepEqual(rows.files.map(x => [x.id, x.kind, x.name, x.sourceUrl]), [
    ['answer:0', 'source', 'first', 'https://docs.example.test/document/first'],
    ['answer:1', 'source', 'second', 'https://docs.example.test/document/second']
  ]);
  assert.equal(projection.fileSource(data, 'answer:0'), null);
  assert.equal(projection.fileSource(data, 'answer:1'), null);
});

test('concrete files retain download positions while source-only links are appended once', () => {
  const concrete = file('download', { file_id: 'file-existing' });
  const data = payload(metadata([concrete, file('cloud'), file('cloud')]));
  data.messages[0].metadata.attachments = [{ id: 'file-upload', name: 'upload.txt' }];
  const rows = projection.files(data).files;
  assert.deepEqual(rows.map(x => [x.kind, x.name]), [['file', 'upload.txt'], ['file', 'download'], ['source', 'cloud']]);
  assert.equal(projection.fileSource(data, 'answer:1').fileCitationReference.file_id, 'file-existing');
  assert.equal(projection.fileSource(data, 'answer:2'), null);
  assert.equal(rows[1].sourceUrl, undefined);
});

test('PCA source links require current approved metadata and respect deletion and mask', () => {
  const item = file('pca', { retrieval_origin: 'pca' });
  const meta = metadata([item], 'seeded');
  assert.equal(citation.sourceLink(item), null);
  let current = true;
  const supplement = { items: [item], status: 'done', mask: { status: 'complete', urls: [item.url] } };
  context.attach(meta, context.resolve(meta, supplement), () => current);
  assert.equal(projection.files(payload(meta)).files[0].kind, 'source');
  assert.equal(projection.files(payload(meta)).truncated, false);
  const approved = context.lookup(meta).items[0];
  assert.equal(citation.sourceLink({ ...approved }), null);
  current = false;
  assert.equal(citation.sourceLink(approved), null);
  assert.equal(projection.files(payload(meta)).files.length, 0);
  context.attach(meta, context.resolve(meta, { ...supplement, mask: { status: 'complete', urls: [] } }), () => true);
  assert.equal(projection.files(payload(meta)).files.length, 0);
  meta.conversation_context_citation_metadata[0].deleted = true;
  context.attach(meta, context.resolve(meta, supplement), () => true);
  assert.equal(projection.files(payload(meta)).files.length, 0);
});

test('source rows traverse the cached private inventory without DOM or a native download request', async () => {
  let network = 0;
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/source' },
    __elonChatGptDocumentToken: 'doc_source_1',
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: 'Bearer synthetic-source-link' }) },
    __elonChatGptPrivateHistoryProjection: require(base + 'chatgpt_web_private_history_projection.js'),
    get document() { throw new Error('Source inventory must not read DOM'); },
    fetch() { network++; throw new Error('Inline sources must not fetch'); }
  };
  const data = payload(metadata([file('native')]));
  const owner = sources.create(root);
  owner.applyCached(data, 'source');
  await owner.enrich(data, 'source');
  const rows = downloads.create(root).register('/c/source', data, projection.files(data));
  assert.equal(rows[0].sourceUrl, 'https://docs.example.test/document/native');
  assert.equal(rows[0].downloadHandle, undefined);
  assert.equal(network, 0);
});

test('exact source-list producer fixture is also consumed by native Kotlin tests', () => {
  const fixture = require('../android/app/src/test/resources/webchat/private-source-links-contract.json');
  const actual = projection.files(fixture.payload);
  assert.deepEqual(actual.files, fixture.event.files);
  assert.equal(actual.truncated, fixture.event.truncated);
});
