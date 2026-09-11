'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_context_sources_policy.js');
const stream = require('../android/app/src/main/assets/chatgpt_web_private_stream_policy.js');
const citation = require('../android/app/src/main/assets/chatgpt_web_private_file_citation.js');
const file = (id = 'one', extra = {}) => ({ type: 'file', file_id: 'file-' + id,
  name: id + '.txt', url: 'file://library/file-' + id, citation_uuid: id, ...extra });
const meta = (items = [], status = 'seeded') => ({
  conversation_context_citation_metadata: items.map(citation => ({ citation })),
  conversation_context_citation_metadata_status: status
});
const source = item => ({ type: 'conversation_context_source', item });
const mask = (urls = [], status = 'complete', types) => ({ type: 'pca_source_filter_mask',
  mask: { status, urls, ...(types ? { filtered_source_types: types } : {}) } });
const sse = events => events.map(e => 'data: ' + JSON.stringify(e) + '\n\n').join('');
const resolve = (metadata, events) => policy.resolve(metadata,
  policy.decode(sse(events), stream.createSseDecoder));

test('seed and supplemental UUID replacement precede mask and row bounds', () => {
  const seed = meta([file('one'), file('two')]);
  const result = resolve(seed, [source(file('one', { deleted: true })), source(file('three'))]);
  assert.deepEqual(result.items.filter(x => !x.deleted).map(x => x.file_id), ['file-two', 'file-three']);
  assert.equal(result.partial, false);
});

test('PCA approval is page-local, resolved, masked and revocable; raw flags cannot forge it', () => {
  const metadata = meta();
  const pca = file('pca', { retrieval_origin: 'pca' });
  assert.equal(citation.target(pca), null);
  let current = true;
  policy.attach(metadata, resolve(metadata, [source(pca), mask([pca.url])]), () => current);
  const row = citation.scan(metadata).items[0];
  assert.equal(citation.target(row).id, 'file-pca');
  assert.equal(citation.target({ ...row }), null);
  assert.equal(Object.isFrozen(row), true);
  current = false;
  assert.equal(citation.target(row), null);
  assert.equal(citation.scan(metadata).truncated, true);
  assert.equal(JSON.stringify(metadata).includes('file-pca'), false);
});

test('source mask keeps ordinary files; filters PCA files except selected types and gmail dream', () => {
  const a = file('allowed', { retrieval_origin: 'pca' });
  const items = [file('plain'), a, file('hidden', { retrieval_origin: 'pca' }),
    file('dream', { retrieval_origin: 'pca', source_type: 'GMAIL_DREAM' })];
  assert.deepEqual(resolve(meta(), [...items.map(source), mask([' ' + a.url + ' '])])
    .items.map(x => x.file_id), ['file-plain', 'file-dream', 'file-allowed']);
  assert.equal(resolve(meta(), [...items.map(source), mask([], 'complete', ['gmail'])]).items.length, 4);
});

test('complete mask can authoritatively empty a seeded PCA set', () => {
  const metadata = meta([file('pca', { retrieval_origin: 'pca' })]);
  const result = resolve(metadata, [mask([])]);
  assert.equal(result.partial, false);
  assert.deepEqual(result.items, []);
});

test('complete inline avoids network; partial inline requests expansion; pending waits', () => {
  assert.equal(policy.inspect(meta([file()], 'complete')).fetch, false);
  assert.equal(policy.inspect(meta([file()], 'complete_inline_only')).expand, true);
  assert.equal(policy.inspect(meta([file()], 'pending_inline_finalize')).fetch, false);
  assert.equal(policy.resolve(meta([file()], 'pending_inline_finalize')).partial, true);
  assert.equal(policy.inspect({ attachments: [] }), null);
  assert.equal(policy.inspect(meta([], 'unknown')).fetch, false);
});

test('inline precedence and outer origin follow the official selection', () => {
  const metadata = meta([file('inline')], 'complete_inline_only');
  metadata.conversation_context_citation_metadata[0].retrieval_origin = 'pca';
  const result = resolve(metadata, [source(file('replacement')), source(file('second')), mask([])]);
  assert.deepEqual(result.items.map(x => x.file_id), ['file-replacement', 'file-second']);
  const complete = meta([file('inline')], 'complete');
  assert.deepEqual(resolve(complete, [source(file('other'))]).items.map(x => x.file_id), ['file-inline']);
});

