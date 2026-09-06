(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateAttachmentImage = api;
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  const types = new Set(['image/jpeg', 'image/png', 'image/webp']);
  const edge = 2048;

  function available(value) {
    return types.has(value?.type) && Number.isSafeInteger(value.width) && Number.isSafeInteger(value.height) &&
      value.width > 0 && value.height > 0 && value.width <= 16384 && value.height <= 16384 &&
      value.width * value.height <= 4_000_000 && typeof root.createImageBitmap === 'function' &&
      typeof root.File === 'function';
  }

  async function prepare(file, descriptor, signal) {
    if (!available(descriptor) || file?.type !== descriptor.type || file?.size !== descriptor.size ||
        file.size < 1 || file.size > 8 * 1024 * 1024) throw new Error('image_source_invalid');
    let bitmap, canvas, timer, abort;
    let ended = false;
    const current = () => {
      if (ended || signal?.aborted) throw new Error('cancelled');
    };
    const work = async () => {
      current();
      bitmap = await root.createImageBitmap(file);
      // A decoder may finish after cancellation/deadline. Its late bitmap must
      // still be released; it cannot trigger encoding or an upload.
      try {
        current();
        if (bitmap.width !== descriptor.width || bitmap.height !== descriptor.height) {
          throw new Error('image_dimensions_mismatch');
        }
        const ratio = Math.min(1, edge / Math.max(bitmap.width, bitmap.height));
        const dimensions = Object.freeze({ width: Math.max(1, Math.trunc(bitmap.width * ratio)),
          height: Math.max(1, Math.trunc(bitmap.height * ratio)) });
        if (ratio === 1 && file.size <= 1024 * 1024) return { file, dimensions };
        canvas = typeof root.OffscreenCanvas === 'function'
          ? new root.OffscreenCanvas(dimensions.width, dimensions.height)
          : root.document?.createElement('canvas');
        if (!canvas) throw new Error('image_encoder_unavailable');
        canvas.width = dimensions.width;
        canvas.height = dimensions.height;
        const context = canvas.getContext('2d');
        if (!context) throw new Error('image_encoder_unavailable');
        context.drawImage(bitmap, 0, 0, dimensions.width, dimensions.height);
        const blob = typeof canvas.convertToBlob === 'function'
          ? await canvas.convertToBlob({ type: file.type, quality: 0.9 })
          : await new Promise((resolve, reject) => {
            if (typeof canvas.toBlob !== 'function') return reject(new Error('image_encoder_unavailable'));
            canvas.toBlob(resolve, file.type, 0.9);
          });
        current();
        if (!blob || blob.type !== file.type || blob.size < 1 || blob.size > 8 * 1024 * 1024) {
          throw new Error('image_encoding_invalid');
        }
        return { file: new root.File([blob], file.name, { type: blob.type, lastModified: file.lastModified }), dimensions };
      } finally {
        bitmap?.close();
        bitmap = null;
        if (canvas) { canvas.width = 1; canvas.height = 1; canvas = null; }
      }
    };
    try {
      return await Promise.race([
        work(),
        new Promise((_, reject) => {
          abort = () => { ended = true; reject(new Error('cancelled')); };
          signal?.addEventListener('abort', abort, { once: true });
          timer = root.setTimeout(() => { ended = true; reject(new Error('image_prepare_timeout')); }, 10000);
        }),
      ]);
    } finally {
      ended = true;
      root.clearTimeout(timer);
      if (abort) signal?.removeEventListener('abort', abort);
      bitmap?.close();
      bitmap = null;
      if (canvas) { canvas.width = 1; canvas.height = 1; canvas = null; }
    }
  }

  return Object.freeze({ version: 1, available, prepare });
});
