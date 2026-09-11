'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const citation = require('../android/app/src/main/assets/chatgpt_web_private_file_citation.js');
const file = (extra = {}) => ({ type: 'file', id: 'file-synthetic',
  name: 'reference.txt', content_type: 'text/plain', source: 'library', ...extra });

test('explicit official file references map to the existing attachment download contract', () => {
  assert.deepEqual(citation.target(file()), {
    id: 'file-synthetic', name: 'reference.txt', mime_type: 'text/plain',
  });
  assert.equal(citation.target(file({ id: undefined, file_id: 'file_other' })).id, 'file_other');
  assert.equal(citation.target(file({ name: undefined, title: 'Title.pdf' })).name, 'Title.pdf');
});

test('library and project identities are preserved, not converted to an ordinary unscoped file', () => {
  const project = 'g-p-0123456789abcdef0123456789abcdef';
  assert.deepEqual(citation.target(file({ libraryFileId: 'libfile_synthetic', project_id: project })), {
    id: 'file-synthetic', name: 'reference.txt', mime_type: 'text/plain',
    library_file_id: 'libfile_synthetic', gizmo_id: project,
  });
  for (const extra of [{ file_id: 'file-other' }, { library_file_id: 'bad' },
    { library_file_id: 'libfile_first', libraryFileId: 'libfile_second' },
    { project_id: 'other' }, { context_scopes: ['other'] }, { context_scopes: {} },
    { conversation_id: 'other' }, { preview_file: {} }, { mounted_library_file_id: 'mount' },
    { shared_library_file_id: 'libfile_other' }, { library_artifact_type: 'gdoc' }]) {
    assert.equal(citation.target(file(extra)), null, JSON.stringify(extra));
  }
});

test('cloud URLs stay attribution only and never become a download URL', () => {
  const cloud = file({ cloud_doc_url: 'https://cloud.example.test/private?token=synthetic',
    url: 'https://external.test/preview', snippet: 'synthetic snippet' });
  assert.doesNotMatch(JSON.stringify(citation.target(cloud)), /https|token|snippet/);
  assert.equal(citation.target({ ...cloud, id: undefined }), null);
  for (const id of ['external-id', 'file-../escape', 'file-x?scope=other', 'file-', 42, {}, 'file-x\n']) {
    assert.equal(citation.target(file({ id })), null);
  }
});

test('PCA, removed, masked, malformed and unsupported reference families remain unclaimed', () => {
  for (const extra of [{ retrieval_origin: 'pca' }, { deleted: true }, { deleted: 'false' },
    { category: 'memory' }, { category: {} }, { type: 'conversation_context_citation' },
    { type: 'webpage' }, { name: 'bad\nname.txt' }, { content_type: {} }]) {
    assert.equal(citation.target(file(extra)), null);
  }
  for (const metadata of [{ conversation_context_citation_metadata: [{}] },
    { conversation_context_citation_metadata: {} },
    { conversation_context_citation_metadata_status: 'marker_only' }]) {
    assert.deepEqual(citation.references({ ...metadata, content_references: [file()] }), []);
  }
  for (const invalid of [undefined, null, [], 42, 'invalid', {}]) assert.equal(citation.target(invalid), null);
});

test('attachment-first deduplication retains file and library identity while keeping bounded references', () => {
  const refs = [file(), file({ id: 'file-copy', library_file_id: 'libfile_copy' }),
    file({ id: 'file-next' }), file({ id: 'file-next' })];
  assert.deepEqual(citation.references({ content_references: refs }, [
    { id: 'file-synthetic', name: 'original.txt' }, { library_file_id: 'libfile_copy', name: 'copy.txt' },
  ]).map(row => row.file.id), ['file-next']);
  const many = Array.from({ length: 21 }, (_, i) => file({ id: 'file-' + i }));
  assert.equal(citation.references({ content_references: many }).length, 20);
  assert.deepEqual(citation.references({ content_references: {} }), []);
});

const groupedFile = (extra = {}) => file({ type: 'webpage', category: 'files',
  url: 'https://cloud.example.test/reference', ...extra });
