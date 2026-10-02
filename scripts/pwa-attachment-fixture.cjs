// Production PWA assets with loopback-only, in-memory attachment/message endpoints.
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');
const { execFileSync } = require('node:child_process');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');
const mimeSource = fs.readFileSync(path.join(__dirname, '../server/src/project_attachment_paths.rs'), 'utf8').split('pub(crate) fn percent_encode_path_segment')[0];
const mimeTypes = new Map();
for (const match of mimeSource.matchAll(/((?:"[a-z0-9]+"\s*\|?\s*)+)\s*=>\s*"([^"]+)"/g)) {
  for (const extension of match[1].matchAll(/"([a-z0-9]+)"/g)) mimeTypes.set(extension[1], match[2]);
}

async function attachmentFixture(context) {
  const state = { uploads: [], messages: [], attempts: 0, failUpload: 0, failSend: false, uploadGate: null, external: [] };
  const fixture = createFixture({ handleSyntheticRequest: async (req, res, url, authenticated) => {
    const p = url.pathname, json = (body, status = 200) => { res.writeHead(status, { 'content-type': 'application/json' }); res.end(JSON.stringify(body)); };
    if (p === '/api/user/mobile-v2-fixture/chat-attachments' && req.method === 'POST') {
      if (!authenticated) { json({}, 401); return true; }
      state.attempts++;
      const chunks = []; let size = 0;
      for await (const chunk of req) { size += chunk.length; if (size > 12 * 1024 * 1024) { json({}, 413); return true; } chunks.push(chunk); }
      if (state.uploadGate) await state.uploadGate;
      if (state.failUpload === state.attempts) { json({ error: 'synthetic upload failure' }, 503); return true; }
      const buffer = Buffer.concat(chunks), mime = url.searchParams.get('mime_type');
      const attachment = { attachment_id: 'fixture-' + state.uploads.length, kind: url.searchParams.get('kind'),
        mime_type: mime, display_name: url.searchParams.get('display_name'), file_name: url.searchParams.get('file_name'),
        size_bytes: buffer.length, sha256: crypto.createHash('sha256').update(buffer).digest('hex'),
        url: url.origin + '/api/user/mobile-v2-fixture/chat-attachments/download/' + state.uploads.length };
      state.uploads.push({ attachment, buffer, headers: req.headers, conversation: url.searchParams.get('conversation_id') });
      json({ attachment }); return true;
    }
    if (p.startsWith('/api/user/mobile-v2-fixture/chat-attachments/download/')) {
      const upload = state.uploads[Number(p.split('/').at(-1))];
      if (!upload) { json({}, 404); return true; }
      // Consume the production mapping, including the fallback that failed WebKit playback.
      res.writeHead(200, { 'content-type': mimeTypes.get(upload.attachment.file_name.split('.').at(-1).toLowerCase())
        || 'application/octet-stream', 'content-length': upload.buffer.length });
      res.end(upload.buffer); return true;
    }
    return false;
  } }), origin = await fixture.listen();
  await context.route('**/*', async route => {
    const request = route.request(), url = new URL(request.url()), p = url.pathname;
    if (url.origin !== origin) { state.external.push(url.origin); return route.abort(); }
    if (p === '/assets/vendor/jsqr.js') return route.fulfill({ contentType: 'text/javascript',
      body: fs.readFileSync(path.join(__dirname, '../server/src/assets/vendor/jsqr.js')) });
    const json = (body, status = 200) => route.fulfill({ json: body, status });
    if (process.env.PWA_ATTACHMENTS_BASELINE && p === '/assets/social_source_compose.js') {
      return route.fulfill({ contentType: 'text/javascript', body: execFileSync('git', ['show', 'HEAD:server/src/assets/social_source_compose.js']) });
    }
    if (p === '/api/me/groups') return json({ groups: [{ id: 'attach-group', name: '附件测试群', member_count: 2 }] });
    if (p === '/api/me/friends') return json({ friends: [{ id: 'attach-friend', nickname: '附件测试好友' }] });
    if (p === '/api/me/message-timeline/read') return json({});
    if (p === '/api/me/message-timeline' || p === '/api/me/message-timeline/window') {
      const query = request.method() === 'POST' ? request.postDataJSON() : Object.fromEntries(url.searchParams);
      const source = `/api/me/${query.kind}s/${query.id}/messages`;
      const messages = state.messages.filter(item => item.path === source).map(item => ({ ...item.message, timeline_cursor: item.message.id }));
      return json({ schema: 'elon.message_timeline.v1', messages, removed_ids: [], has_more: false, sync: 'fixture-live' });
    }
    if (/^\/api\/me\/(groups|friends)\/attach-/.test(p)) {
      if (request.method() === 'POST' && p.endsWith('/messages')) {
        if (state.failSend) return json({ error: 'synthetic uncertain send' }, 503);
        const message = { ...request.postDataJSON(), id: 'message-' + state.messages.length,
          outgoing: true, sender_user_id: 'mobile-v2-fixture', created_at: new Date().toISOString() };
        if (message.quote_source) {
          const source = state.messages.find(item => item.path === p && item.message.id === message.quote_source.message_id)?.message;
          if (!source || source.recalled_at || (source.revision || 1) !== message.quote_source.revision) return json({ error: '原消息已修改或不可用' }, 400);
          message.quote = { message_id: source.id, sender_name: source.sender_name || '演示成员', content: source.content,
            attachments: source.attachments || [], revision: source.revision || 1, unavailable: false };
        }
        state.messages.push({ path: p, message }); return json({ message });
      }
      return json({ messages: state.messages.filter(item => item.path === p).map(item => item.message),
        posts: [], items: [], members: [], ai_members: [] });
    }
    return route.continue();
  });
  return { ...state, state, origin, close: () => fixture.close() };
}

