(function () {
  'use strict';

  if (window.__elonChatGptPrivateReadAloudEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateReadAloudTransport;
  if (existing && Number(existing.version) >= 2) return;

  const privateTransport = window.__elonChatGptPrivateTransport;
  if (!privateTransport || typeof privateTransport.copySameOriginRequestHeaders !== 'function' ||
      typeof window.fetch !== 'function' || typeof window.Audio !== 'function') return;

  const listeners = new Set();
  const MESSAGE_ID = /^[A-Za-z0-9_-]{8,180}$/;
  const CONTEXT_ID = /^[A-Za-z0-9_.:-]{1,160}$/;
  const VOICE = /^[a-z][a-z0-9_-]{1,31}$/;
  const FORMATS = new Set(['aac', 'mp3', 'wav', 'opus', 'ogg']);
  const SETTINGS_KEY = 'elon.chatgpt.private.read_aloud.settings.v1';
  const REQUEST_TIMEOUT_MS = 15000;
  const STREAM_STALL_TIMEOUT_MS = 12000;
  const SOURCE_OPEN_TIMEOUT_MS = 4000;
  const PLAYBACK_START_TIMEOUT_MS = 8000;
  const BUFFER_APPEND_TIMEOUT_MS = 4000;
  const FAILURE_WINDOW_MS = 60000;
  const COOLDOWN_MS = 45000;
  const MAX_FAILURES = 2;

  let phase = 'idle';
  let activeContextId = '';
  let failureCode = '';
  let controller = null;
  let streamReader = null;
  let audio = null;
  let mediaSource = null;
  let sourceBuffer = null;
  let objectUrl = '';
  let requestTimer = null;
  let playbackStartTimer = null;
  let generation = 0;
  let failureTimes = [];
  let cooldownUntil = 0;
  let observedSettings = loadSettings();
  let performanceObserver = null;

  function loadSettings() {
    let parsed = null;
    try { parsed = JSON.parse(window.sessionStorage.getItem(SETTINGS_KEY) || 'null'); }
    catch (_) { /* Defaults remain authoritative. */ }
    return {
      voice: parsed && VOICE.test(String(parsed.voice || '')) ? String(parsed.voice) : 'ember',
      format: parsed && FORMATS.has(String(parsed.format || '')) ? String(parsed.format) : 'aac'
    };
  }

  function saveSettings(next) {
    observedSettings = next;
    try { window.sessionStorage.setItem(SETTINGS_KEY, JSON.stringify(next)); }
    catch (_) { /* Storage is an optimization, never a requirement. */ }
  }

  function conversationId() {
    const match = String(location.pathname || '').match(
      /^(?:\/c\/([A-Za-z0-9_-]{8,180})|\/g\/g-p-[A-Za-z0-9_-]{1,160}\/c\/([A-Za-z0-9_-]{8,180}))\/?$/
    );
    return match ? String(match[1] || match[2] || '') : '';
  }

  function observeSynthesisUrl(rawUrl) {
    let url;
    try { url = new URL(String(rawUrl || ''), location.href); }
    catch (_) { return; }
    if (url.origin !== location.origin || url.pathname !== '/backend-api/synthesize') return;
    const voice = String(url.searchParams.get('voice') || '');
    const format = String(url.searchParams.get('format') || '');
    if (!VOICE.test(voice) || !FORMATS.has(format)) return;
    if (voice !== observedSettings.voice || format !== observedSettings.format) {
      saveSettings({ voice, format });
    }
  }

  function installSettingsObserver() {
    try {
      performance.getEntriesByType('resource').forEach((entry) => observeSynthesisUrl(entry.name));
      if (typeof PerformanceObserver !== 'function') return;
      performanceObserver = new PerformanceObserver((list) => {
        list.getEntries().forEach((entry) => observeSynthesisUrl(entry.name));
      });
      performanceObserver.observe({ type: 'resource', buffered: true });
    } catch (_) { performanceObserver = null; }
  }

  function notify() {
    const value = state();
    listeners.forEach((listener) => {
      try { listener(value); }
      catch (_) { /* Native snapshots remain the source of truth. */ }
    });
  }

  function clearRequestTimer() {
    if (requestTimer !== null) window.clearTimeout(requestTimer);
    requestTimer = null;
  }

  function clearPlaybackStartTimer() {
    if (playbackStartTimer !== null) window.clearTimeout(playbackStartTimer);
    playbackStartTimer = null;
  }

  function armRequestTimer(delayMs, onTimeout) {
    clearRequestTimer();
    requestTimer = window.setTimeout(onTimeout, delayMs);
  }

  function releaseAudio() {
    clearRequestTimer();
    clearPlaybackStartTimer();
    if (streamReader) {
      try {
        const cancellation = streamReader.cancel();
        if (cancellation && typeof cancellation.catch === 'function') cancellation.catch(() => {});
      }
      catch (_) { /* The stream may already be closed. */ }
    }
    streamReader = null;
    if (controller) {
      try { controller.abort(); }
      catch (_) { /* It may already be settled. */ }
    }
    controller = null;
    if (audio) {
      audio.onended = null;
      audio.onerror = null;
      try { audio.pause(); }
      catch (_) { /* The element may not have started. */ }
      try { audio.removeAttribute('src'); audio.load(); }
      catch (_) { /* Releasing the blob URL below is sufficient. */ }
      try { audio.remove(); }
      catch (_) { /* Detached elements need no cleanup. */ }
    }
    audio = null;
    if (sourceBuffer) {
      try { if (sourceBuffer.updating) sourceBuffer.abort(); }
      catch (_) { /* The MediaSource may already be closed. */ }
    }
    sourceBuffer = null;
    mediaSource = null;
    if (objectUrl) {
      try { URL.revokeObjectURL(objectUrl); }
      catch (_) { /* The URL may already be revoked. */ }
    }
    objectUrl = '';
  }

  function setState(nextPhase, contextId, detail) {
    phase = nextPhase;
    activeContextId = String(contextId || '').slice(0, 160);
    failureCode = String(detail || '').slice(0, 80);
    notify();
  }

  function normalizeCooldown() {
    if (phase === 'cooldown' && Date.now() >= cooldownUntil) {
      phase = 'idle';
      activeContextId = '';
      failureCode = '';
      cooldownUntil = 0;
    }
  }

  function state() {
    normalizeCooldown();
    return Object.freeze({
      ready: !!conversationId(),
      state: phase,
      contextId: activeContextId,
      failure: failureCode,
      cooldownRemainingMs: Math.max(0, cooldownUntil - Date.now())
    });
  }

  function recordFailure(contextId, code) {
    releaseAudio();
    const now = Date.now();
    failureTimes = failureTimes.filter((value) => now - value < FAILURE_WINDOW_MS);
    failureTimes.push(now);
    if (failureTimes.length >= MAX_FAILURES) {
      cooldownUntil = now + COOLDOWN_MS;
      setState('cooldown', contextId, code);
    } else {
      setState('failed', contextId, code);
    }
    return { ok: false, detail: code };
  }

  function selectorValue(value) {
    if (window.CSS && typeof window.CSS.escape === 'function') return window.CSS.escape(value);
    return String(value || '').replace(/[^A-Za-z0-9_-]/g, '\\$&');
  }

  function assistantMessageId(contextId) {
    const value = String(contextId || '');
    if (!CONTEXT_ID.test(value)) return '';
    if (MESSAGE_ID.test(value) && !/^conversation-turn-/i.test(value)) return value;
    const escaped = selectorValue(value);
    const node = document.querySelector('[data-testid="' + escaped + '"], ' +
      '[data-message-id="' + escaped + '"], #' + escaped);
    if (!node) return '';
    const turn = node.closest('[data-testid^="conversation-turn-"]') || node;
    const roleNode = turn.matches('[data-message-author-role]')
      ? turn
      : turn.querySelector('[data-message-author-role]');
    const role = String(roleNode && roleNode.getAttribute('data-message-author-role') || '');
    if (role && role !== 'assistant') return '';
    const candidates = [
      turn.getAttribute && turn.getAttribute('data-message-id'),
      roleNode && roleNode.getAttribute('data-message-id'),
      turn.querySelector('[data-message-id]') &&
        turn.querySelector('[data-message-id]').getAttribute('data-message-id')
    ];
    return String(candidates.find((candidate) => MESSAGE_ID.test(String(candidate || ''))) || '');
  }

  function stop(reason) {
    generation += 1;
    releaseAudio();
    const previous = phase;
    failureTimes = reason === 'user' ? [] : failureTimes;
    cooldownUntil = reason === 'user' ? 0 : cooldownUntil;
    setState('idle', '', '');
    return { ok: true, detail: previous === 'idle' ? 'already_idle' : 'playback_stopped' };
  }

  function createPlaybackSignal(operation, contextId) {
    let resolvePromise;
    const signal = {
      invoked: false,
      settled: false,
      failed: false,
      promise: new Promise((resolve) => { resolvePromise = resolve; }),
      succeed: () => {
        if (signal.settled || operation !== generation) return;
        signal.settled = true;
        resolvePromise({ ok: true, detail: 'playback_started' });
      },
      fail: (code) => {
        if (signal.failed || operation !== generation) return;
        signal.failed = true;
        const result = recordFailure(contextId, code);
        if (!signal.settled) {
          signal.settled = true;
          resolvePromise(result);
        }
      }
    };
    return signal;
  }

  function createAttachedAudio(operation, contextId, signal) {
    const element = document.createElement('audio');
    element.preload = 'auto';
    element.setAttribute('playsinline', '');
    element.setAttribute('aria-hidden', 'true');
    element.style.display = 'none';
    element.onended = () => {
      if (operation !== generation) return;
      generation += 1;
      releaseAudio();
      failureTimes = [];
      setState('idle', '', '');
    };
    element.onerror = () => signal.fail('playback_error');
    document.documentElement.appendChild(element);
    audio = element;
    return element;
  }

  function beginPlayback(operation, contextId, signal) {
    if (signal.invoked || signal.failed || operation !== generation || !audio) return;
    signal.invoked = true;
    clearPlaybackStartTimer();
    playbackStartTimer = window.setTimeout(
      () => signal.fail('playback_start_timeout'),
      PLAYBACK_START_TIMEOUT_MS
    );
    let playResult;
    try { playResult = audio.play(); }
    catch (_) {
      signal.fail('playback_start_failed');
      return;
    }
    Promise.resolve(playResult).then(() => {
      if (operation !== generation || signal.failed) return;
      clearPlaybackStartTimer();
      failureTimes = [];
      cooldownUntil = 0;
      setState('playing', contextId, '');
      signal.succeed();
    }).catch(() => signal.fail('playback_start_failed'));
  }

  function waitForMediaSourceOpen(operation) {
    return new Promise((resolve, reject) => {
      if (!mediaSource || operation !== generation) {
        reject(new Error('playback_stopped'));
        return;
      }
      if (mediaSource.readyState === 'open') {
        resolve();
        return;
      }
      let timer = null;
      const cleanup = () => {
        if (timer !== null) window.clearTimeout(timer);
        timer = null;
        if (!mediaSource) return;
        mediaSource.removeEventListener('sourceopen', onOpen);
        mediaSource.removeEventListener('sourceclose', onClose);
      };
      const onOpen = () => {
        cleanup();
        if (operation === generation) resolve();
        else reject(new Error('playback_stopped'));
      };
      const onClose = () => {
        cleanup();
        reject(new Error('media_source_closed'));
      };
      mediaSource.addEventListener('sourceopen', onOpen);
      mediaSource.addEventListener('sourceclose', onClose);
      timer = window.setTimeout(() => {
        cleanup();
        reject(new Error('media_source_timeout'));
      }, SOURCE_OPEN_TIMEOUT_MS);
    });
  }

  function appendMediaChunk(operation, chunk) {
    return new Promise((resolve, reject) => {
      if (!sourceBuffer || operation !== generation) {
        reject(new Error('playback_stopped'));
        return;
      }
      let timer = null;
      const cleanup = () => {
        if (timer !== null) window.clearTimeout(timer);
        timer = null;
        if (!sourceBuffer) return;
        sourceBuffer.removeEventListener('updateend', onDone);
        sourceBuffer.removeEventListener('error', onError);
        sourceBuffer.removeEventListener('abort', onAbort);
      };
      const onDone = () => { cleanup(); resolve(); };
      const onError = () => { cleanup(); reject(new Error('buffer_error')); };
      const onAbort = () => { cleanup(); reject(new Error('buffer_aborted')); };
      sourceBuffer.addEventListener('updateend', onDone);
      sourceBuffer.addEventListener('error', onError);
      sourceBuffer.addEventListener('abort', onAbort);
      timer = window.setTimeout(() => {
        cleanup();
        reject(new Error('buffer_timeout'));
      }, BUFFER_APPEND_TIMEOUT_MS);
      try {
        const bytes = chunk.byteOffset === 0 && chunk.byteLength === chunk.buffer.byteLength
          ? chunk.buffer
          : chunk.buffer.slice(chunk.byteOffset, chunk.byteOffset + chunk.byteLength);
        sourceBuffer.appendBuffer(bytes);
      } catch (_) {
        cleanup();
        reject(new Error('buffer_append_failed'));
      }
    });
  }

  async function pumpStreamingAudio(operation, contextId, signal, onStall) {
    let receivedAudio = false;
    try {
      while (operation === generation) {
        armRequestTimer(STREAM_STALL_TIMEOUT_MS, onStall);
        const part = await streamReader.read();
        clearRequestTimer();
        if (operation !== generation) return;
        if (part.done) break;
        if (!part.value || part.value.byteLength <= 0) continue;
        await appendMediaChunk(operation, part.value);
        receivedAudio = true;
        beginPlayback(operation, contextId, signal);
      }
      if (operation !== generation) return;
      if (!receivedAudio) {
        signal.fail('empty_audio');
        return;
      }
      streamReader = null;
      if (mediaSource && mediaSource.readyState === 'open') mediaSource.endOfStream();
    } catch (error) {
      if (operation !== generation) return;
      signal.fail(error && error.name === 'AbortError' ? 'stream_timeout' : 'stream_failed');
    }
  }

  async function pumpBufferedAudio(operation, contextId, contentType, signal, onStall) {
    const chunks = [];
    try {
      while (operation === generation) {
        armRequestTimer(STREAM_STALL_TIMEOUT_MS, onStall);
        const part = await streamReader.read();
        clearRequestTimer();
        if (operation !== generation) return;
        if (part.done) break;
        if (part.value && part.value.byteLength > 0) chunks.push(part.value);
      }
      if (operation !== generation) return;
      if (chunks.length === 0) {
        signal.fail('empty_audio');
        return;
      }
      streamReader = null;
      objectUrl = URL.createObjectURL(new Blob(chunks, { type: contentType }));
      audio.src = objectUrl;
      audio.load();
      beginPlayback(operation, contextId, signal);
    } catch (error) {
      if (operation !== generation) return;
      signal.fail(error && error.name === 'AbortError' ? 'stream_timeout' : 'stream_failed');
    }
  }

  async function start(contextId) {
    normalizeCooldown();
    if (phase === 'cooldown') return { ok: false, detail: 'circuit_open' };
    const targetConversationId = conversationId();
    if (!targetConversationId) return recordFailure(contextId, 'conversation_unavailable');
    const messageId = assistantMessageId(contextId);
    if (!messageId) return recordFailure(contextId, 'message_identity_unavailable');
    const headers = privateTransport.copySameOriginRequestHeaders();
    if (!headers || typeof headers !== 'object') {
      return recordFailure(contextId, 'runtime_authorization_unavailable');
    }

    releaseAudio();
    const operation = ++generation;
    controller = new AbortController();
    let stalled = false;
    const abortForStall = () => {
      stalled = true;
      if (controller) controller.abort();
    };
    armRequestTimer(REQUEST_TIMEOUT_MS, abortForStall);
    setState('loading', contextId, '');

    const query = new URLSearchParams({
      message_id: messageId,
      conversation_id: targetConversationId,
      voice: observedSettings.voice,
      format: observedSettings.format
    });
    try {
      const response = await window.fetch('/backend-api/synthesize?' + query.toString(), {
        method: 'GET',
        credentials: 'include',
        headers,
        cache: 'no-store',
        signal: controller.signal
      });
      clearRequestTimer();
      if (operation !== generation) return { ok: true, detail: 'playback_stopped' };
      const contentType = String(response && response.headers &&
        response.headers.get('content-type') || '').toLowerCase();
      if (!response || !response.ok || !contentType.startsWith('audio/')) {
        return recordFailure(contextId, 'synthesis_rejected');
      }
      if (!response.body || typeof response.body.getReader !== 'function') {
        return recordFailure(contextId, 'audio_stream_unavailable');
      }
      const signal = createPlaybackSignal(operation, contextId);
      createAttachedAudio(operation, contextId, signal);
      streamReader = response.body.getReader();
      const mediaType = contentType.split(';')[0].trim();
      if (typeof MediaSource === 'function' && MediaSource.isTypeSupported(mediaType)) {
        mediaSource = new MediaSource();
        objectUrl = URL.createObjectURL(mediaSource);
        audio.src = objectUrl;
        audio.load();
        await waitForMediaSourceOpen(operation);
        if (operation !== generation) return { ok: true, detail: 'playback_stopped' };
        sourceBuffer = mediaSource.addSourceBuffer(mediaType);
        try { sourceBuffer.mode = 'sequence'; }
        catch (_) { /* The default segment mode remains valid. */ }
        void pumpStreamingAudio(operation, contextId, signal, abortForStall);
      } else {
        void pumpBufferedAudio(operation, contextId, mediaType, signal, abortForStall);
      }
      return await signal.promise;
    } catch (_) {
      clearRequestTimer();
      if (operation !== generation) return { ok: true, detail: 'playback_stopped' };
      return recordFailure(contextId, stalled ? 'request_timeout' : 'request_failed');
    }
  }

  function toggle(contextId) {
    const value = String(contextId || '');
    if (!CONTEXT_ID.test(value)) return Promise.resolve({ ok: false, detail: 'invalid_context_id' });
    if (activeContextId === value && (phase === 'loading' || phase === 'playing')) {
      return Promise.resolve(stop('user'));
    }
    if (phase === 'loading' || phase === 'playing') stop('replace');
    return start(value);
  }

  installSettingsObserver();
  window.__elonChatGptPrivateReadAloudTransport = Object.freeze({
    version: 2,
    enabled: true,
    state,
    toggle,
    stop: () => stop('user'),
    subscribe: (listener) => {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      return () => listeners.delete(listener);
    }
  });
})();