const ids = refs => citation.references({ content_references: refs }).map(row => row.file.id);

const inline = (citations, status = 'complete') => ({
  conversation_context_citation_metadata: citations,
  conversation_context_citation_metadata_status: status,
});

test('completed inline context files use their explicit identity and exclude unrelated legacy references', () => {
  const metadata = { ...inline([{ citation: file() }]),
    content_references: [file({ id: 'file-legacy' })],
    content_references_by_file: { old: [file({ id: 'file-per-file-legacy' })] } };
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-synthetic']);
  assert.deepEqual(citation.references(metadata, [{ id: 'file-synthetic', name: 'existing.txt' }]), []);
  const project = 'g-p-0123456789abcdef0123456789abcdef';
  const scoped = citation.references(inline([{ citation: file({ library_file_id: 'libfile_inline', gizmo_id: project }) }]));
  assert.equal(scoped[0].file.library_file_id, 'libfile_inline');
  assert.equal(scoped[0].file.gizmo_id, project);
});

test('inline citation UUID replacement happens before source masking and download deduplication', () => {
  const first = file({ citation_uuid: 'citation-a' });
  const next = file({ citation_uuid: 'citation-a', id: 'file-new', name: 'new.txt' });
  const metadata = inline([{ citation: first }, { citation: next }]);
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-new']);
  metadata.conversation_context_citation_metadata.push({ citation: { ...next, deleted: true } });
  assert.deepEqual(citation.references(metadata), []);
  metadata.conversation_context_citation_metadata.at(-1).citation = { ...next, retrieval_origin: 'pca' };
  assert.deepEqual(citation.references(metadata), []);
  const laterDeletion = [{ citation: first }, ...Array.from({ length: 40 }, (_, i) => ({
    citation: file({ id: 'file-filler-' + i }),
  })), { citation: { ...first, deleted: true } }];
  assert.ok(citation.references(inline(laterDeletion)).every(row => row.file.id !== first.id),
    'inspect replacements beyond the 20-row display budget');
});

test('inline origin overrides follow the official wrapper contract without admitting PCA or past chats', () => {
  const metadata = inline([
    { citation: file(), retrieval_origin: 'pca' },
    { citation: file({ id: 'file-allowed', retrieval_origin: 'pca' }), retrieval_origin: 'library' },
    { citation: { type: 'conversation_context_citation', conversation_context_type: 'past_conversation',
      id: 'file-past', name: 'past.txt' } },
    { citation: file({ id: 'file-deleted', deleted: true }) },
    { citation: file({ id: 'file-other-scope', context_scopes: ['other'] }) },
  ]);
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-allowed']);
});

test('incomplete, marker and malformed context graphs cannot grant a file reference', () => {
  for (const status of [undefined, 'seeded', 'pending_inline_finalize', 'marker_only', 'unknown', '']) {
    const metadata = { ...inline([{ citation: file() }]), conversation_context_citation_metadata_status: status };
    assert.deepEqual(citation.references(metadata), []);
    assert.equal(citation.scan(metadata).truncated, true);
  }
  for (const entry of [null, [], 42, {}, { citation: null }, { citation: [] }]) {
    assert.deepEqual(citation.references(inline([entry])), []);
  }
});

test('inline-only and capped graphs report incomplete coverage rather than silently complete lists', () => {
  const metadata = inline([{ citation: file() }], 'complete_inline_only');
  assert.equal(citation.references(metadata).length, 1);
  assert.equal(citation.scan(metadata).truncated, true);
  const many = Array.from({ length: 21 }, (_, i) => ({ citation: file({ id: 'file-' + i }) }));
  assert.equal(citation.references(inline(many)).length, 20);
  assert.equal(citation.scan(inline(many)).truncated, true);
  const oversized = Array.from({ length: 257 }, () => ({ citation: file() }));
  Object.defineProperty(oversized, 256, { get() { throw Error('unbounded inline access'); } });
  assert.deepEqual(citation.scan(inline(oversized)), { items: [], truncated: true });
});

