(function (root, factory) {
  'use strict';
  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateMountedLibraryAttachment = exported;
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const MAX_BYTES = 8 * 1024 * 1024;
  const IMAGE_TYPES = ['image/jpeg', 'image/png', 'image/webp'];
  const DRIVE_TYPES = ['application/vnd.google-apps.document', 'application/vnd.google-apps.spreadsheet',
    'application/vnd.google-apps.presentation'];

  function metadata(name, type, size, source = false) {
    if (typeof name !== 'string' || !name.trim() || name.length > 120 || /[\x00-\x1f\x7f/\\]/.test(name) ||
        size != null && (!Number.isSafeInteger(size) || size < 0 || size > MAX_BYTES)) return null;
    const file = { name, type, size: size ?? 0 };
    return IMAGE_TYPES.includes(type) || root.__elonChatGptPrivateAttachmentProtocol?.isDocument(file) === true ||
      source && DRIVE_TYPES.includes(type) ? file : null;
  }

  function descriptor(source) {
    return root.__elonChatGptPrivateLibraryDownload?.catalogTarget(source)
      ? metadata(source.name, source.mime_type, source.file_size_bytes, true) : null;
  }

  async function prepare(source, requestId, signal, current) {
    const check = () => {
      if (signal.aborted || !current()) throw new Error('library_attachment_context_changed');
    };
    check();
    if (!descriptor(source) || !/^mcp_[a-z0-9]{1,32}$/.test(requestId || '') ||
        root.location.origin !== 'https://chatgpt.com') throw new Error('library_file_unsupported');
    // Official onAttachMountedLibraryFile -> qR defaults retrieval indexing to true.
    // Download's index_for_retrieval:false cannot prepare a composer attachment.
    const response = await root.__elonChatGptPrivateJsonRequest.request(root,
      'https://chatgpt.com/backend-api/files/library/mounted/materialize', {
        method: 'POST', credentials: 'same-origin', cache: 'no-store', redirect: 'error', signal,
        headers: { ...root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), 'Content-Type': 'application/json' },
        body: JSON.stringify({ file_id: source.id, name: source.name,
          mime_type: source.mime_type, index_for_retrieval: true }),
      }, { timeoutMs: 12000, maxBytes: 65536 });
    check();
    const value = response.payload;
    const file = metadata(value?.file_name, value?.mime_type || source.mime_type, value?.file_size_bytes);
    if (!file || !/^file[_-][A-Za-z0-9_-]{1,155}$/.test(value?.file_id || '') ||
        value.preview_file != null && (typeof value.preview_file !== 'object' || Array.isArray(value.preview_file))) {
      throw new Error('library_attachment_unconfirmed');
    }
    const provider = { gdrive: 'google_drive', box: 'box', dropbox: 'dropbox', sharepoint: 'sharepoint' }[
      /^external-([^:]+):/.exec(source.id)?.[1]];
    // Preserve both identities: the prepared file is sent; the mounted ID supplies provenance.
    return { descriptor: file, item: { id: 'private_attachment_' + requestId, attached: {
      tempId: 'native_library_' + requestId, status: 'ready', file: new root.File([], file.name, { type: file.type }),
      fileId: value.file_id, cdnUrl: null, progress: 100, source: 'library',
      mountedLibraryFileId: source.id, mountedLibraryMimeType: source.mime_type,
      libraryProvider: provider, libraryEntrypoint: 'composer_library_picker', isBigPaste: false,
      ...(value.preview_file == null ? {} : { previewFile: value.preview_file }),
      fileSpec: { id: value.file_id, name: file.name, size: file.size, mimeType: file.type, isBigPaste: false,
        ...(IMAGE_TYPES.includes(file.type) ? { width: 512, height: 512 } : {}) },
    } } };
  }
  return Object.freeze({ descriptor, prepare });
});
