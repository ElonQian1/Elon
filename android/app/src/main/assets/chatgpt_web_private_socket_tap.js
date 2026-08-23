(function () {
  'use strict';

  if (window.__elonChatGptPrivateStreamObserverEnabled !== true) return;
  if (location.origin !== 'https://chatgpt.com') return;
  const existing = window.__elonChatGptPrivateSocketTap;
  if (existing && Number(existing.version) >= 1) return;
  const OriginalWebSocket = window.WebSocket;
  if (typeof OriginalWebSocket !== 'function') return;

  const MAX_FRAME_LENGTH = 65536;
  const MAX_BUFFER_LENGTH = 262144;
  const MAX_BUFFERED_FRAMES = 24;
  const listeners = new Set();
  const buffered = [];
  let bufferedLength = 0;

  function allowed(url) {
    try {
      const parsed = new URL(String(url || ''), location.href);
      return parsed.protocol === 'wss:' &&
        (parsed.hostname === 'ws.chatgpt.com' || parsed.hostname === 'chatgpt.com');
    } catch (_) {
      return false;
    }
  }

  function publish(text) {
    const value = String(text || '');
    if (!value || value.length > MAX_FRAME_LENGTH) return;
    buffered.push(value);
    bufferedLength += value.length;
    while (buffered.length > MAX_BUFFERED_FRAMES || bufferedLength > MAX_BUFFER_LENGTH) {
      bufferedLength -= buffered.shift().length;
    }
    listeners.forEach((listener) => {
      try { listener(value); }
      catch (_) { /* The official socket remains authoritative. */ }
    });
  }

  function observe(data) {
    if (typeof data === 'string') return publish(data);
    if (data instanceof ArrayBuffer) {
      if (data.byteLength <= MAX_FRAME_LENGTH && typeof TextDecoder === 'function') {
        try { publish(new TextDecoder().decode(data)); }
        catch (_) { /* Ignore undecodable binary application frames. */ }
      }
      return;
    }
    if (typeof Blob === 'function' && data instanceof Blob && data.size <= MAX_FRAME_LENGTH) {
      Promise.resolve(data.text()).then(publish).catch(function () {});
    }
  }

  function WrappedWebSocket(url, protocols) {
    const socket = arguments.length > 1
      ? new OriginalWebSocket(url, protocols)
      : new OriginalWebSocket(url);
    if (allowed(url) && socket && typeof socket.addEventListener === 'function') {
      socket.addEventListener('message', function (event) { observe(event && event.data); });
    }
    return socket;
  }

  WrappedWebSocket.prototype = OriginalWebSocket.prototype;
  try { Object.setPrototypeOf(WrappedWebSocket, OriginalWebSocket); }
  catch (_) { /* Static constants remain available through most engines. */ }
  window.WebSocket = WrappedWebSocket;

  window.__elonChatGptPrivateSocketTap = Object.freeze({
    version: 1,
    subscribe: function (listener, replay) {
      if (typeof listener !== 'function') return function () {};
      listeners.add(listener);
      if (replay !== false) buffered.slice().forEach(listener);
      return function () { listeners.delete(listener); };
    },
    bufferedCount: function () { return buffered.length; },
    dispose: function () {
      listeners.clear();
      buffered.splice(0, buffered.length);
      bufferedLength = 0;
      if (window.WebSocket === WrappedWebSocket) window.WebSocket = OriginalWebSocket;
    }
  });
})();
