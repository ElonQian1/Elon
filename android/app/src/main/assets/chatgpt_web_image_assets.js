(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptImageAssets;
  if (existing && Number(existing.version) >= 2) return;
  if (existing && typeof existing.dispose === 'function') existing.dispose();

  const MAX_ENTRIES = 96;
  const MAX_SOURCE_BYTES = 12 * 1024 * 1024;
  const MAX_PREVIEW_BASE64 = 1400000;
  const MAX_PREVIEW_EDGE = 1024;
  const HANDLE = /^image_[a-f0-9]{16}$/;
  const entries = new Map();
  const pending = new Map();
  const queue = [];
  const MAX_PENDING = 16;
  const MAX_ACTIVE = 2;
  const REQUEST_TIMEOUT_MS = 8000;
  let activeCount = 0;
  let disposed = false;

  function sourceFor(node) {
    if (!(node instanceof HTMLImageElement)) return '';
    const raw = String(node.currentSrc || node.getAttribute('src') || '').trim();
    if (!raw) return '';
    try {
      const url = new URL(raw, location.origin);
      if (url.protocol === 'https:') return url.href;
      if (url.protocol === 'blob:' && url.href.startsWith('blob:https://chatgpt.com/')) {
        return url.href;
      }
    } catch (_) {}
    return '';
  }

  function fnv32(value, seed) {
    let hash = seed >>> 0;
    for (let index = 0; index < value.length; index += 1) {
      hash ^= value.charCodeAt(index);
      hash = Math.imul(hash, 0x01000193) >>> 0;
    }
    return hash.toString(16).padStart(8, '0');
  }

  function handleFor(source) {
    let stable = source;
    try {
      const url = new URL(source);
      if (url.protocol === 'https:') {
        const identity = new URLSearchParams();
        url.searchParams.forEach((value, key) => {
          // Preserve query-addressed images while dropping expiring authorization material.
          if (!/^(?:token|access_token|signature|expires|exp|sig|se|sp|sv|sr|st|skt|ske|sks|skv|skoid|sktid|x-amz-.*|x-goog-.*)$/i.test(key)) {
            identity.append(key, value);
          }
        });
        identity.sort();
        stable = url.origin + url.pathname + (identity.size ? '?' + identity.toString() : '');
      }
    } catch (_) {}
    return 'image_' + fnv32(stable, 0x811c9dc5) + fnv32(stable, 0x9e3779b9);
  }

  function remember(handle, source, node) {
    entries.delete(handle);
    entries.set(handle, { source, node });
    while (entries.size > MAX_ENTRIES) {
      entries.delete(entries.keys().next().value);
    }
  }

  function describe(node) {
    const source = sourceFor(node);
    if (!source) return {};
    const handle = handleFor(source);
    remember(handle, source, node);
    return {
      assetHandle: handle,
      imageWidth: Math.max(0, Math.min(4096, Number(node.naturalWidth) || 0)),
      imageHeight: Math.max(0, Math.min(4096, Number(node.naturalHeight) || 0))
    };
  }

  function imageNodes(root) {
    return Array.from((root || document).querySelectorAll('img')).filter((node) => {
      const source = sourceFor(node);
      const width = Number(node.naturalWidth || node.width || 0);
      const height = Number(node.naturalHeight || node.height || 0);
      return !!source && width >= 120 && height >= 120;
    }).slice(0, MAX_ENTRIES);
  }

  function scan(root) {
    return imageNodes(root).map(describe).filter((value) => HANDLE.test(value.assetHandle || ''));
  }

  function boundedDimensions(width, height) {
    const longest = Math.max(1, width, height);
    const scale = Math.min(1, MAX_PREVIEW_EDGE / longest);
    return {
      width: Math.max(1, Math.round(width * scale)),
      height: Math.max(1, Math.round(height * scale))
    };
  }

  async function bitmapFromBlob(blob) {
    if (typeof createImageBitmap === 'function') return createImageBitmap(blob);
    const objectUrl = URL.createObjectURL(blob);
    try {
      const image = new Image();
      image.decoding = 'async';
      image.src = objectUrl;
      await image.decode();
      return image;
    } finally {
      URL.revokeObjectURL(objectUrl);
    }
  }

  async function sourceBlob(response, job) {
    const reader = response.body && typeof response.body.getReader === 'function'
      ? response.body.getReader() : null;
    if (!reader) return response.blob();
    job.reader = reader;
    const chunks = [];
    let size = 0;
    try {
      while (!job.done) {
        const item = await reader.read();
        if (job.done) throw new Error('cancelled');
        if (item.done) break;
        size += item.value.byteLength;
        if (size > MAX_SOURCE_BYTES) throw new Error('source_too_large');
        chunks.push(item.value);
      }
      return new Blob(chunks, { type: response.headers.get('content-type') || '' });
    } finally {
      job.reader = null;
      if (job.done || size > MAX_SOURCE_BYTES) {
        try { Promise.resolve(reader.cancel()).catch(() => {}); } catch (_) {}
      }
      try { reader.releaseLock(); } catch (_) {}
    }
  }

  async function exportAsset(job) {
    const { handle, entry } = job;
    let bitmap = null;
    let canvas = null;
    try {
      const parsed = new URL(entry.source, location.origin);
      const response = await fetch(entry.source, {
        credentials: parsed.origin === location.origin ? 'include' : 'omit',
        cache: 'force-cache',
        redirect: 'follow',
        signal: job.controller ? job.controller.signal : undefined
      });
      if (job.done) return;
      if (!response.ok) throw new Error('http_error');
      const declaredLength = Number(response.headers.get('content-length') || 0);
      if (declaredLength > MAX_SOURCE_BYTES) throw new Error('source_too_large');
      const blob = await sourceBlob(response, job);
      if (job.done) return;
      if (!String(blob.type || '').toLowerCase().startsWith('image/')) throw new Error('not_image');
      if (blob.size <= 0 || blob.size > MAX_SOURCE_BYTES) throw new Error('source_too_large');
      bitmap = await bitmapFromBlob(blob);
      if (job.done) return;
      const dimensions = boundedDimensions(Number(bitmap.width) || 1, Number(bitmap.height) || 1);
      canvas = document.createElement('canvas');
      canvas.width = dimensions.width;
      canvas.height = dimensions.height;
      const context = canvas.getContext('2d', { alpha: false });
      if (!context) throw new Error('canvas_unavailable');
      context.fillStyle = '#000';
      context.fillRect(0, 0, canvas.width, canvas.height);
      context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      const encoded = canvas.toDataURL('image/jpeg', 0.84).split(',')[1] || '';
      if (!encoded || encoded.length > MAX_PREVIEW_BASE64) throw new Error('preview_too_large');
      finish(job, { ok: true, error: '' }, {
        type: 'image_asset',
        handle,
        state: 'ready',
        mediaType: 'image/jpeg',
        width: canvas.width,
        height: canvas.height,
        data: encoded
      });
    } catch (error) {
      const code = [
        'http_error', 'source_too_large', 'not_image', 'canvas_unavailable', 'preview_too_large'
      ].includes(String(error && error.message)) ? String(error.message) : 'fetch_failed';
      finish(job, { ok: false, error: code });
    } finally {
      if (bitmap && typeof bitmap.close === 'function') bitmap.close();
      if (canvas) { canvas.width = 0; canvas.height = 0; }
    }
  }

  function finish(job, result, event) {
    if (job.done) return;
    job.done = true;
    if (job.timer != null) window.clearTimeout(job.timer);
    if (job.controller) job.controller.abort();
    if (job.reader) {
      try { Promise.resolve(job.reader.cancel()).catch(() => {}); } catch (_) {}
    }
    pending.delete(job.handle);
    if (job.started) activeCount -= 1;
    const value = event || { type: 'image_asset', handle: job.handle, state: 'failed', error: result.error };
    if (!disposed) job.listeners.forEach((listener) => { try { listener(value); } catch (_) {} });
    job.listeners.clear();
    job.resolve(result);
    pump();
  }

  function pump() {
    while (!disposed && activeCount < MAX_ACTIVE && queue.length) {
      const job = queue.shift();
      if (job.done) continue;
      job.started = true;
      activeCount += 1;
      job.timer = window.setTimeout(() => finish(job, { ok: false, error: 'timeout' }), REQUEST_TIMEOUT_MS);
      exportAsset(job);
    }
  }

  function request(handle, emitEvent) {
    const normalized = String(handle || '');
    if (!HANDLE.test(normalized) || typeof emitEvent !== 'function') {
      return Promise.resolve({ ok: false, error: 'invalid_request' });
    }
    if (disposed) return Promise.resolve({ ok: false, error: 'cancelled' });
    const current = pending.get(normalized);
    if (current) { current.listeners.add(emitEvent); return current.promise; }
    const entry = entries.get(normalized);
    if (!entry) return Promise.resolve({ ok: false, error: 'unknown_handle' });
    if (pending.size >= MAX_PENDING) return Promise.resolve({ ok: false, error: 'busy' });
    const job = {
      handle: normalized, entry, listeners: new Set([emitEvent]), done: false, started: false,
      controller: typeof AbortController === 'function' ? new AbortController() : null
    };
    job.promise = new Promise((resolve) => { job.resolve = resolve; });
    pending.set(normalized, job);
    queue.push(job);
    pump();
    return job.promise;
  }

  function dispose() {
    if (disposed) return;
    disposed = true;
    Array.from(pending.values()).forEach((job) => finish(job, { ok: false, error: 'cancelled' }));
    queue.length = 0;
    entries.clear();
  }

  window.__elonChatGptImageAssets = Object.freeze({ version: 2, describe, request, scan, dispose });
})();
