(function () {
  'use strict';

  var VERSION = 4;
  var allowedOrigins = new Set(['https://google.com', 'https://www.google.com']);
  if (!allowedOrigins.has(location.origin)) return;

  var baseBridge = window.__elonGoogleWebBridge;
  var directory = window.__elonGoogleWebPrivateThreadDirectory;
  var nativeBridge = window.elonGoogleWebNative;
  if (!baseBridge || typeof baseBridge.command !== 'function' ||
      !directory || typeof directory.snapshot !== 'function' ||
      !nativeBridge || typeof nativeBridge.postMessage !== 'function') return;

  var existingState = window.__elonWinGooglePrivateConversationBridge;
  if (existingState && Number(existingState.version || 0) >= VERSION &&
      typeof existingState.rebind === 'function') {
    existingState.rebind(baseBridge, directory, nativeBridge);
    return;
  }
  var bindingRevision = 1;
  var bridgeProxy;

  function parse(raw) {
    try { return JSON.parse(String(raw || '{}')); }
    catch (_) { return {}; }
  }

  function requestId(value) {
    var id = String(value || '');
    return /^mcp_[a-z0-9]{1,32}$/.test(id) ? id : '';
  }

  function emitResult(action, ok, detail, id) {
    var event = {
      type: 'command_result',
      action: String(action || '').slice(0, 48),
      ok: ok === true,
      detail: String(detail || '').slice(0, 240)
    };
    if (id) event.requestId = id;
    nativeBridge.postMessage(JSON.stringify(event));
  }

  function safeConversationPath(value) {
    return /^\/c\/[A-Za-z0-9_-]{12,160}$/.test(String(value || ''));
  }

  function conversations() {
    var values = directory.snapshot();
    return Array.isArray(values) ? values.slice(0, 200) : [];
  }

  function resetPrivateReplyState() {
    var state = window.__elonWinGooglePrivateReplyState;
    if (state && typeof state.reset === 'function') state.reset();
  }

  function safeProviderUrl(value) {
    var url;
    try { url = new URL(String(value || ''), location.href); }
    catch (_) { return null; }
    if (url.origin !== location.origin) return null;
    if (url.pathname !== '/search') return null;
    var aiMode = url.searchParams.get('udm') === '50' || url.searchParams.get('aep') === '11';
    var conversationId = String(url.searchParams.get('csuir') || '').trim();
    return aiMode && conversationId ? url : null;
  }

  function command(raw) {
    var payload = parse(raw);
    var action = String(payload.action || '');
    var id = requestId(payload.requestId);
    if (action === 'list_conversations') {
      var items = conversations();
      if (!items.length) {
        emitResult(action, false, 'Google AI 官网会话目录仍在后台同步。', id);
        return;
      }
      emitResult(action, true, '已同步 ' + items.length + ' 个 Google AI 官网会话。', id);
      return;
    }
    if (action === 'open_conversation') {
      var path = String(payload.value || '');
      if (!safeConversationPath(path)) {
        emitResult(action, false, 'Google AI 会话地址无效。', id);
        return;
      }
      var item = conversations().find(function (candidate) {
        return candidate && candidate.path === path;
      });
      var target = item && safeProviderUrl(item.providerUrl);
      if (!target) {
        emitResult(action, false, 'Google AI 官网会话已变化，请先重新同步。', id);
        return;
      }
      resetPrivateReplyState();
      emitResult(action, true, '', id);
      location.assign(target.href);
      return;
    }
    if (action === 'new_conversation') resetPrivateReplyState();
    baseBridge.command(raw);
  }

  bridgeProxy = {
    command: command,
    dispose: function () {
      if (typeof baseBridge.dispose === 'function') baseBridge.dispose();
    },
  };
  Object.defineProperty(bridgeProxy, 'version', {
    enumerable: true,
    get: function () { return baseBridge.version; },
  });
  Object.defineProperty(bridgeProxy, 'documentToken', {
    enumerable: true,
    get: function () { return baseBridge.documentToken; },
  });
  bridgeProxy = Object.freeze(bridgeProxy);

  function rebind(nextBridge, nextDirectory, nextNative) {
    var changed = false;
    if (nextBridge && nextBridge !== bridgeProxy && nextBridge !== baseBridge &&
        typeof nextBridge.command === 'function') {
      if (typeof baseBridge.dispose === 'function') {
        try { baseBridge.dispose(); } catch (_) {}
      }
      baseBridge = nextBridge;
      changed = true;
    }
    if (nextDirectory && nextDirectory !== directory &&
        typeof nextDirectory.snapshot === 'function') {
      directory = nextDirectory;
      changed = true;
    }
    if (nextNative && nextNative !== nativeBridge &&
        typeof nextNative.postMessage === 'function') {
      nativeBridge = nextNative;
      changed = true;
    }
    if (changed) bindingRevision += 1;
    window.__elonGoogleWebBridge = bridgeProxy;
    return changed;
  }

  window.__elonGoogleWebBridge = bridgeProxy;
  window.__elonWinGooglePrivateConversationBridgeVersion = VERSION;
  window.__elonWinGooglePrivateConversationBridge = Object.freeze({
    version: VERSION,
    rebind: rebind,
    diagnostics: function () {
      return 'v' + VERSION + '|bindings=' + bindingRevision;
    },
  });
})();
