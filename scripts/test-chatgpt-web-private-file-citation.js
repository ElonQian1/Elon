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
  for (const metadata of [{ conversation_context_citation_metadata: [] },
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
