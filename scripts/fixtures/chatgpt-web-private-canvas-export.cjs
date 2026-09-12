'use strict';
const { fixture, PATH, ID } = require('./chatgpt-web-private-canvas-documents.cjs');
const base = '../../android/app/src/main/assets/';
const exporter = require(base + 'chatgpt_web_private_canvas_export.js');
const downloads = require(base + 'chatgpt_web_private_file_download.js');
const binary = require(base + 'chatgpt_web_private_library_download.js');
const text = require(base + 'chatgpt_web_private_canvas_text_export.js');
const policy = require(base + 'chatgpt_web_private_canvas_document_policy.js');
const MIME = { pdf: 'application/pdf', docx: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' };
const URL = 'https://chatgpt.com/backend-api/export_doc/canvas';

function setup(options = {}) {
  const f = fixture(), packets = [], requests = [], stored = [], receipts = [];
  const format = options.format || 'pdf';
  const data = options.bytes || Buffer.from(format === 'pdf' ? '%PDF-1.7\nSynthetic\n%%EOF' : 'PK\x03\x04SyntheticDocx');
  let saved = false, cancelled = false;
  const bridge = { onmessage: null, postMessage(raw) {
    const p = JSON.parse(raw); packets.push(p);
    if (p.cancel) { cancelled = true; if (!saved) stored.length = 0; return; }
    if (p.byteOperation === 'chunk') stored.push(Buffer.from(p.data, 'base64'));
    if (p.byteOperation === 'commit') saved = true;
    options.packet?.(p, f);
    if (options.dropCommit && p.byteOperation === 'commit') return;
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: p.leaseId, sequence: p.sequence,
      byteOperation: p.byteOperation, state: { begin: 'ready', chunk: 'written', commit: 'saved' }[p.byteOperation] }) }));
  } };
  Object.assign(f.page, { __elonChatGptPrivateCanvasExport: exporter, __elonChatGptPrivateLibraryDownload: binary,
    __elonChatGptPrivateCanvasDocumentPolicy: policy,
    __elonChatGptPrivateCanvasTextExport: options.text || text,
    __elonChatGptPrivateCanvasDocuments: { create: () => f.api }, elonChatGptFileDownload: bridge, AbortController, btoa, Blob, Response,
    setTimeout: (fn, ms) => setTimeout(fn, options.dropCommit && ms === 8000 || options.fastPrepare && ms === 20000 ? 15 : ms),
    fetch: async (url, init) => {
      requests.push({ url, init });
      await options.fetch?.(f);
      if (options.lostResponse) throw Error('network_failure');
      const response = new Response(new ReadableStream({ start(controller) {
        if (options.fragmented) for (const b of data) controller.enqueue(Uint8Array.of(b));
        else controller.enqueue(data);
        controller.close();
      }, cancel() { f.bodyCancelled = true; } }), { status: options.status || 200,
        headers: { 'content-type': options.mime || MIME[format], 'content-length': String(options.length ?? data.length) } });
      Object.defineProperty(response, 'url', { value: options.url || url });
      return response;
    } });
  const download = downloads.create(f.page);
  f.page.__elonChatGptPrivateFileDownload = download;
  async function prepare() {
    const list = await f.list();
    return f.run({ operation: 'prepare_export', id: ID, ticket: list.ticket, scope: list.scope, format });
  }
  const run = value => download.start(JSON.stringify({ version: 1, byteTransferVersion: 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: f.page.__elonChatGptDocumentToken,
    href: f.page.location.href, path: PATH, name: value.file.name, downloadHandle: value.file.downloadHandle }),
    (...args) => receipts.push(args));
  return Object.assign(f, { prepare, runDownload: run, download, packets, exports: requests, stored, receipts, data,
    saved: () => saved, cancelled: () => cancelled });
}
module.exports = { setup, PATH, ID, URL };