test('per-file references follow top-level references without treating bucket keys as file identities', () => {
  const metadata = { content_references: [file({ id: 'file-first' })],
    content_references_by_file: { 'not-a-file-id': [file({ id: 'file-second' })],
      'file-not-a-target': [{ type: 'grouped_webpages', items: [groupedFile({ id: 'file-third' })] }] } };
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-first', 'file-second', 'file-third']);
  delete metadata.content_references;
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-second', 'file-third']);
  assert.deepEqual(citation.references(metadata, [{ id: 'file-second', name: 'existing.txt' }])
    .map(row => row.file.id), ['file-third']);
  metadata.content_references = [groupedFile({ id: 'file-first' })];
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-first', 'file-second'],
    'first URL wins across both official metadata fields');
});

test('per-file references keep context masks, deleted sources and malformed buckets unclaimed', () => {
  const metadata = { content_references_by_file: { valid: [file()],
    notAnArray: file({ id: 'file-wrong' }), nested: [[file({ id: 'file-nested' })]],
    removed: [file({ id: 'file-deleted', deleted: true })],
    pca: [file({ id: 'file-pca', retrieval_origin: 'pca' })] } };
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-synthetic']);
  for (const extra of [{ conversation_context_citation_metadata: [{}] },
    { conversation_context_citation_metadata_status: 'marker_only' }]) {
    assert.deepEqual(citation.references({ ...metadata, ...extra }), []);
  }
  for (const byFile of [null, [], 'invalid', 42]) {
    assert.deepEqual(citation.references({ content_references: [file()], content_references_by_file: byFile })
      .map(row => row.file.id), ['file-synthetic']);
  }
  const inherited = Object.create({ ignored: [file({ id: 'file-inherited' })] });
  inherited.own = [file()];
  assert.deepEqual(citation.references({ content_references_by_file: inherited }).map(row => row.file.id), ['file-synthetic']);
});

test('both metadata fields share bounded outer traversal, source budget and truncation', () => {
  const many = Array.from({ length: 21 }, (_, i) => file({ id: 'file-' + i }));
  const metadata = { content_references: many.slice(0, 10),
    content_references_by_file: { first: many.slice(10, 20), second: many.slice(20) } };
  assert.equal(citation.references(metadata).length, 20);
  assert.equal(citation.scan(metadata).truncated, true);
  assert.deepEqual(citation.references(metadata, [], 2).map(row => row.file.id), ['file-0', 'file-1']);
  const emptyBuckets = Object.fromEntries(Array.from({ length: 20 }, (_, i) => ['empty' + i, []]));
  Object.defineProperty(emptyBuckets, 'beyondBudget', { enumerable: true,
    get() { throw new Error('must not read an unbounded number of buckets'); } });
  assert.deepEqual(citation.scan({ content_references_by_file: emptyBuckets }), { items: [], truncated: true });
  const nested = { type: 'grouped_webpages', items: many.map((f, i) => ({ ...f,
    category: 'files', url: 'file://library/file-' + i })) };
  assert.equal(citation.scan({ content_references_by_file: { grouped: [nested] } }).items.length, 20);
  assert.equal(citation.scan({ content_references_by_file: { grouped: [nested] } }).truncated, true);
});

test('official file URL path identities enter the existing citation target without fetching that URL', () => {
  for (const url of ['file://library/file-synthetic', 'file:///file-synthetic',
    'FILE://library/nested/file%2Dsynthetic', 'file://library/file-synthetic#preview']) {
    assert.deepEqual(citation.target(groupedFile({ id: undefined, url })), {
      id: 'file-synthetic', name: 'reference.txt', mime_type: 'text/plain',
    }, url);
  }
  assert.equal(citation.target(groupedFile({ id: undefined, category: undefined,
    url: 'file://library/file_synthetic' })).id, 'file_synthetic');
  assert.equal(citation.target(groupedFile({ url: 'file://library/file-other' })).id, 'file-synthetic',
    'an explicit valid ID retains the existing precedence over attribution');
});

