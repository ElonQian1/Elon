(function (root, factory) {
  'use strict';

  const exported = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = exported;
  if (root && root.location && root.location.origin === 'https://chatgpt.com') {
    const current = root.__elonChatGptPrivateDictationTransport;
    if (!current || Number(current.version) < exported.version) {
      root.__elonChatGptPrivateDictationTransport = Object.freeze(factory(root));
    }
  }
})(typeof window === 'object' ? window : globalThis, function (root) {
  'use strict';

  const VERSION = 1;
  const AUTH_TIMEOUT_MS = 5000;
  const TRANSCRIBE_TIMEOUT_MS = 30000;
  const MAX_CAPTURE_MS = 120000;
  const STORAGE_KEY = 'elon.chatgpt.private.dictation.health.v1';
  const MIME_TYPES = ['audio/webm;codecs=opus', 'audio/webm'];
  const BLOCKED_INHERITED_HEADERS = new Set([
    'accept-encoding', 'connection', 'content-length', 'content-type', 'cookie',
    'host', 'if-modified-since', 'if-none-match', 'origin', 'referer', 'sec-fetch-dest',
    'sec-fetch-mode', 'sec-fetch-site', 'transfer-encoding', 'user-agent'
  ]);
  const enabled = root && root.__elonChatGptPrivateDictationEnabled === true;
  let phase = 'idle';
  let generation = 0;
  let recorder = null;
  let stream = null;
  let chunks = [];
  let startedAt = 0;
  let captureTimer = null;
  let requestHeaders = null;
  let health = readHealth();

  function now() {
    return Date.now();
  }

  function storage() {
    try { return root.sessionStorage || null; } catch (_) { return null; }
  }

  function readHealth() {
    const fallback = { failures: 0, cooldownUntil: 0, lastOutcome: 'none' };
    const store = storage();
    if (!store || typeof store.getItem !== 'function') return fallback;
    try {
      const value = JSON.parse(store.getItem(STORAGE_KEY) || 'null');
      if (!value || value.version !== VERSION) return fallback;
      return {
        failures: Math.max(0, Math.min(10, Number(value.failures) || 0)),
        cooldownUntil: Math.max(0, Number(value.cooldownUntil) || 0),
        lastOutcome: String(value.lastOutcome || 'none').slice(0, 32)
      };
    } catch (_) {
      return fallback;
    }
  }

  function persistHealth() {
    const store = storage();
    if (!store || typeof store.setItem !== 'function') return;
    try {
      store.setItem(STORAGE_KEY, JSON.stringify({ version: VERSION, ...health }));
    } catch (_) {}
  }

  function recordSuccess() {
    health = { failures: 0, cooldownUntil: 0, lastOutcome: 'success' };
    persistHealth();
  }

  function recordFailure(outcome, longCooldown) {
    const failures = Math.min(10, health.failures + 1);
    const cooldownMs = longCooldown ? 5 * 60 * 1000 : failures >= 2 ? 60 * 1000 : 10 * 1000;
    health = {
      failures,
      cooldownUntil: now() + cooldownMs,
      lastOutcome: String(outcome || 'network').slice(0, 32)
    };
    persistHealth();
  }

  function supported() {
    return !!(enabled && root && root.navigator && root.navigator.mediaDevices &&
      typeof root.navigator.mediaDevices.getUserMedia === 'function' &&
      typeof root.MediaRecorder === 'function' && typeof root.FormData === 'function' &&
      typeof root.Blob === 'function' && typeof root.fetch === 'function');
  }

  function ready() {
    return supported() && phase === 'idle' && health.cooldownUntil <= now();
  }

  function sanitizedHeaders(source) {
    const headers = {};
    if (!source || typeof source !== 'object') return headers;
    Object.keys(source).forEach((name) => {
      const lower = String(name).toLowerCase();
      if (!lower || BLOCKED_INHERITED_HEADERS.has(lower)) return;
      headers[name] = String(source[name]);
    });
    return headers;
  }

  function inheritedHeaders() {
    const transport = root.__elonChatGptPrivateTransport;
    if (!transport || typeof transport.copySameOriginRequestHeaders !== 'function') return {};
    try { return sanitizedHeaders(transport.copySameOriginRequestHeaders()); } catch (_) { return {}; }
  }

  function authorizationHeader(headers) {
    return Object.keys(headers).find((name) => String(name).toLowerCase() === 'authorization');
  }

  async function fetchWithTimeout(url, options, timeoutMs) {
    const controller = typeof root.AbortController === 'function' ? new root.AbortController() : null;
    const timer = controller ? root.setTimeout(() => controller.abort(), timeoutMs) : null;
    try {
      return await root.fetch(url, Object.assign({}, options, {
        signal: controller ? controller.signal : undefined
      }));
    } finally {
      if (timer !== null) root.clearTimeout(timer);
    }
  }

  async function acquireRequestHeaders() {
    const inherited = inheritedHeaders();
    if (authorizationHeader(inherited)) return Object.assign({ Accept: 'application/json' }, inherited);
    const response = await fetchWithTimeout('/api/auth/session', {
      method: 'GET', credentials: 'include', cache: 'no-store', headers: { Accept: 'application/json' }
    }, AUTH_TIMEOUT_MS);
    if (!response || !response.ok) throw new Error('auth_http_' + Number(response && response.status));
    const payload = await response.json();
    const accessToken = payload && typeof payload.accessToken === 'string' ? payload.accessToken : '';
    if (!accessToken) throw new Error('auth_missing');
    return Object.assign({ Accept: 'application/json', Authorization: 'Bearer ' + accessToken }, inherited);
  }

  function selectedMimeType() {
    const supports = root.MediaRecorder && typeof root.MediaRecorder.isTypeSupported === 'function'
      ? root.MediaRecorder.isTypeSupported.bind(root.MediaRecorder)
      : () => true;
    return MIME_TYPES.find(supports) || '';
  }

  function createRecorder(mediaStream) {
    const mimeType = selectedMimeType();
    return mimeType ? new root.MediaRecorder(mediaStream, { mimeType }) : new root.MediaRecorder(mediaStream);
  }

  function uuid() {
    if (root.crypto && typeof root.crypto.randomUUID === 'function') return root.crypto.randomUUID();
    return '00000000-0000-4000-8000-' + Math.random().toString(16).slice(2).padEnd(12, '0').slice(0, 12);
  }

  function language() {
    const value = String(root.document && root.document.documentElement &&
      root.document.documentElement.lang || root.navigator.language || 'en');
    return value.toLowerCase().split(/[-_]/)[0].slice(0, 8) || 'en';
  }

  function stopTracks() {
    const tracks = stream && typeof stream.getTracks === 'function' ? stream.getTracks() : [];
    tracks.forEach((track) => {
      if (track && typeof track.stop === 'function') {
        try { track.stop(); } catch (_) {}
      }
    });
    stream = null;
  }

  function clearCaptureTimer() {
    if (captureTimer !== null) root.clearTimeout(captureTimer);
    captureTimer = null;
  }

  function reset() {
    clearCaptureTimer();
    stopTracks();
    recorder = null;
    chunks = [];
    startedAt = 0;
    requestHeaders = null;
    phase = 'idle';
  }

  function outcome(ok, code, captured, extra) {
    return Object.assign({ ok: ok === true, code: String(code || ''), captured: captured === true }, extra || {});
  }

  function waitForRecorderStart(target, token) {
    if (target.state === 'recording') return Promise.resolve(true);
    return new Promise((resolve) => {
      const timer = root.setTimeout(() => resolve(false), 1500);
      target.addEventListener('start', () => {
        root.clearTimeout(timer);
        resolve(token === generation && target.state === 'recording');
      }, { once: true });
    });
  }

  async function start() {
    if (!ready()) return outcome(false, 'before_capture:unavailable', false);
    generation += 1;
    const token = generation;
    phase = 'starting';
    try {
      requestHeaders = await acquireRequestHeaders();
      if (token !== generation || phase !== 'starting') return outcome(false, 'before_capture:cancelled', false);
      stream = await root.navigator.mediaDevices.getUserMedia({
        audio: { channelCount: 1, echoCancellation: true, noiseSuppression: true, autoGainControl: true }
      });
      const liveTracks = stream && typeof stream.getAudioTracks === 'function'
        ? stream.getAudioTracks().filter((track) => track && track.readyState !== 'ended')
        : [];
      if (!liveTracks.length) throw new Error('capture_missing');
      recorder = createRecorder(stream);
      chunks = [];
      recorder.addEventListener('dataavailable', (event) => {
        if (token === generation && event.data && event.data.size > 0) chunks.push(event.data);
      });
      recorder.start(250);
      if (!await waitForRecorderStart(recorder, token)) throw new Error('capture_start_timeout');
      startedAt = now();
      phase = 'capturing';
      captureTimer = root.setTimeout(() => {
        if (token !== generation || phase !== 'capturing' || !recorder || recorder.state === 'inactive') return;
        try { recorder.stop(); } catch (_) {}
        phase = 'captured';
        stopTracks();
      }, MAX_CAPTURE_MS);
      return outcome(true, 'capture_started', true);
    } catch (error) {
      const captured = !!(recorder && recorder.state === 'recording');
      if (!captured) {
        recordFailure(String(error && error.message || 'start'), /auth/.test(String(error && error.message)));
      }
      reset();
      return outcome(false, captured ? 'capture:start_failed' : 'before_capture:start_failed', captured);
    }
  }

  function stopRecorder(target) {
    if (!target || target.state === 'inactive') return Promise.resolve();
    return new Promise((resolve) => {
      let settled = false;
      const finish = () => {
        if (settled) return;
        settled = true;
        resolve();
      };
      target.addEventListener('stop', finish, { once: true });
      root.setTimeout(finish, 3000);
      try { target.stop(); } catch (_) { finish(); }
    });
  }

  async function submit() {
    if (phase !== 'capturing' && phase !== 'captured') {
      return outcome(false, 'capture:not_active', false);
    }
    const token = generation;
    const target = recorder;
    phase = 'submitting';
    clearCaptureTimer();
    await stopRecorder(target);
    stopTracks();
    if (token !== generation || phase !== 'submitting') return outcome(false, 'capture:cancelled', true);
    const mimeType = String(target && target.mimeType || selectedMimeType() || 'audio/webm');
    const blob = new root.Blob(chunks, { type: mimeType });
    const durationMs = Math.max(1, now() - startedAt);
    if (!blob.size) {
      recordFailure('empty_audio', false);
      reset();
      return outcome(false, 'capture:empty_audio', true);
    }
    try {
      const form = new root.FormData();
      form.append('file', blob, 'dictation.webm');
      form.append('dictation_session_id', uuid());
      form.append('attempt_id', uuid());
      form.append('language', language());
      form.append('duration_ms', String(durationMs));
      const response = await fetchWithTimeout('/backend-api/transcribe', {
        method: 'POST', credentials: 'include', cache: 'no-store', headers: requestHeaders, body: form
      }, TRANSCRIBE_TIMEOUT_MS);
      if (!response || !response.ok) throw new Error('http_' + Number(response && response.status));
      const payload = await response.json();
      const transcript = payload && typeof payload.text === 'string' ? payload.text.trim() : '';
      if (!transcript) throw new Error('empty_transcript');
      recordSuccess();
      reset();
      return outcome(true, 'transcript_ready', true, { transcript, transcriptLength: transcript.length });
    } catch (error) {
      const message = String(error && error.message || 'network');
      recordFailure(message, /http_40[13]|auth/.test(message));
      reset();
      return outcome(false, 'capture:transcribe_failed', true);
    }
  }

  async function cancel() {
    if (phase === 'idle') return outcome(false, 'capture:not_active', false);
    generation += 1;
    const target = recorder;
    phase = 'cancelling';
    clearCaptureTimer();
    await stopRecorder(target);
    reset();
    return outcome(true, 'capture_cancelled', true);
  }

  function snapshot() {
    return Object.freeze({
      version: VERSION,
      enabled,
      supported: supported(),
      ready: ready(),
      phase,
      active: phase !== 'idle',
      cooldownRemainingMs: Math.max(0, health.cooldownUntil - now()),
      consecutiveFailures: health.failures,
      lastOutcome: health.lastOutcome
    });
  }

  return Object.freeze({ version: VERSION, ready, start, submit, cancel, snapshot });
});