test('strict shared SSE handles delta channels and EOF, never applies child patches twice', () => {
  const text = 'event: delta_encoding\ndata: v1\n\n' +
    'event: delta\ndata: ' + JSON.stringify({ c: 0, p: '', o: 'replace', v: source(file()) }) + '\n\n' +
    'event: delta\ndata: ' + JSON.stringify({ c: 1, p: '', o: 'replace', v: mask([]) }) + '\n\n';
  const decoded = policy.decode(text, stream.createSseDecoder);
  assert.equal(decoded.status, 'done');
  assert.equal(decoded.items.length, 1);
  assert.equal(decoded.mask.status, 'complete');
  assert.throws(() => policy.decode('event: delta_encoding\ndata: v9\n\n', stream.createSseDecoder));
  assert.throws(() => policy.decode('data: {broken}\n\n', stream.createSseDecoder));
  assert.throws(() => policy.decode(sse([source(file())]) + 'data: {broken}\n\n', stream.createSseDecoder));
  assert.throws(() => policy.decode('data: {broken}\n\n' + text, stream.createSseDecoder));
});

test('oversized graphs, missing mask fields and HTML are unresolved, not approved empty lists', () => {
  assert.throws(() => policy.decode(sse([mask(null)]), stream.createSseDecoder));
  assert.throws(() => policy.decode('<html>login</html>', stream.createSseDecoder));
  assert.equal(policy.resolve(meta(Array.from({ length: 257 }, (_, i) => file('n' + i)), 'complete')).partial, true);
  assert.throws(() => policy.decode(sse(Array.from({ length: 257 }, (_, i) => source(file('n' + i)))),
    stream.createSseDecoder));
});

test('pending mask and URL-only files remain partial; explicit removals are not downloadable', () => {
  const metadata = meta();
  policy.attach(metadata, resolve(metadata, [source(file('pca', { retrieval_origin: 'pca' })),
    source({ type: 'file', url: 'https://example.test/file', name: 'cloud' }), mask([], 'pending')]), () => true);
  assert.equal(citation.scan(metadata).truncated, true);
  assert.equal(citation.references(metadata).length, 0);
  const deleted = meta([file('gone', { deleted: true })], 'complete');
  policy.attach(deleted, policy.resolve(deleted), () => true);
  assert.equal(citation.references(deleted).length, 0);
});

test('replacement overlays revoke old approved objects and retain original metadata', () => {
  const metadata = meta();
  policy.attach(metadata, resolve(metadata, [source(file('pca', { retrieval_origin: 'pca' }))]), () => true);
  const old = citation.scan(metadata).items[0];
  policy.attach(metadata, resolve(metadata, [source(file('pca', { retrieval_origin: 'pca', deleted: true }))]), () => true);
  assert.equal(citation.target(old), null);
  assert.equal(citation.references(metadata).length, 0);
});

test('context index and matched memory references deduplicate before UUID and file selection', () => {
  const metadata = meta([{ type: 'conversation_context_citation', conversation_context_type: 'past_conversation',
    matched_text: '\ue200memcite\ue202PC\ue2022\ue201', citation_uuid: 'old' }, file('seed')]);
  const result = resolve(metadata, [source({ type: 'conversation_context_citation',
    conversation_context_type: 'past_conversation', index: 1, citation_uuid: 'new' }), source(file('extra'))]);
  assert.equal(result.items.length, 3);
  assert.equal(result.items[0].citation_uuid, 'new');
});

test('oversized merged graph fails partial instead of approving early rows', () => {
  const metadata = meta(Array.from({ length: 200 }, (_, i) => file('seed' + i)));
  const result = resolve(metadata, Array.from({ length: 100 }, (_, i) => source(file('extra' + i))));
  assert.equal(result.partial, true);
  assert.deepEqual(result.items, []);
});
