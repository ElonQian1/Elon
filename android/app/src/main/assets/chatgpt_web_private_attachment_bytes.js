(function (root, factory) {
  'use strict';
  const api = Object.freeze(factory());
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentBytes = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const ORIGIN = 'https://chatgpt.com';
  const VERSION = '2020-04-08';
  const MAX_PARTS = 128;
  const MAX_CONCURRENCY = 2;

  function plan(payload, file, protocol) {
    if (!payload || payload.status !== 'success' || !/^[A-Za-z0-9_-]{1,160}$/.test(payload.file_id || '')) {
      throw new Error('invalid_prepare_response');
    }
    if (typeof payload.upload_url !== 'string' || !payload.upload_url.trim()) throw new Error('invalid_upload_url');
    let url;
    try { url = new URL(payload.upload_url, ORIGIN); } catch (_) { throw new Error('invalid_upload_url'); }
    if (url.origin === ORIGIN && /^\/(?:backend-)?api\/estuary\/upload_content_bytes\/?$/.test(url.pathname)) {
      if (url.username || url.password || url.hash || payload.direct_library_upload_strategy ||
          payload.upload_headers && Object.keys(payload.upload_headers).length) throw new Error('unsupported_upload_route');
      const targets = url.searchParams.getAll('upload_url');
      if (targets.length > 1) throw new Error('unsupported_upload_route');
      // Only the evidenced signed blob destination may be forwarded to the same-origin uploader.
      if (targets.length) protocol.destination({ ...payload, upload_url: targets[0] }, file.type);
      return Object.freeze({ kind: 'estuary_bytes', fileId: payload.file_id, url: url.href,
        forwardUrl: targets[0] ?? payload.upload_url });
    }
    const strategy = payload.direct_library_upload_strategy;
    if (strategy == null) return Object.freeze({ kind: 'single', ...protocol.destination(payload, file.type) });
    const destination = protocol.destination({ ...payload, direct_library_upload_strategy: undefined }, file.type);
    const partSize = strategy.part_size_bytes, partCount = strategy.part_count, concurrency = strategy.max_part_concurrency;
    if (strategy.kind !== 'direct_azure_multipart' ||
        Object.keys(strategy).some(key => !['kind', 'part_size_bytes', 'part_count', 'max_part_concurrency'].includes(key)) ||
        !Number.isSafeInteger(file.size) || file.size < 1 || file.size > protocol.maxFileBytes ||
        !Number.isSafeInteger(partSize) || partSize < 1 || !Number.isSafeInteger(partCount) ||
        partCount < 2 || partCount > MAX_PARTS || Math.ceil(file.size / partSize) !== partCount ||
        !Number.isSafeInteger(concurrency) || concurrency < 1 ||
        Array.from(url.searchParams.keys()).some(key => key.toLowerCase() === 'x-amz-algorithm')) {
      throw new Error('unsupported_upload_route');
    }
    return Object.freeze({ kind: 'direct_azure_multipart', fileId: destination.fileId, url: destination.url,
      partSize, partCount, concurrency: Math.min(concurrency, partCount, MAX_CONCURRENCY) });
  }

  async function upload(root, destination, file, headers, owner) {
    const now = () => root.performance?.now?.() ?? Date.now();
    const deadline = now() + 30000;
    function send(url, init) {
      owner.assertCurrent();
      const remaining = Math.floor(deadline - now());
      if (remaining <= 0) throw new Error('timeout');
      return owner.dispatch(url, { ...init, redirect: 'error' }, 'none', remaining);
    }
    if (destination.kind === 'single') {
      return send(destination.url, { method: 'PUT', credentials: 'omit', headers: destination.headers, body: file });
    }
    if (destination.kind === 'estuary_bytes') {
      if (typeof root.FormData !== 'function') throw new Error('unsupported_upload_route');
      const body = new root.FormData();
      body.append('file', file);
      body.append('upload_url', destination.forwardUrl);
      const sameOriginHeaders = Object.fromEntries(Object.entries(headers).filter(([key]) => key.toLowerCase() !== 'content-type'));
      return send(destination.url, { method: 'POST', credentials: 'include', headers: sameOriginHeaders, body });
    }
    if (destination.kind !== 'direct_azure_multipart' || typeof root.btoa !== 'function') {
      throw new Error('unsupported_upload_route');
    }
    const ids = Array.from({ length: destination.partCount }, (_, index) => root.btoa(String(index).padStart(8, '0')));
    let next = 0, failure = null;
    // Workers share the existing upload owner. The first failure cancels siblings and forbids commit.
    const workers = Array.from({ length: destination.concurrency }, async () => {
      while (!failure && next < destination.partCount) {
        const index = next++;
        try {
          const partUrl = new URL(destination.url);
          partUrl.searchParams.set('comp', 'block');
          partUrl.searchParams.set('blockid', ids[index]);
          const start = index * destination.partSize;
          await send(partUrl.href, { method: 'PUT', credentials: 'omit',
            headers: { 'Content-Type': file.type, 'x-ms-version': VERSION },
            body: file.slice(start, Math.min(file.size, start + destination.partSize)) });
        } catch (error) {
          if (!failure) { failure = error; owner.abort(); }
        }
      }
    });
    await Promise.all(workers);
    if (failure) throw failure;
    owner.assertCurrent();
    const commitUrl = new URL(destination.url);
    commitUrl.searchParams.set('comp', 'blocklist');
    commitUrl.searchParams.delete('blockid');
    const body = ['<?xml version="1.0" encoding="utf-8"?>', '<BlockList>',
      ...ids.map(id => '  <Latest>' + id + '</Latest>'), '</BlockList>'].join('\n');
    return send(commitUrl.href, { method: 'PUT', credentials: 'omit', body,
      headers: { 'Content-Type': 'application/xml', 'x-ms-version': VERSION, 'x-ms-blob-content-type': file.type } });
  }

  return { version: 1, plan, upload };
});
