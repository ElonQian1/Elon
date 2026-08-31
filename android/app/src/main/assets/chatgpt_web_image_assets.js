(function () {
  'use strict';

  if (window.__elonChatGptImageAssets || location.origin !== 'https://chatgpt.com') return;

  const MAX_ENTRIES = 96;
  const MAX_SOURCE_BYTES = 12 * 1024 * 1024;
  const MAX_PREVIEW_BASE64 = 1400000;
  const MAX_PREVIEW_EDGE = 1024;
  const HANDLE = /^image_[a-f0-9]{16}$/;
  const entries = new Map();
  let serial = Promise.resolve();

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
      if (url.protocol === 'https:') stable = url.origin + url.pathname;
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

  async function exportAsset(handle, emitEvent) {
    const entry = entries.get(handle);
    if (!entry) return { ok: false, error: 'unknown_handle' };
    try {
      const parsed = new URL(entry.source, location.origin);
      const response = await fetch(entry.source, {
        credentials: parsed.origin === location.origin ? 'include' : 'omit',
        cache: 'force-cache',
        redirect: 'follow'
      });
      if (!response.ok) throw new Error('http_error');
      const declaredLength = Number(response.headers.get('content-length') || 0);
      if (declaredLength > MAX_SOURCE_BYTES) throw new Error('source_too_large');
      const blob = await response.blob();
      if (!String(blob.type || '').toLowerCase().startsWith('image/')) throw new Error('not_image');
      if (blob.size <= 0 || blob.size > MAX_SOURCE_BYTES) throw new Error('source_too_large');
      const bitmap = await bitmapFromBlob(blob);
      const dimensions = boundedDimensions(Number(bitmap.width) || 1, Number(bitmap.height) || 1);
      const canvas = document.createElement('canvas');
      canvas.width = dimensions.width;
      canvas.height = dimensions.height;
      const context = canvas.getContext('2d', { alpha: false });
      if (!context) throw new Error('canvas_unavailable');
      context.fillStyle = '#000';
      context.fillRect(0, 0, canvas.width, canvas.height);
      context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      if (bitmap && typeof bitmap.close === 'function') bitmap.close();
      const encoded = canvas.toDataURL('image/jpeg', 0.84).split(',')[1] || '';
      if (!encoded || encoded.length > MAX_PREVIEW_BASE64) throw new Error('preview_too_large');
      emitEvent({
        type: 'image_asset',
        handle,
        state: 'ready',
        mediaType: 'image/jpeg',
        width: canvas.width,
        height: canvas.height,
        data: encoded
      });
      return { ok: true, error: '' };
    } catch (error) {
      const code = [
        'http_error', 'source_too_large', 'not_image', 'canvas_unavailable', 'preview_too_large'
      ].includes(String(error && error.message)) ? String(error.message) : 'fetch_failed';
      emitEvent({ type: 'image_asset', handle, state: 'failed', error: code });
      return { ok: false, error: code };
    }
  }

  function request(handle, emitEvent) {
    const normalized = String(handle || '');
    if (!HANDLE.test(normalized) || typeof emitEvent !== 'function') {
      return Promise.resolve({ ok: false, error: 'invalid_request' });
    }
    const task = serial.then(() => exportAsset(normalized, emitEvent));
    serial = task.catch(() => undefined);
    return task;
  }

  window.__elonChatGptImageAssets = Object.freeze({ describe, request, scan });
})();
