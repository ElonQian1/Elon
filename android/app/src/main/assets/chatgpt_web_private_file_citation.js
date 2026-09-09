(function (root, factory) {
  'use strict';
  const exported = factory();
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root) root.__elonChatGptPrivateFileCitation = exported;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const FILE = /^file[_-][A-Za-z0-9_-]{1,155}$/;
  const LIBRARY = /^libfile[_-][A-Za-z0-9_-]{1,152}$/;
  const PROJECT = /^g-p-[a-f0-9]{32}$/i;
  const MIME = /^[A-Za-z0-9.+-]{1,63}\/[A-Za-z0-9.+-]{1,63}$/;

  function text(value) {
    return typeof value === 'string' && !/[\x00-\x1f\x7f]/.test(value)
      ? value.replace(/\u00a0/g, ' ').trim() : '';
  }

  function eligible(metadata) {
    // Context citations have a separate deletion/source-mask protocol. Do not
    // treat an incomplete context graph as ordinary content references.
    return metadata && typeof metadata === 'object' && !Array.isArray(metadata) &&
      metadata.conversation_context_citation_metadata == null &&
      metadata.conversation_context_citation_metadata_status == null;
  }

  function target(reference) {
    if (!reference || typeof reference !== 'object' || Array.isArray(reference) ||
        reference.type !== 'file' || reference.retrieval_origin === 'pca' ||
        reference.deleted != null && reference.deleted !== false ||
        reference.category != null && text(reference.category).toLowerCase() !== 'files') return null;
    // Official kpr -> p6i/g6i -> PXe: explicit file citations resolve to a
    // ChatGPT file ID. A cloud URL by itself is not a download credential.
    const ids = [reference.file_id, reference.id].filter(value => value != null);
    if (!ids.length || ids.some(value => typeof value !== 'string' || !FILE.test(value)) ||
        new Set(ids).size !== 1) return null;
    if (['mounted_library_file_id', 'shared_library_file_id', 'library_download_id',
      'preview_file', 'source_url', 'context_connector', 'connector_id',
      'context_connector_info', 'library_artifact_type', 'libraryArtifactType',
      'conversation_id', 'server_thread_id'].some(key => reference[key] != null && reference[key] !== '')) return null;
    if (reference.context_scopes != null &&
        (!Array.isArray(reference.context_scopes) || reference.context_scopes.length)) return null;
    const libraries = [reference.library_file_id, reference.libraryFileId].filter(value => value != null);
    if (libraries.some(value => typeof value !== 'string' || !LIBRARY.test(value)) ||
        new Set(libraries).size > 1) return null;
    const projects = [reference.gizmo_id, reference.project_id].filter(value => value != null);
    if (projects.some(value => typeof value !== 'string' || !PROJECT.test(value)) ||
        new Set(projects).size > 1) return null;
    const name = text(reference.name) || text(reference.title);
    if (!name || name.length > 1024) return null;
    const mime = reference.content_type == null ? '' : text(reference.content_type);
    if (reference.content_type != null && !MIME.test(mime)) return null;
    return { id: ids[0], name: name.slice(0, 180),
      ...(mime ? { mime_type: mime } : {}),
      ...(libraries.length ? { library_file_id: libraries[0] } : {}),
      ...(projects.length ? { gizmo_id: projects[0] } : {}) };
  }

  function references(metadata, existing, limit = 20) {
    if (!eligible(metadata) || !Array.isArray(metadata.content_references)) return [];
    const ids = new Set(), libraries = new Set();
    for (const file of existing || []) {
      if (!file || !text(file.name)) continue;
      if (file.id) ids.add(file.id);
      if (file.library_file_id) libraries.add(file.library_file_id);
    }
    const result = [];
    for (const reference of metadata.content_references.slice(0, limit)) {
      const file = target(reference);
      if (!file || ids.has(file.id) || file.library_file_id && libraries.has(file.library_file_id)) continue;
      ids.add(file.id);
      if (file.library_file_id) libraries.add(file.library_file_id);
      result.push({ reference, file });
    }
    return result;
  }

  return Object.freeze({ version: 1, eligible, target, references });
});