function audioFile() {
  const samples = 16000, buffer = Buffer.alloc(44 + samples * 2);
  buffer.write('RIFF'); buffer.writeUInt32LE(buffer.length - 8, 4); buffer.write('WAVEfmt ', 8);
  buffer.writeUInt32LE(16, 16); buffer.writeUInt16LE(1, 20); buffer.writeUInt16LE(1, 22);
  buffer.writeUInt32LE(16000, 24); buffer.writeUInt32LE(32000, 28); buffer.writeUInt16LE(2, 32);
  buffer.writeUInt16LE(16, 34); buffer.write('data', 36); buffer.writeUInt32LE(samples * 2, 40);
  for (let i = 0; i < samples; i++) buffer.writeInt16LE(Math.round(Math.sin(i * Math.PI * 2 * 440 / 16000) * 2000), 44 + i * 2);
  return { name: '测试音频.wav', mimeType: 'audio/wav', buffer };
}

async function installRecorderDouble(page) {
  await page.evaluate(() => {
    window.voiceTest = { mode: 'ok', stops: 0, requests: 0, recorders: [], supported: 'audio/mp4' };
    const stream = () => ({ getTracks: () => [{ stop: () => { voiceTest.stops++; } }] });
    if (!navigator.mediaDevices) Object.defineProperty(navigator, 'mediaDevices', { configurable: true, value: {} });
    Object.defineProperty(navigator.mediaDevices, 'getUserMedia', { configurable: true, value: async () => {
      voiceTest.requests++;
      if (voiceTest.mode === 'deny') throw new DOMException('denied', 'NotAllowedError');
      if (voiceTest.mode === 'pending') return new Promise((resolve, reject) => {
        voiceTest.resolve = () => resolve(stream()); voiceTest.reject = () => reject(new DOMException('late denial', 'NotAllowedError'));
      });
      return stream();
    } });
    window.MediaRecorder = class {
      static isTypeSupported(type) { return type === voiceTest.supported; }
      constructor(_, options) { this.mimeType = options?.mimeType || 'audio/webm'; this.state = 'inactive'; voiceTest.recorders.push(this); }
      start() { this.state = 'recording'; }
      stop() { this.state = 'inactive'; queueMicrotask(() => {
        this.ondataavailable?.({ data: new Blob(voiceTest.mode === 'empty' ? [] : ['synthetic-recording'], { type: this.mimeType }) });
        this.onstop?.();
      }); }
    };
  });
}
module.exports = { attachmentFixture, audioFile, installRecorderDouble };
