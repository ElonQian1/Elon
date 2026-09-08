(function (root, factory) {
  'use strict';
  const contentSource = typeof module === 'object' && module.exports
    ? require('./chatgpt_web_private_content_source.js') : root?.__elonChatGptPrivateContentSource;
  const api = factory(contentSource);
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryDownload = api;
})(typeof window === 'object' ? window : null, function (contentSource) {
  'use strict';
  const MAX_BYTES = 512 * 1024 * 1024;
  const CHUNK_BYTES = 49152;
  const LIBRARY = /^libfile[_-][A-Za-z0-9_-]{1,152}$/;
  const EXPORTS = Object.freeze({
    'application/vnd.google-apps.document': ['docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
    'application/vnd.google-apps.spreadsheet': ['xlsx', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
    'application/vnd.google-apps.presentation': ['pptx', 'application/vnd.openxmlformats-officedocument.presentationml.presentation'],
  });

  function target(file) {
    if (!file || file.source !== 'library' || !LIBRARY.test(file.library_file_id || '') ||
        file.id != null && file.id !== file.library_file_id ||
        typeof file.name !== 'string' || !file.name.trim() ||
        ['mounted_library_file_id', 'shared_library_file_reference', 'preview_file',
          'context_connector_info', 'source_url', 'connector_id', 'context_connector',
          'shared_library_file_id', 'library_download_id'].some(key => file[key] != null && file[key] !== '')) return null;
    // Gar's library reference has no ordinary file ID. The preview resolver also
    // recognizes a library ID equal to the file ID; neither is a files/download ID.
    return { sharedLibraryFileId: file.library_file_id };
  }

  function sharedReference(file) {
    if (!file || typeof file !== 'object' || Array.isArray(file) ||
        typeof file.library_file_id !== 'string' || !LIBRARY.test(file.library_file_id) ||
        typeof file.name !== 'string' || !file.name.trim() || /[\x00-\x1f\x7f]/.test(file.name) ||
        Object.keys(file).some(key => !['library_file_id', 'name', 'display_path', 'mime_type',
          'size_bytes', 'entrypoint'].includes(key)) ||
        file.mime_type != null && (typeof file.mime_type !== 'string' ||
          !/^[A-Za-z0-9.+-]{1,63}\/[A-Za-z0-9.+-]{1,63}$/.test(file.mime_type)) ||
        file.size_bytes != null && (!Number.isSafeInteger(file.size_bytes) || file.size_bytes < 0)) return null;
    // attachSharedLibraryFileReference -> metadata.shared_library_file_references
    // -> document reference preview's sharedLibraryFileId -> fEt/AX binary route.
    return { sharedLibraryFileId: file.library_file_id };
  }

  function sharePointFile(id) {
    const prefix = 'external-sharepoint:file:v1:item:';
    if (!id.startsWith(prefix)) return false;
    const parts = id.slice(prefix.length).split(':');
    // BQ/Ajt: two canonical base64url components, not URLs or container IDs.
    return parts.length === 2 && parts.every(part => {
      if (!/^[A-Za-z0-9_-]{1,683}$/.test(part)) return false;
      try {
        const encoded = part.replace(/-/g, '+').replace(/_/g, '/');
        const decoded = atob(encoded + '='.repeat((4 - encoded.length % 4) % 4));
        return /^[A-Za-z0-9._!~:-]{1,512}$/.test(decoded) &&
          btoa(decoded).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '') === part;
      } catch (_) { return false; }
    });
  }

  function mountedTarget(file, reference = false) {
    if (!file || typeof file !== 'object' || Array.isArray(file) ||
        typeof file.mounted_library_file_id !== 'string' ||
        typeof file.name !== 'string' || !file.name.trim() || file.name.length > 1024 ||
        /[\x00-\x1f\x7f]/.test(file.name)) return null;
    const id = file.mounted_library_file_id;
    // Pjt and BQ admit concrete provider files; a1n/zQ materializes their IDs.
    const sharePoint = sharePointFile(id);
    const match = /^external-(gdrive|box|dropbox):(?:account:([A-Za-z0-9_-]{1,512}):)?file:(\S{1,512})$/.exec(id);
    if (!sharePoint && (!match || match[2] && match[1] !== 'gdrive')) return null;
    const provider = sharePoint ? 'sharepoint' : { gdrive: 'google_drive', box: 'box', dropbox: 'dropbox' }[match[1]];
    if (!sharePoint && !(match[1] === 'gdrive' ? /^[A-Za-z0-9_-]{5,512}$/.test(match[3]) :
      match[1] === 'box' ? /^[1-9][0-9]*$/.test(match[3]) :
        match[3].length <= 256 && /^id:[A-Za-z0-9_:-]+$/.test(match[3]))) return null;
    if (reference) {
      if (Object.keys(file).some(key => !['mounted_library_file_id', 'name'].includes(key))) return null;
    } else if (file.source !== 'library' || file.id != null && file.id !== id ||
        file.library_file_id != null && file.library_file_id !== id ||
        file.library_provider != null && file.library_provider !== provider ||
        ['preview_file', 'context_connector_info', 'shared_library_file_reference', 'source_url',
          'context_connector', 'connector_id', 'shared_library_file_id', 'library_download_id']
          .some(key => file[key] != null && file[key] !== '')) return null;
    for (const value of [file.mime_type, file.mounted_library_mime_type]) {
      if (value != null && (typeof value !== 'string' ||
          !/^[A-Za-z0-9.+-]{1,63}\/[A-Za-z0-9.+-]{1,63}$/.test(value))) return null;
    }
    // SEt prefers mounted source MIME; cB/n$/fEt permit only these Drive exports.
    const sourceMime = (file.mounted_library_mime_type || file.mime_type || '').toLowerCase();
    if (sourceMime.startsWith('application/vnd.google-apps.') &&
        (provider !== 'google_drive' || !EXPORTS[sourceMime])) return null;
    if (provider === 'box' && /\.(?:boxnote|boxcanvas|gdoc|gsheet|gslide|gslides)$/i.test(file.name.trimEnd())) return null;
    return { mountedFileId: id, mountedMediaType: sourceMime };
  }

  async function materialize(root, job, current) {
    const check = () => { if (!current(job)) throw new Error('download_cancelled'); };
    check();
    if (root.location.origin !== 'https://chatgpt.com' || !job.entry.mountedFileId ||
        !root.__elonChatGptPrivateJsonRequest?.request) throw new Error('download_source_unsupported');
    const sourceMime = job.entry.mountedMediaType || job.entry.mediaType;
    const exported = EXPORTS[sourceMime];
    const canResolve = job.descriptor.resolvedFileVersion === 1;
    if (exported && !canResolve) throw new Error('download_bridge_unavailable');
    // qR + a1n: materialize without retrieval indexing, then authorize its returned
    // ordinary file. The caller consumes the selection before this single POST.
    const result = await root.__elonChatGptPrivateJsonRequest.request(root,
      new URL('/backend-api/files/library/mounted/materialize', root.location.origin).href, {
        method: 'POST', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
        headers: { ...root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders(), 'Content-Type': 'application/json' },
        signal: job.controller.signal, body: JSON.stringify({ file_id: job.entry.mountedFileId,
          name: job.entry.name, mime_type: sourceMime || null, index_for_retrieval: false }),
      }, { timeoutMs: 6000, maxBytes: 65536 });
    check();
    const value = result.payload;
    if (typeof value?.file_id !== 'string' || !/^[A-Za-z0-9_-]{1,160}$/.test(value.file_id) ||
        typeof value.file_name !== 'string' || !value.file_name.trim() || value.file_name.length > 1024 ||
        /[\x00-\x1f\x7f]/.test(value.file_name) ||
        value.file_size_bytes != null && (!Number.isSafeInteger(value.file_size_bytes) || value.file_size_bytes < 0) ||
        value.mime_type != null && (typeof value.mime_type !== 'string' ||
          !/^[A-Za-z0-9.+-]{1,63}\/[A-Za-z0-9.+-]{1,63}$/.test(value.mime_type))) {
      throw new Error('download_prepare_failed');
    }
    if (value.file_size_bytes > MAX_BYTES) throw new Error('download_file_too_large');
    const name = value.file_name.replace(/\u00a0/g, ' ').trim();
    const mediaType = (value.mime_type || exported?.[1] || job.entry.mediaType || '').toLowerCase();
    if (exported && (!name.toLowerCase().endsWith('.' + exported[0]) || mediaType !== exported[1])) {
      throw new Error('download_content_invalid');
    }
    // Old APKs cannot accept a resolved name/type. New APKs bind both to the
    // consumed lease before either native storage or DownloadManager is opened.
    if (!canResolve && (name.slice(0, 180) !== job.entry.name ||
        job.entry.mediaType && value.mime_type && mediaType !== job.entry.mediaType.toLowerCase())) {
      throw new Error('download_source_unsupported');
    }
    return { fileId: value.file_id, mediaType, ...(canResolve
      ? { resolvedFile: Object.freeze({ version: 1, name, mediaType }) } : {}) };
  }

  function contentUrl(value) {
    return contentSource?.contentUrl(value) || null;
  }

  function run(root, job, current, validateSignedUrl, libraryFileId = job.entry.sharedLibraryFileId) {
    if (typeof libraryFileId !== 'string' || !LIBRARY.test(libraryFileId)) throw new Error('download_source_unsupported');
    const url = new URL('/api/library/files/' + encodeURIComponent(libraryFileId) + '/download', root.location.origin);
    return transfer(root, job, current, validateSignedUrl, url.href, false);
  }

  function runContent(root, job, current, value) {
    const url = contentUrl(value);
    if (root.location.origin !== 'https://chatgpt.com' || !url) throw new Error('download_source_unsupported');
    return transfer(root, job, current, null, url, true);
  }

  async function transfer(root, job, current, validateSignedUrl, url, content) {
    if (job.descriptor.byteTransferVersion !== 1) throw new Error('download_bridge_unavailable');
    const bridge = root.elonChatGptFileDownload;
    if (!bridge?.postMessage) throw new Error('download_bridge_unavailable');
    const signal = job.controller.signal;
    let reader, response, pending, total = 0, sequence = 0, emptyReads = 0;
    const previous = bridge.onmessage;
    function check() { if (signal.aborted || !current(job)) throw new Error('download_cancelled'); }
    function wait(promise, milliseconds) {
      return new Promise((resolve, reject) => {
        let settled = false;
        const finish = (error, value) => {
          if (settled) return;
          settled = true;
          root.clearTimeout(timer);
          signal.removeEventListener('abort', cancelled);
          if (error) reject(error); else resolve(value);
        };
        const cancelled = () => finish(new Error('download_cancelled'));
        const timer = root.setTimeout(() => finish(new Error('download_transfer_timeout')), milliseconds);
        signal.addEventListener('abort', cancelled, { once: true });
        if (signal.aborted) cancelled();
        Promise.resolve(promise).then(value => finish(null, value), error => finish(error));
      });
    }
    const listener = event => {
      let result;
      try { result = JSON.parse(event.data); } catch (_) { return; }
      if (result.leaseId === job.descriptor.leaseId && result.state === 'cancelled') {
        job.controller.abort();
        return;
      }
      if (!pending || result.leaseId !== job.descriptor.leaseId ||
          result.byteOperation !== pending.operation || result.sequence !== pending.sequence) return;
      const owner = pending;
      pending = null;
      if (result.state !== owner.expected) owner.reject(new Error('download_storage_failed'));
      else owner.resolve(result);
    };
    async function packet(operation, values, expected) {
      check();
      const acknowledgement = new Promise((resolve, reject) => {
        pending = { operation, sequence, expected, resolve, reject };
        try {
          bridge.postMessage(JSON.stringify({ leaseId: job.descriptor.leaseId,
            documentToken: job.entry.token, byteOperation: operation, sequence, ...values }));
        } catch (error) { reject(error); }
      });
      try { await wait(acknowledgement, 8000); check(); }
      catch (error) {
        if (operation === 'commit' && error?.message === 'download_transfer_timeout') {
          throw new Error('download_confirmation_unknown');
        }
        throw error;
      } finally { pending = null; }
    }
    bridge.onmessage = listener;
    try {
      check();
      // Official download anchors use ambient cookies. Keep identity and URLs in
      // the page; the native lease receives only bounded bytes and acknowledgements.
      response = await wait(root.fetch(url, {
        method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: content ? 'error' : 'follow', signal,
      }), 8000);
      check();
      if (response.status === 404) throw new Error('download_file_unavailable');
      if (!response.ok || response.status !== 200 || !response.body?.getReader) throw new Error('download_prepare_failed');
      if (response.url !== url) {
        if (content) throw new Error('download_source_unsupported');
        validateSignedUrl(response.url);
      }
      const mime = String(response.headers.get('content-type') || '').split(';')[0].trim().toLowerCase();
      const attachment = /^attachment(?:;|$)/i.test(response.headers.get('content-disposition') || '');
      if (mime === 'text/html' && !attachment || mime === 'application/json' && !attachment &&
          job.entry.mediaType !== 'application/json') throw new Error('download_content_invalid');
      const rawLength = response.headers.get('content-length');
      const encoded = response.headers.get('content-encoding');
      let expectedBytes = -1;
      if (rawLength != null && (!encoded || encoded === 'identity')) {
        if (!/^\d{1,16}$/.test(rawLength) || !Number.isSafeInteger(Number(rawLength))) throw new Error('download_content_invalid');
        expectedBytes = Number(rawLength);
        if (expectedBytes > MAX_BYTES) throw new Error('download_file_too_large');
      }
      reader = response.body.getReader();
      await packet('begin', { expectedBytes,
        ...(job.entry.resolvedFile ? { resolvedFile: job.entry.resolvedFile } : {}) }, 'ready');
      while (true) {
        check();
        const next = await wait(reader.read(), 20000);
        check();
        if (next.done) break;
        if (!(next.value instanceof Uint8Array)) throw new Error('download_content_invalid');
        emptyReads = next.value.byteLength ? 0 : emptyReads + 1;
        if (emptyReads > 16) throw new Error('download_content_invalid');
        if (total + next.value.byteLength > MAX_BYTES || expectedBytes >= 0 && total + next.value.byteLength > expectedBytes) {
          throw new Error('download_file_too_large');
        }
        for (let offset = 0; offset < next.value.byteLength; offset += CHUNK_BYTES) {
          const bytes = next.value.subarray(offset, offset + CHUNK_BYTES);
          const data = root.btoa(String.fromCharCode(...bytes));
          await packet('chunk', { data }, 'written');
          total += bytes.byteLength;
          sequence += 1;
        }
      }
      if (expectedBytes >= 0 && total !== expectedBytes) throw new Error('download_content_invalid');
      await packet('commit', { totalBytes: total }, 'saved');
      return 'download_saved';
    } finally {
      pending = null;
      if (bridge.onmessage === listener) bridge.onmessage = previous;
      try { await wait(reader ? reader.cancel() : response?.body?.cancel(), 1000); } catch (_) {}
      try { reader?.releaseLock(); } catch (_) {}
    }
  }
  return Object.freeze({ version: 7, target, sharedReference, mountedTarget, materialize, contentUrl, run, runContent });
});
