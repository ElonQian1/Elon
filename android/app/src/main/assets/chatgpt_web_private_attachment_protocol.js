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
  const PROJECT_ID = /^g-p-[a-f0-9]{32}$/i;
  const USE_CASES = new Set(['ace_upload', 'my_files', 'multimodal', 'gizmo']);
  const IMAGE_TYPES = new Set(['image/jpeg', 'image/png', 'image/webp']);
  // These document categories use the inspected composer file transaction, not media conversion.
  const documentMimeTypes = Object.freeze([
    'text/plain', 'application/pdf', 'application/msword',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'application/vnd.ms-powerpoint',
    'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    'application/vnd.ms-excel',
    'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    'application/vnd.oasis.opendocument.text', 'application/rtf', 'text/rtf',
    'text/csv', 'text/tab-separated-values', 'text/markdown', 'application/json',
    'application/xml', 'text/xml', 'text/html',
  ]);

  function isDocument(file) { return documentMimeTypes.includes(file?.type); }

  function isPdf(file) {
    return file?.type === 'application/pdf' || /\.pdf$/i.test(file?.name || '');
  }

  function creationHeaders(file, context) {
    if (!isPdf(file)) return {};
    const slug = context?.modelSlug;
    if (typeof slug !== 'string' || !/^[a-z0-9][a-z0-9._-]{0,127}$/i.test(slug)) {
      throw new Error('unsupported_upload_context');
    }
    // The official PDF create request carries the composer model, not its UI label.
    return { 'x-oai-model-slug': slug };
  }

  function imageDimensions(value) {
    if (!Number.isSafeInteger(value?.width) || !Number.isSafeInteger(value?.height) ||
        value.width < 1 || value.height < 1 || value.width > 2048 || value.height > 2048) {
      throw new Error('unsupported_upload_context');
    }
    return Object.freeze({ width: value.width, height: value.height });
  }

  function originInfo(value) {
    const hasOrigin = value?.origination_thread_id !== undefined || value?.origination_message_id !== undefined;
    const uuid = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
    if (hasOrigin && (!uuid.test(value.origination_thread_id || '') || !uuid.test(value.origination_message_id || ''))) {
      throw new Error('unsupported_upload_context');
    }
    return hasOrigin ? { origination_thread_id: value.origination_thread_id,
      origination_message_id: value.origination_message_id } : {};
  }

  function projectInfo(context) {
    const value = context?.libraryFileInfo;
    if (!context?.isProjectThread) {
      if (context?.gizmoId || value || context?.projectScopeId || context?.useCase === 'gizmo') {
        throw new Error('unsupported_upload_context');
      }
      return null;
    }
    if (context.isProjectThread !== true || context.isTemporaryChat ||
        context.projectScopeId !== undefined && !PROJECT_ID.test(context.projectScopeId)) {
      throw new Error('unsupported_upload_context');
    }
    // Official chat-only project uploads keep conversation origins, not project-write metadata.
    if (value?.gizmo_id == null) {
      if (!PROJECT_ID.test(context.projectScopeId || '') || context.gizmoId != null ||
          !['ace_upload', 'multimodal'].includes(context.useCase) || value != null &&
          (typeof value !== 'object' || Array.isArray(value) || Object.keys(value).some(key =>
            !['origination_thread_id', 'origination_message_id'].includes(key)))) {
        throw new Error('unsupported_upload_context');
      }
      return Object.freeze(originInfo(value));
    }
    if (!PROJECT_ID.test(value.gizmo_id) || value.is_project !== true ||
        context.projectScopeId !== undefined && context.projectScopeId !== value.gizmo_id ||
        value.should_upload_to_project !== true ||
        Object.keys(value).some(key => !['gizmo_id', 'is_project', 'should_upload_to_project',
          'origination_thread_id', 'origination_message_id'].includes(key)) ||
        (context.useCase === 'gizmo' ? context.gizmoId !== value.gizmo_id :
          context.useCase !== 'multimodal' || context.gizmoId != null)) throw new Error('unsupported_upload_context');
    return Object.freeze({ gizmo_id: value.gizmo_id, is_project: true, should_upload_to_project: true,
      ...originInfo(value) });
  }

  function prepare(file, context) {
    if (!file || !Number.isSafeInteger(file.size) || file.size < 1 || file.size > MAX_BYTES ||
        typeof file.slice !== 'function') throw new Error('invalid_file');
    if (typeof file.name !== 'string' || !file.name.trim() || file.name.length > 120 ||
        /[\x00-\x1f\x7f/\\]/.test(file.name)) throw new Error('invalid_file_name');
    const temporary = context?.isTemporaryChat === true;
    if (!context || !USE_CASES.has(context.useCase) || typeof context.storeInLibrary !== 'boolean' ||
        context.libraryPersistenceMode !== (temporary ? undefined : 'required') ||
        typeof context.indexForRetrieval !== 'boolean' ||
        context.isTemporaryChat !== undefined && typeof context.isTemporaryChat !== 'boolean' ||
        temporary && (context.storeInLibrary || context.indexForRetrieval)) {
      throw new Error('unsupported_upload_context');
    }
    if (context.directoryId) {
      throw new Error('unsupported_upload_context');
    }
    const project = projectInfo(context);
    if (/^image\//i.test(file.type)) {
      if (!IMAGE_TYPES.has(file.type) || context.useCase !== 'multimodal' || context.indexForRetrieval && !project) {
        throw new Error('unsupported_upload_context');
      }
      imageDimensions(context.imageDimensions);
    } else if (context.useCase === 'multimodal' || context.imageDimensions != null) {
      throw new Error('unsupported_upload_context');
    }
    if (typeof file.type !== 'string' || !/^[\w.+-]+\/[\w.+-]+$/.test(file.type)) {
      throw new Error('invalid_mime_type');
    }
    return {
      file_name: file.name, file_size: file.size, use_case: context.useCase,
      timezone_offset_min: new Date().getTimezoneOffset(), reset_rate_limits: false,
      supports_direct_azure_multipart: false, mime_type: file.type, entry_surface: 'chat_composer',
      store_in_library: context.storeInLibrary,
      ...(project && context.useCase === 'gizmo' ? { gizmo_id: project.gizmo_id } : {}),
      ...(context.libraryPersistenceMode == null ? {} : { library_persistence_mode: context.libraryPersistenceMode }),
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
    const project = projectInfo(context);
    return {
      file_id: fileId, file_name: file.name, use_case: context.useCase,
      index_for_retrieval: context.indexForRetrieval, entry_surface: 'chat_composer',
      ...(project && context.useCase === 'gizmo' ? { gizmo_id: project.gizmo_id } : {}),
      ...(context.libraryPersistenceMode == null ? {} : { library_persistence_mode: context.libraryPersistenceMode }),
      metadata: { store_in_library: context.storeInLibrary,
        is_temporary_chat: context.isTemporaryChat === true, is_project_thread: !!project,
        ...(project && Object.keys(project).length ? { library_file_info: project } : {}) },
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

  return { version: 9, maxFileBytes: MAX_BYTES, prepare, destination, processBody, processed,
    imageDimensions, projectInfo, isPdf, creationHeaders, documentMimeTypes, isDocument };
});
