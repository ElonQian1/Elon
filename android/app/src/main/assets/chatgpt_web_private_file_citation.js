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
    if (!metadata || typeof metadata !== 'object' || Array.isArray(metadata)) return false;
    const graph = metadata.conversation_context_citation_metadata;
    const status = metadata.conversation_context_citation_metadata_status;
    if (graph != null && !Array.isArray(graph)) return false;
    return graph?.length > 0 ? ['complete', 'complete_inline_only'].includes(status) : status == null;
  }

  function inlineSources(metadata, limit) {
    const graph = metadata.conversation_context_citation_metadata;
    if (!Array.isArray(graph) || !graph.length) return null;
    // n5i/e5i replace matching citation UUIDs before the source mask. Inspect
    // the bounded complete graph, so a later deletion cannot leave an old file.
    if (graph.length > 256) return { items: [], truncated: true };
    const items = [], positions = new Map();
    for (const entry of graph) {
      const value = entry?.citation;
      if (!value || typeof value !== 'object' || Array.isArray(value)) continue;
      const reference = typeof entry.retrieval_origin === 'string'
        ? { ...value, retrieval_origin: entry.retrieval_origin } : value;
      const uuid = typeof reference.citation_uuid === 'string' && reference.citation_uuid;
      if (uuid && positions.has(uuid)) items[positions.get(uuid)] = reference;
      else {
        if (uuid) positions.set(uuid, items.length);
        items.push(reference);
      }
    }
    return { items: items.slice(0, limit), truncated: items.length > limit ||
      metadata.conversation_context_citation_metadata_status === 'complete_inline_only' ||
      items.some(item => item.retrieval_origin === 'pca') };
  }

  function fileUrlId(value) {
    const url = text(value);
    if (!url.toLowerCase().startsWith('file://') || url.length > 8192) return '';
    try {
      const parsed = new URL(url);
      // Official c6i appends search before g6i validates the concrete file ID.
      // Never fetch the file URL or interpret its host as a provider identity.
      const id = decodeURIComponent(parsed.pathname.split('/').filter(Boolean).at(-1) || '') + parsed.search;
      return FILE.test(id) ? id : '';
    } catch (_) { return ''; }
  }

  function target(reference) {
    if (!reference || typeof reference !== 'object' || Array.isArray(reference) ||
        reference.type === 'conversation_context_citation' || reference.retrieval_origin === 'pca' ||
        reference.deleted != null && reference.deleted !== false) return null;
    // c2 classifies grouped/cite-map items by category before type/attribution.
    const isFile = reference.category != null ? text(reference.category).toLowerCase() === 'files' :
      reference.type === 'file' || ['files', 'library'].includes(text(reference.attribution).toLowerCase()) ||
      text(reference.url).toLowerCase().startsWith('file://');
    if (!isFile) return null;
    // Official p6i/c6i/g6i also resolves a concrete ID from a file URL path.
    // A cloud URL by itself is not a download credential.
    const ids = [reference.file_id, reference.id].filter(value => value != null);
    if (!ids.length) {
      const id = fileUrlId(reference.url);
      if (id) ids.push(id);
    }
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

  function scan(metadata, limit = 20) {
    const items = [], urls = new Set();
    let visited = 0, truncated = false;
    limit = Math.max(1, Math.min(20, Number.isSafeInteger(limit) ? limit : 20));
    if (!eligible(metadata)) return { items, truncated:
      metadata?.conversation_context_citation_metadata != null ||
      metadata?.conversation_context_citation_metadata_status != null };
    const object = value => value && typeof value === 'object' && !Array.isArray(value);
    const active = value => object(value) && value.retrieval_origin !== 'pca' &&
      (value.deleted == null || value.deleted === false);
    const bounded = values => {
      if (values.length > limit) truncated = true;
      return values.slice(0, limit);
    };
    function add(value, explicit = false) {
      if (visited >= limit) { truncated = true; return false; }
      visited++;
      if (!object(value)) return true;
      const key = explicit ? 'file://' + text(value.source) + '/' +
        (text(value.id || value.file_id) || fileUrlId(value.url)) : text(value.url);
      if (!key || key.length > 8192 || urls.has(key)) return true;
      urls.add(key); items.push(value);
      return true;
    }
    // Official R5i combines top-level references with one level of per-file arrays.
    // Bucket keys identify rendered output files, not download targets or scopes.
    const inline = inlineSources(metadata, limit);
    if (inline?.truncated) truncated = true;
    const sources = inline ? inline.items :
      Array.isArray(metadata.content_references) ? bounded(metadata.content_references) : [];
    const byFile = metadata.content_references_by_file;
    if (!inline && object(byFile)) {
      let buckets = 0;
      for (const key in byFile) {
        if (!Object.prototype.hasOwnProperty.call(byFile, key)) continue;
        if (++buckets > limit || sources.length >= limit) { truncated = true; break; }
        const values = byFile[key];
        if (!Array.isArray(values)) continue;
        const remaining = limit - sources.length;
        if (values.length > remaining) truncated = true;
        sources.push(...values.slice(0, remaining));
      }
    }
    // kpr traverses these containers once. Do not recursively crawl arbitrary metadata.
    for (const reference of sources) {
      if (!active(reference)) continue;
      if (['grouped_webpages', 'grouped_webpages_v2', 'grouped_webpages_model_predicted_fallback'].includes(reference.type)) {
        if (!Array.isArray(reference.items)) continue;
        const group = reference.items.length ? reference.items : reference.fallback_items;
        if (!Array.isArray(group)) continue;
        for (const item of bounded(group)) {
          if (!add(item)) break;
          if (!active(item) || !Array.isArray(item.supporting_websites)) continue;
          for (const website of bounded(item.supporting_websites)) {
            const child = active(website) ? { ...website, refs: item.refs, type: item.type,
              attribution: website.attribution ?? item.attribution ?? null, category: item.category ?? null,
              retrieval_origin: item.retrieval_origin ?? null } : website;
            if (!add(child)) break;
          }
        }
      } else if (reference.type === 'file') {
        if (!add(reference, true)) break;
      } else if (['webpage', 'webpage_extended'].includes(reference.type)) {
        if (!add(reference)) break;
      } else if (object(reference.cite_map)) {
        let count = 0;
        for (const key in reference.cite_map) {
          if (!Object.prototype.hasOwnProperty.call(reference.cite_map, key)) continue;
          if (++count > limit) { truncated = true; break; }
          if (!add(reference.cite_map[key])) break;
        }
      }
    }
    return { items, truncated };
  }

  function references(metadata, existing, limit = 20) {
    const ids = new Set(), libraries = new Set();
    for (const file of existing || []) {
      if (!file || !text(file.name)) continue;
      if (file.id) ids.add(file.id);
      if (file.library_file_id) libraries.add(file.library_file_id);
    }
    const result = [];
    for (const reference of scan(metadata, limit).items) {
      const file = target(reference);
      if (!file || ids.has(file.id) || file.library_file_id && libraries.has(file.library_file_id)) continue;
      ids.add(file.id);
      if (file.library_file_id) libraries.add(file.library_file_id);
      result.push({ reference, file });
    }
    return result;
  }

  return Object.freeze({ version: 5, eligible, target, references, scan });
});
