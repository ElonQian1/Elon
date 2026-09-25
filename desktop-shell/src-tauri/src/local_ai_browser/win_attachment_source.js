(function (root, factory) {
  'use strict';
  if (typeof module === 'object' && module.exports) module.exports = factory;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonWinAttachmentSource) root.__elonWinAttachmentSource = factory(root);
})(typeof window === 'object' ? window : null, function (root) {
  'use strict';
  let batch = null;
  const validId = value => /^[a-f0-9-]{36}$/.test(value || '');
  const current = job => batch === job && root.location.href === job.href && root.__elonChatGptDocumentToken === job.token;
  function clear() {
    if (!batch) return;
    root.clearTimeout(batch.timer);
    batch.files.forEach(file => { file.bytes = null; });
    batch = null;
  }
  function cancel() { root.__elonChatGptPrivateAttachmentSend?.cancel(); clear(); }
  const bridge = { onmessage: null, postMessage(raw) {
    const request = JSON.parse(raw), job = batch;
    const file = job?.files.find(file => file.leaseId === request.leaseId);
    if (!job || !current(job) || request.documentToken !== job.token || !file?.bytes || request.offset !== file.read) {
      bridge.onmessage?.({ data: JSON.stringify({ requestId: request.requestId, code: 'expired' }) }); return;
    }
    const bytes = file.bytes.subarray(file.read, Math.min(file.read + 65536, file.size));
    let text = ''; for (let i = 0; i < bytes.length; i++) text += String.fromCharCode(bytes[i]);
    file.read += bytes.length;
    if (file.read === file.size) file.bytes = null;
    bridge.onmessage?.({ data: JSON.stringify({ requestId: request.requestId, offset: request.offset, data: root.btoa(text) }) });
  } };
  root.elonChatGptAttachmentSource = bridge;
  function receipt(action, requestId, ok, detail) {
    root.elonChatGptNative.postMessage(JSON.stringify({ type: 'command_result', action, requestId, ok, detail,
      adapterVersion: root.__elonChatGptAdapterVersion, documentToken: root.__elonChatGptDocumentToken }));
  }
  async function execute(value, requestId) {
    if (value.step === 'cancel') { cancel(); return; }
    if (value.step === 'begin') {
      if (batch || !validId(value.batchId) || !Array.isArray(value.files) || value.files.length < 1 || value.files.length > 9
        || !root.__elonChatGptDocumentToken) throw new Error('invalid_batch');
      const files = value.files.map(file => {
        if (!validId(file.leaseId) || !Number.isSafeInteger(file.size) || file.size < 1 || file.size > 8388608
          || typeof file.name !== 'string' || !file.name.trim() || file.name.length > 120 || /[\x00-\x1f\x7f/\\]/.test(file.name)
          || !['image/png', 'image/jpeg', 'image/webp'].includes(file.type) && !root.__elonChatGptPrivateAttachmentProtocol?.isDocument(file)
          || file.sha256 && !/^[a-f0-9]{64}$/i.test(file.sha256)) throw new Error('invalid_file');
        return { leaseId: file.leaseId, name: file.name, type: file.type, size: file.size, sha256: file.sha256,
          bytes: new Uint8Array(file.size), offset: 0, read: 0 };
      });
      if (new Set(files.map(f => f.leaseId)).size !== files.length) throw new Error('duplicate_file');
      batch = { id: value.batchId, files, href: root.location.href, token: root.__elonChatGptDocumentToken, uploading: false };
      batch.timer = root.setTimeout(cancel, 180000);
      return;
    }
    const job = batch;
    if (!job || !current(job) || value.batchId !== job.id || job.uploading) throw new Error('expired_batch');
    if (value.step === 'chunk') {
      const file = job.files.find(f => f.leaseId === value.leaseId);
      if (!file || value.offset !== file.offset || typeof value.data !== 'string' || value.data.length > 88 * 1024) throw new Error('invalid_chunk');
      const binary = root.atob(value.data);
      if (!binary.length || binary.length > 65536 || file.offset + binary.length > file.size) throw new Error('invalid_chunk');
      for (let i = 0; i < binary.length; i++) file.bytes[file.offset + i] = binary.charCodeAt(i);
      file.offset += binary.length;
      return;
    }
    if (value.step !== 'upload' || job.files.some(file => file.offset !== file.size)) throw new Error('incomplete_files');
    job.uploading = true;
    const descriptors = [];
    for (const file of job.files) {
      if (file.sha256) {
        const digest = new Uint8Array(await root.crypto.subtle.digest('SHA-256', file.bytes));
        const hash = Array.from(digest, b => b.toString(16).padStart(2, '0')).join('');
        if (hash !== file.sha256.toLowerCase()) throw new Error('file_changed');
      }
      let width, height;
      if (file.type.startsWith('image/')) {
        const bitmap = await root.createImageBitmap(new root.Blob([file.bytes], { type: file.type }));
        width = bitmap.width; height = bitmap.height; bitmap.close();
      }
      if (!current(job)) throw new Error('expired_batch');
      descriptors.push({ version: 1, leaseId: file.leaseId, name: file.name, type: file.type, size: file.size,
        width, height, href: job.href, documentToken: job.token, uploadCopy: true });
    }
    if (!root.__elonChatGptPrivateAttachmentSend) throw new Error('private_upload_unavailable');
    await root.__elonChatGptPrivateAttachmentSend.start(JSON.stringify({ version: 2, files: descriptors,
      href: job.href, documentToken: job.token }),
    (action, ok, detail) => receipt(action, requestId, ok, detail),
    () => root.__elonChatGptBridge?.command(JSON.stringify({ action: 'snapshot', documentToken: job.token })),
    () => receipt('request_attachment_upload', requestId, false, 'private_upload_unavailable'));
    clear();
  }
  async function command(raw) {
    let command;
    try {
      command = JSON.parse(raw);
      const value = JSON.parse(command.value);
      await execute(value, command.requestId);
      if (value.step !== 'upload') receipt('stage_attachments', command.requestId, true, 'staged');
    } catch (_) {
      const action = (() => { try { return JSON.parse(command?.value).step === 'upload' ? 'request_attachment_upload' : 'stage_attachments'; } catch (_) { return 'stage_attachments'; } })();
      cancel();
      receipt(action, command?.requestId, false, '附件读取或上传失败；未发送文字，请重新选择。');
    }
  }
  root.addEventListener?.('pagehide', cancel);
  function guardSend(raw) {
    const value = JSON.parse(raw);
    if (!batch || value.action !== 'send_prompt') return false;
    receipt('send_prompt', value.requestId, false, '附件仍在上传，文字未发送。请等待附件完成后再发送。');
    return true;
  }
  return Object.freeze({ command, cancel, guardSend });
});
