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
