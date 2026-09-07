(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateLibraryDownload = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const MAX_BYTES = 512 * 1024 * 1024;
  const CHUNK_BYTES = 49152;
  const LIBRARY = /^libfile[_-][A-Za-z0-9_-]{1,152}$/;

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

  async function run(root, job, current, validateSignedUrl) {
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
      const url = new URL('/api/library/files/' + encodeURIComponent(job.entry.sharedLibraryFileId) + '/download', root.location.origin);
      // The official anchor uses the page's cookies. No copied bearer headers or
      // Android HTTP identity are needed for this same-origin binary request.
      response = await wait(root.fetch(url.href, {
        method: 'GET', credentials: 'same-origin', cache: 'no-store', redirect: 'follow', signal,
      }), 8000);
      check();
      if (response.status === 404) throw new Error('download_file_unavailable');
      if (!response.ok || response.status !== 200 || !response.body?.getReader) throw new Error('download_prepare_failed');
      if (response.url !== url.href) validateSignedUrl(response.url);
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
      await packet('begin', { expectedBytes }, 'ready');
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
  return Object.freeze({ version: 2, target, sharedReference, run });
});
