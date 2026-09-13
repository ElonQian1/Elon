(function (encodedCommand, token, href) {
  'use strict';
  // The only retryable result is produced before entering the page command.
  try {
    if (window.__elonChatGptDocumentToken !== token || window.location.href !== href) return 'stale';
    const bridge = window.__elonChatGptBridge;
    if (!bridge || typeof bridge.command !== 'function') return 'missing';
    try {
      bridge.command(encodedCommand);
      return 'entered';
    } catch (_) {
      return 'unknown';
    }
  } catch (_) {
    return 'unknown';
  }
})
