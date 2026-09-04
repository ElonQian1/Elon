'use strict';

const assert = require('assert');
const transportModule = require('../android/app/src/main/assets/chatgpt_web_private_dictation_transport.js');
const orchestratorModule = require('../android/app/src/main/assets/chatgpt_web_private_dictation_orchestrator.js');

class FakeStorage {
  constructor() { this.values = new Map(); }
  getItem(key) { return this.values.get(key) || null; }
  setItem(key, value) { this.values.set(key, value); }
}

class FakeTrack {
  constructor() { this.readyState = 'live'; }
  stop() { this.readyState = 'ended'; }
}

class FakeStream {
  constructor() { this.track = new FakeTrack(); }
  getAudioTracks() { return [this.track]; }
  getTracks() { return [this.track]; }
}

class FakeBlob {
  constructor(parts, options) {
    this.size = parts.reduce((total, part) => total + Number(part && part.size || 0), 0);
    this.type = options && options.type || '';
  }
}

class FakeFormData {
  constructor() { this.entries = []; }
  append(name, value, filename) { this.entries.push({ name, value, filename }); }
}

class FakeMediaRecorder {
  static isTypeSupported() { return true; }
  constructor(stream, options) {
    this.stream = stream;
    this.mimeType = options && options.mimeType || 'audio/webm';
    this.state = 'inactive';
    this.listeners = new Map();
  }
  addEventListener(name, listener) {
    const values = this.listeners.get(name) || [];
    values.push(listener);
    this.listeners.set(name, values);
  }
  emit(name, event) {
    (this.listeners.get(name) || []).forEach((listener) => listener(event || {}));
  }
  start() {
    this.state = 'recording';
    this.emit('start');
  }
  stop() {
    if (this.state === 'inactive') return;
    this.emit('dataavailable', { data: { size: 32 } });
    this.state = 'inactive';
    this.emit('stop');
  }
}

function response(status, payload) {
  return {
    status,
    ok: status >= 200 && status < 300,
    json: async () => payload
  };
}

function createRoot(options) {
  const config = options || {};
  const requests = [];
  let microphoneRequests = 0;
  let authRequests = 0;
  const root = {
    __elonChatGptPrivateDictationEnabled: true,
    __elonChatGptPrivateAuthContext: {
      acquireRequestHeaders: async () => {
        authRequests += 1;
        if (config.authFailure) throw new Error('auth_http_401');
        return { Authorization: 'Bearer page-local-token' };
      },
      invalidate: () => {}
    },
    location: { origin: 'https://chatgpt.com' },
    navigator: {
      language: 'zh-CN',
      mediaDevices: {
        getUserMedia: async () => {
          microphoneRequests += 1;
          return new FakeStream();
        }
      }
    },
    document: { documentElement: { lang: 'zh-CN' } },
    sessionStorage: new FakeStorage(),
    MediaRecorder: FakeMediaRecorder,
    FormData: FakeFormData,
    Blob: FakeBlob,
    AbortController,
    crypto: { randomUUID: () => '11111111-1111-4111-8111-111111111111' },
    setTimeout,
    clearTimeout,
    fetch: async (url, init) => {
      requests.push({ url, init });
      if (url === '/backend-api/transcribe') return response(200, { text: 'hello' });
      return response(404, {});
    }
  };
  return {
    root,
    requests,
    authRequests: () => authRequests,
    microphoneRequests: () => microphoneRequests
  };
}

async function privateTransportCompletesBufferedTranscription() {
  const fixture = createRoot();
  const transport = transportModule.create(fixture.root);

  assert.strictEqual(transport.ready(), true);
  const started = await transport.start();
  assert.deepStrictEqual(started, { ok: true, code: 'capture_started', captured: true });
  assert.strictEqual(fixture.microphoneRequests(), 1);
  assert.strictEqual(fixture.authRequests(), 1);
  assert.strictEqual(transport.snapshot().phase, 'capturing');

  const submitted = await transport.submit();
  assert.strictEqual(submitted.ok, true);
  assert.strictEqual(submitted.transcript, 'hello');
  assert.strictEqual(submitted.transcriptLength, 5);
  assert.strictEqual(transport.snapshot().phase, 'idle');
  const request = fixture.requests.find((entry) => entry.url === '/backend-api/transcribe');
  assert(request);
  assert.strictEqual(request.init.method, 'POST');
  assert.strictEqual(request.init.headers.Authorization.startsWith('Bearer '), true);
  assert.strictEqual(fixture.requests.some((entry) => entry.url === '/api/auth/session'), false);
  assert.deepStrictEqual(
    request.init.body.entries.map((entry) => entry.name),
    ['file', 'dictation_session_id', 'attempt_id', 'language', 'duration_ms']
  );
}

async function authenticationFailureNeverStartsMicrophone() {
  const fixture = createRoot({ authFailure: true });
  const transport = transportModule.create(fixture.root);
  const started = await transport.start();

  assert.strictEqual(started.ok, false);
  assert.strictEqual(started.captured, false);
  assert.strictEqual(started.code.startsWith('before_capture:'), true);
  assert.strictEqual(fixture.microphoneRequests(), 0);
  assert.strictEqual(fixture.authRequests(), 1);
  assert.strictEqual(transport.snapshot().phase, 'idle');
}

async function orchestratorCommitsTranscriptBeforeReportingSuccess() {
  let draft = 'hello';
  const results = [];
  const orchestrator = orchestratorModule.create({
    transport: {
      ready: () => true,
      start: async () => ({ ok: true, code: 'capture_started', captured: true }),
      submit: async () => ({ ok: true, code: 'transcript_ready', transcript: 'world', transcriptLength: 5 }),
      cancel: async () => ({ ok: true, code: 'capture_cancelled', captured: true })
    },
    findComposer: () => ({}),
    composerValue: () => draft,
    setComposerValue: (_composer, value) => { draft = value; return true; },
    comparableText: (value) => String(value || '').trim(),
    scheduleSnapshot: () => {}
  });
  const respond = (action, ok, detail) => results.push({ action, ok, detail, draft });

  await orchestrator.start('hello', 'hello', respond);
  await orchestrator.submit(respond);

  assert.strictEqual(draft, 'hello world');
  assert.deepStrictEqual(results[1], {
    action: 'private_submit_dictation',
    ok: true,
    detail: 'transcript_ready:5',
    draft: 'hello world'
  });
}

(async () => {
  await privateTransportCompletesBufferedTranscription();
  await authenticationFailureNeverStartsMicrophone();
  await orchestratorCommitsTranscriptBeforeReportingSuccess();
  console.log('chatgpt private dictation transport tests passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
