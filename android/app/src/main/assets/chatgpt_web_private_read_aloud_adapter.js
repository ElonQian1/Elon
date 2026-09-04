(function () {
  'use strict';

  if (location.origin !== 'https://chatgpt.com') return;

  function create(scheduleSnapshot) {
    const transport = window.__elonChatGptPrivateReadAloudTransport;
    if (!transport || typeof transport.state !== 'function') return null;
    let unsubscribeTransport = null;
    let unsubscribeAuth = null;

    function addSnapshotFields(event) {
      const state = transport.state();
      event.privateReadAloudReady = state.ready === true;
      event.privateReadAloudState = String(state.state || 'idle');
      event.privateReadAloudContextId = String(state.contextId || '').slice(0, 160);
    }

    function handle(action, command, respond) {
      if (action !== 'toggle_private_read_aloud') return false;
      if (typeof transport.toggle !== 'function') {
        respond(action, false, 'private_read_aloud_unavailable');
        return true;
      }
      Promise.resolve(transport.toggle(String(command.value || '').slice(0, 160)))
        .then((outcome) => {
          const value = outcome && typeof outcome === 'object' ? outcome : {};
          respond(action, value.ok === true, String(value.detail || '').slice(0, 80));
          scheduleSnapshot(true);
        })
        .catch(() => {
          respond(action, false, 'private_read_aloud_failed');
          scheduleSnapshot(true);
        });
      return true;
    }

    function subscribe() {
      if (!unsubscribeTransport && typeof transport.subscribe === 'function') {
        unsubscribeTransport = transport.subscribe(() => scheduleSnapshot(true));
      }
      const authContext = window.__elonChatGptPrivateAuthContext;
      if (!unsubscribeAuth && authContext && typeof authContext.subscribe === 'function') {
        unsubscribeAuth = authContext.subscribe(() => scheduleSnapshot(true));
      }
    }

    function dispose() {
      if (typeof unsubscribeTransport === 'function') unsubscribeTransport();
      if (typeof unsubscribeAuth === 'function') unsubscribeAuth();
      unsubscribeTransport = null;
      unsubscribeAuth = null;
    }

    return Object.freeze({ addSnapshotFields, handle, subscribe, dispose });
  }

  window.__elonChatGptPrivateReadAloudAdapter = Object.freeze({ version: 1, create });
})();
