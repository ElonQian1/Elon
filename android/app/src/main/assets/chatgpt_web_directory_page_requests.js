(function () {
  'use strict';
  if (location.origin !== 'https://chatgpt.com') return;
  window.__elonChatGptDirectoryPageRequests = Object.freeze({ version: 1, create });
  function create(directory, emit) {
    let generation = 0, requestId = '';
    function cancel() { generation++; requestId = ''; directory?.cancelBrowse?.(); }
    function handle(command, respond) {
      const action = command.action;
      if (action === 'cancel_directory_page') {
        if (command.value === requestId) cancel();
        respond(action, true, ''); return true;
      }
      if (action !== 'browse_directory_page') return false;
      let value;
      try { value = JSON.parse(command.value); } catch (_) {}
      if (!/^mcp_[a-z0-9]{1,32}$/.test(command.requestId || '') || !value ||
          !['conversations', 'projects'].includes(value.scope) && !/^g-p-[A-Za-z0-9_-]{1,160}$/.test(value.scope || '') ||
          typeof value.handle !== 'string' || value.handle && !/^dp_[a-f0-9]{32}$/.test(value.handle)) {
        respond(action, false, 'directory_scope_invalid'); return true;
      }
      const own = ++generation;
      requestId = command.requestId;
      if (typeof directory?.browsePage !== 'function') {
        respond(action, false, 'directory_reader_unavailable'); return true;
      }
      Promise.resolve().then(() => directory.browsePage(value.scope, value.handle)).then(result => {
        if (own !== generation) return;
        if (result?.ok) emit({ type: 'directory_page', version: 1, requestId: command.requestId,
          scope: value.scope, requestedHandle: value.handle, handle: result.handle,
          nextHandle: result.nextHandle, complete: result.complete, cached: result.cached,
          conversations: value.scope === 'projects' ? [] : result.items,
          projects: value.scope === 'projects' ? result.items : [] });
        respond(action, result?.ok === true, result?.code || 'directory_page_failed');
      }).catch(() => { if (own === generation) respond(action, false, 'directory_page_failed'); });
      return true;
    }
    return Object.freeze({ handle, cancel });
  }
})();
