(function (root, factory) {
  'use strict';
  const protocol = Object.freeze(factory());
  if (typeof module === 'object' && module.exports) module.exports = protocol;
  if (root && root.location?.origin === 'https://chatgpt.com') {
    root.__elonChatGptPrivateAttachmentProtocol = protocol;
  }
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const MAX_BYTES = 8 * 1024 * 1024;
  const FILE_ID = /^[A-Za-z0-9_-]{1,160}$/;
  const USE_CASES = new Set(['ace_upload', 'my_files']);

  function prepare(file, context) {
    if (!file || !Number.isSafeInteger(file.size) || file.size < 1 || file.size > MAX_BYTES ||
        typeof file.slice !== 'function') throw new Error('invalid_file');
    if (typeof file.name !== 'string' || !file.name.trim() || file.name.length > 120 ||
        /[\x00-\x1f\x7f/\\]/.test(file.name)) throw new Error('invalid_file_name');
    if (!context || !USE_CASES.has(context.useCase) || typeof context.storeInLibrary !== 'boolean' ||
        context.libraryPersistenceMode !== 'required' || typeof context.indexForRetrieval !== 'boolean') {
      throw new Error('unsupported_upload_context');
    }
    if (context.isProjectThread || context.isTemporaryChat || context.gizmoId || context.directoryId ||
        /^image\//i.test(file.type)) throw new Error('unsupported_upload_context');
    if (typeof file.type !== 'string' || !/^[\w.+-]+\/[\w.+-]+$/.test(file.type)) {
      throw new Error('invalid_mime_type');
    }
    return {
      file_name: file.name, file_size: file.size, use_case: context.useCase,
      timezone_offset_min: new Date().getTimezoneOffset(), reset_rate_limits: false,
      supports_direct_azure_multipart: false, mime_type: file.type, entry_surface: 'chat_composer',
      store_in_library: context.storeInLibrary, library_persistence_mode: context.libraryPersistenceMode,
    };
  }

  function destination(payload, mimeType) {
    if (!payload || payload.status !== 'success' || !FILE_ID.test(payload.file_id || '')) {
      throw new Error('invalid_prepare_response');
    }
    if (payload.direct_library_upload_strategy || payload.upload_headers &&
        Object.keys(payload.upload_headers).length) throw new Error('unsupported_upload_route');
    let url;
    try { url = new URL(payload.upload_url); } catch (_) { throw new Error('invalid_upload_url'); }
    // Signed blob URLs never receive page authorization, cookies or workspace headers.
    if (url.protocol !== 'https:' || !url.hostname.endsWith('.oaiusercontent.com') ||
        url.port || url.username || url.password || url.hash || !url.search) {
      throw new Error('unsupported_upload_route');
    }
    const aws = Array.from(url.searchParams.keys()).some(key => key.toLowerCase() === 'x-amz-algorithm');
    return {
      fileId: payload.file_id, url: url.href,
      headers: aws ? { 'Content-Type': mimeType } : {
        'Content-Type': mimeType, 'x-ms-blob-type': 'BlockBlob', 'x-ms-version': '2020-04-08',
      },
    };
  }

  function processBody(fileId, file, context) {
    if (!FILE_ID.test(fileId)) throw new Error('invalid_file_id');
    return {
      file_id: fileId, file_name: file.name, use_case: context.useCase,
      index_for_retrieval: context.indexForRetrieval, entry_surface: 'chat_composer',
      library_persistence_mode: context.libraryPersistenceMode,
      metadata: { store_in_library: context.storeInLibrary, is_temporary_chat: false, is_project_thread: false },
    };
  }

  function processed(text, fileId) {
    if (!FILE_ID.test(fileId || '')) throw new Error('invalid_file_id');
    if (typeof text !== 'string' || text.length > 256 * 1024) throw new Error('invalid_process_stream');
    let count = 0;
    let complete = false;
    const metadata = {};
    const events = [];
    // The current endpoint advertises event-stream but sends newline-delimited JSON.
    for (const line of text.split(/\r?\n/)) {
      if (!line.trim()) continue;
      if (++count > 256) throw new Error('invalid_process_stream');
      let item;
      try { item = JSON.parse(line); } catch (_) { throw new Error('invalid_process_stream'); }
      if (!item || typeof item !== 'object' || Array.isArray(item) ||
          typeof item.event !== 'string' || !/^[a-z0-9_.]{1,100}$/i.test(item.event)) {
        throw new Error('invalid_process_stream');
      }
      if (item.file_id !== fileId) throw new Error('process_file_mismatch');
      const ending = item.event.split('.').pop();
      if (['error', 'cancelled', 'failed', 'unknown'].includes(ending)) throw new Error('processing_failed');
      if (!events.includes(item.event) && events.length < 20) events.push(item.event);
      // file_ready also reports 100; only the final completed event certifies processing.
      complete = item.event === 'file.processing.completed' && item.progress === 100;
      const extra = item.extra;
      if (extra && typeof extra === 'object') {
        if (Number.isSafeInteger(extra.total_tokens) && extra.total_tokens >= 0) metadata.fileTokenSize = extra.total_tokens;
        if (typeof extra.mime_type === 'string' && extra.mime_type.length <= 128) metadata.mimeType = extra.mime_type;
        if (FILE_ID.test(extra.metadata_object_id || '')) metadata.libraryFileId = extra.metadata_object_id;
        if (['library', 'temporary'].includes(extra.library_persistence_result)) {
          metadata.libraryPersistenceResult = extra.library_persistence_result;
        }
      }
    }
    if (!complete) throw new Error('processing_unconfirmed');
    return { metadata, eventCount: count, events };
  }

  return { version: 1, maxFileBytes: MAX_BYTES, prepare, destination, processBody, processed };
});
