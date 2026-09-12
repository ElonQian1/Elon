(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptPrivateCanvasExport = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const TYPES = Object.freeze({ pdf: 'application/pdf',
    docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' });
  const ID = /^[A-Za-z0-9_-]{1,128}$/;
  const URL = 'https://chatgpt.com/backend-api/export_doc/canvas';
  const fail = code => { throw Error('download_' + code); };

  function validator(format) {
    const prefix = [], signature = format === 'pdf' ? [37, 80, 68, 70, 45] : [80, 75, 3, 4];
    let total = 0;
    return (bytes, done = false) => {
      total += bytes.length;
      if (total > 64 * 1024 * 1024) fail('file_too_large');
      for (const byte of bytes) {
        if (prefix.length === signature.length) break;
        prefix.push(byte);
        if (byte !== signature[prefix.length - 1]) fail('content_invalid');
      }
      if (done && prefix.length !== signature.length) fail('content_invalid');
    };
  }

  function register(page, binding, document, format, verify) {
    if (!Object.hasOwn(TYPES, format) || document.documentType !== 'document' || !ID.test(document.id) ||
        typeof verify !== 'function' || !page.__elonChatGptPrivateFileDownload?.registerCanvasExport) {
      throw Error('canvas_export_unsupported');
    }
    let stem = document.title.replace(/[\x00-\x1f\x7f-\x9f\u202a-\u202e\u2066-\u2069\\/:*?"<>|]/g, '_').trim().slice(0, 140);
    if (/[\ud800-\udbff]$/.test(stem)) stem = stem.slice(0, -1);
    const name = (stem || 'Canvas') + '.' + format;
    const file = { id: 'canvas-export-' + document.id + '-' + format, name, mediaType: TYPES[format] };
    const prepare = (job, current) => verify(async check => {
      check();
      job.entry.canvasCurrent = () => { try { check(); return true; } catch (_) { return false; } };
      if (!current(job)) fail('cancelled');
      const headers = { Accept: TYPES[format], 'Content-Type': 'application/json' };
      for (const [key, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders())) {
        if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
          'oai-client-build-number'].includes(key.toLowerCase())) headers[key] = String(value);
      }
      // Xe's official document export: POST returns bytes, not a file URL or body-save result.
      const response = await page.fetch(URL, { method: 'POST', headers, credentials: 'include', cache: 'no-store',
        redirect: 'error', signal: job.controller.signal, body: JSON.stringify({ textdoc_id: document.id, export_type: format }),
        __elonPrivateTransport: 'canvas_export_v1' });
      try {
        check();
        if (!current(job)) fail('cancelled');
        if (response.status === 401 || response.status === 403) {
          page.__elonChatGptPrivateAuthContext?.invalidate?.('canvas_export_rejected');
          fail('authorization_failed');
        }
        if (response.status === 404) fail('file_unavailable');
        if (!response.ok || response.status !== 200 || response.url !== URL || response.redirected || !response.body?.getReader) {
          fail('prepare_failed');
        }
        const mime = String(response.headers.get('content-type') || '').split(';')[0].trim().toLowerCase();
        if (![TYPES[format], 'application/octet-stream'].includes(mime)) fail('content_invalid');
        return { response, validateBytes: validator(format) };
      } catch (error) {
        try { await response.body?.cancel(); } catch (_) {}
        throw error;
      }
    });
    const handle = page.__elonChatGptPrivateFileDownload.registerCanvasExport(binding.path, file, prepare);
    if (!handle) throw Error('canvas_export_unavailable');
    return { documentId: document.id, documentVersion: document.documentVersion, format,
      file: { ...file, downloadHandle: handle } };
  }
  return Object.freeze({ version: 1, register });
});