test('an empty context-citation array does not hide unrelated ordinary file references', () => {
  const metadata = { conversation_context_citation_metadata: [], content_references: [file()] };
  assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-synthetic']);
  for (const status of ['marker_only', 'unknown', '']) {
    assert.deepEqual(citation.references({ ...metadata, conversation_context_citation_metadata_status: status }), []);
  }
  for (const extra of [{ retrieval_origin: 'pca' }, { deleted: true }, { type: 'conversation_context_citation' }]) {
    assert.deepEqual(citation.references({ ...metadata, content_references: [file(extra)] }), []);
  }
});

test('file URL fallback cannot rescue invalid explicit identity or acquire cloud/context access', () => {
  const url = 'file://library/file-synthetic';
  for (const extra of [{ id: '' }, { id: 42 }, { id: 'bad-id' }, { file_id: 'bad-id' },
    { category: 'web' }, { retrieval_origin: 'pca' }, { deleted: true },
    { context_scopes: ['other'] }, { preview_file: {} }, { mounted_library_file_id: 'mounted' }]) {
    assert.equal(citation.target(groupedFile({ id: undefined, url, ...extra })), null);
  }
  for (const url of ['https://external.example.test/file-synthetic', 'file://file-synthetic',
    'file://library/unknown-id', 'file://library/file-', 'file://library/file-synthetic?scope=other',
    'file://library/file-synthetic%3Fscope%3Dother', 'file://library/file-synthetic%2Fother',
    'file://library/file-synthetic%00', 'file://library/file-%zz', 'file://library/file-x\n',
    'file://library/' + 'x'.repeat(8192) + '/file-synthetic']) {
    assert.equal(citation.target(groupedFile({ id: undefined, url })), null, url.slice(0, 80));
  }
});

test('grouped and cite-map URL-only references deduplicate against existing attachments', () => {
  const item = groupedFile({ id: undefined, url: 'file://library/file-synthetic' });
  for (const container of [{ type: 'grouped_webpages_v2', items: [item] }, { cite_map: { first: item } }]) {
    const metadata = { content_references: [container] };
    assert.deepEqual(citation.references(metadata).map(row => row.file.id), ['file-synthetic']);
    assert.deepEqual(citation.references(metadata, [{ id: 'file-synthetic', name: 'original.txt' }]), []);
  }
  assert.deepEqual(ids([
    file({ id: undefined, url: 'file://library/file-first' }),
    file({ id: undefined, url: 'file://library/file-second' }),
  ]), ['file-first', 'file-second']);
});

test('file-classified reference items retain concrete identity without using their cloud URL', () => {
  for (const extra of [{ category: 'files' }, { category: undefined, attribution: 'Library' },
    { category: undefined, attribution: 'Files' }, { category: undefined, url: 'file://library/file-synthetic' }]) {
    assert.deepEqual(citation.target(groupedFile(extra)), {
      id: 'file-synthetic', name: 'reference.txt', mime_type: 'text/plain',
    });
  }
  assert.equal(citation.target(groupedFile({ category: 'web', attribution: 'files' })), null);
  assert.equal(citation.target(groupedFile({ id: undefined })), null);
  assert.equal(citation.target(groupedFile({ type: 'conversation_context_citation' })), null);
});

for (const type of ['grouped_webpages', 'grouped_webpages_v2', 'grouped_webpages_model_predicted_fallback']) {
  test(type + ' adds file sources through the existing attachment contract', () => {
    const item = groupedFile({ library_file_id: 'libfile_synthetic' });
    const rows = citation.references({ content_references: [{ type, items: [item] }] });
    assert.equal(rows.length, 1);
    assert.equal(rows[0].reference, item);
    assert.equal(rows[0].file.library_file_id, 'libfile_synthetic');
    assert.doesNotMatch(JSON.stringify(rows[0].file), /https|cloud/);
  });
}

test('group fallback is used only when the primary items array is empty', () => {
  const group = { type: 'grouped_webpages', items: [], fallback_items: [groupedFile()] };
  assert.deepEqual(ids([group]), ['file-synthetic']);
  group.items = [{ type: 'webpage', url: 'https://other.example.test', title: 'not a file' }];
  assert.deepEqual(ids([group]), []);
  group.items = null;
  assert.deepEqual(ids([group]), []);
});

test('cite-map values use their own URL and concrete file identity, not the map key', () => {
  const map = { type: 'fixture_map', cite_map: {
    'file-not-a-target': groupedFile(),
    'second': groupedFile({ id: 'file-second', url: 'https://cloud.example.test/second' }),
    'invalid': { ...file({ id: 'file-without-url' }), url: undefined }, 'null': null,
  } };
  assert.deepEqual(ids([map]), ['file-synthetic', 'file-second']);
  assert.deepEqual(ids([{ cite_map: [groupedFile()] }]), []);
});

test('supporting sources inherit category and retrieval origin from their owning item', () => {
  const support = groupedFile({ id: 'file-support', url: 'https://cloud.example.test/support',
    category: undefined, library_file_id: 'libfile_support' });
  const item = { type: 'webpage', category: 'files', url: 'https://primary.example.test', supporting_websites: [support] };
  const group = { type: 'grouped_webpages_v2', items: [item] };
  const rows = citation.references({ content_references: [group] });
  assert.equal(rows.length, 1); assert.equal(rows[0].reference.category, 'files');
  assert.equal(rows[0].file.library_file_id, 'libfile_support');
  item.retrieval_origin = 'pca'; assert.deepEqual(ids([group]), []);
  delete item.retrieval_origin;
  support.retrieval_origin = 'pca'; assert.deepEqual(ids([group]), []);
  delete support.retrieval_origin;
  item.category = 'web'; assert.deepEqual(ids([group]), []);
});

test('duplicate grouped URLs retain the first source rather than acquiring a new file identity', () => {
  const first = groupedFile(), second = groupedFile({ id: 'file-other' });
  assert.deepEqual(ids([{ type: 'grouped_webpages', items: [first, second] }]), ['file-synthetic']);
  assert.deepEqual(ids([{ type: 'grouped_webpages', items: [
    { url: first.url, category: 'web', title: 'not a file' }, second,
  ] }]), []);
});

test('deleted/PCA containers and unsupported recursive groups cannot add download targets', () => {
  const group = { type: 'grouped_webpages', items: [groupedFile()] };
  for (const extra of [{ deleted: true }, { deleted: 'false' }, { retrieval_origin: 'pca' }]) {
    assert.deepEqual(ids([{ ...group, ...extra }]), []);
    assert.deepEqual(ids([{ type: 'fixture_map', cite_map: { first: groupedFile() }, ...extra }]), []);
  }
  assert.deepEqual(ids([{ type: 'grouped_webpages', items: [group] }]), []);
  assert.deepEqual(ids([{ type: 'grouped_webpages', items: [groupedFile({ deleted: true })] }]), []);
});

test('nested reference traversal shares a bounded budget and reports truncation', () => {
  const items = Array.from({ length: 21 }, (_, i) => groupedFile({ id: 'file-' + i,
    url: 'https://cloud.example.test/' + i }));
  const metadata = { content_references: [{ type: 'grouped_webpages', items }] };
  assert.equal(citation.references(metadata).length, 20);
  assert.equal(citation.scan(metadata).truncated, true);
  metadata.content_references[0].items = items.slice(0, 20);
  assert.equal(citation.scan(metadata).truncated, false);
  const map = { content_references: [{ cite_map: Object.fromEntries(items.map((item, i) => [i, item])) }] };
  assert.equal(citation.references(map).length, 20); assert.equal(citation.scan(map).truncated, true);
  const support = { content_references: [{ type: 'grouped_webpages', items: [
    { url: 'https://primary.example.test', type: 'webpage', category: 'files', supporting_websites: items },
  ] }] };
  assert.equal(citation.references(support).length, 19);
  assert.equal(citation.scan(support).truncated, true);
});
