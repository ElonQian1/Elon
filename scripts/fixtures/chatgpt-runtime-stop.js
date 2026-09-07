'use strict';

const submit = require('../../android/app/src/main/assets/chatgpt_web_private_text_runtime_submit.js');
const stop = require('../../android/app/src/main/assets/chatgpt_web_private_stop_runtime.js');
const id = '11111111-2222-3333-4444-555555555555';
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };

function fixture(options = {}) {
  const calls = [], timers = new Map(), listeners = new Set(), loads = [];
  let serial = 0, settle, reject, credential = 'Bearer synthetic', request = 'synthetic-request';
  let active = true, asyncStatus = { value: 3 }, response;
  const conversation = { id: 'synthetic-client-thread', serverId$: () => id };
  const controller = { conversation };
  const props = { conversation, composerController: controller, isNewThread: false,
    currentRequestId: request, currentLeafId: 'synthetic-leaf', isDisabled: true,
    isComposerSubmissionReady: false };
  const shared = { getSharedProps: () => props, subscribeToSharedProps(fn) {
    listeners.add(fn); options.onSubscribe?.(); return () => listeners.delete(fn);
  } };
  const files = { files$: () => [], readyFiles$: () => [], hasUploadInProgress$: () => true };
  const top = { return: null, stateNode: {} }; top.stateNode.current = top;
  const fiber = { return: top, dependencies: { firstContext: {
    memoizedValue: { store: shared }, next: { memoizedValue: files }
  } } };
  const node = { isConnected: true, __reactFiber$fixture: fiber };
  const page = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/' + id },
    document: { querySelector: () => null, querySelectorAll: () => [] },
    performance: { getEntriesByName: () => [{}] },
    __elonChatGptDocumentToken: 'doc_runtime_stop', __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ authorization: credential }) },
    __elonChatGptPrivateStreamTransport: { subscribe(fn) { listeners.add(fn); return () => listeners.delete(fn); } },
    setTimeout(fn, ms) { timers.set(++serial, { fn, ms }); return serial; },
    clearTimeout(key) { timers.delete(key); }
  };
  const enums = { STREAMING: 3, UNREAD: 4, REALTIME: 5, REALTIME_BUSY: 6, REALTIME_BACKGROUND: 7 };
  const modules = {
    shared: { XM: key => { if (key !== conversation.id) throw Error('wrong thread'); return {}; },
      HM: { getRequestId: () => request }, Fl: key => key === 'synthetic-request' && active,
      Fx: value => { if (value !== conversation) throw Error('wrong owner'); return asyncStatus; }, v7: enums },
    conversation: { FVt(...args) {
      calls.push(args);
      if (response) return response();
      return new Promise((done, fail) => { settle = done; reject = fail; });
    } }
  };
  page.__elonChatGptPrivateTextRuntimeSubmit = submit.create(page);
  const api = stop.create(page, { ...options, loadRuntime: options.loadRuntime || (url => {
    loads.push(url);
    return Promise.resolve(url.includes('/conversation-small-') ? modules.conversation : modules.shared);
  }) });
  page.__elonChatGptPrivateStopRuntime = api;
  return { api, page, node, props, shared, files, fiber, top, modules, calls, timers, listeners, loads,
    command: { composer: node, requestId: 'mcp_stop' },
    publish: () => { for (const fn of [...listeners]) fn(); },
    settle: () => settle(), reject: () => reject(Error('synthetic failure')),
    response: fn => { response = fn; },
    setIdentity: () => { credential = 'Bearer changed-synthetic'; },
    setState(isActive, mode) { active = isActive; asyncStatus = mode == null ? null : { value: mode }; },
    setRequest(value) { request = value; props.currentRequestId = value; },
    runTimer(ms) {
      const found = [...timers].find(([, timer]) => timer.ms === ms);
      if (!found) return false;
      timers.delete(found[0]); found[1].fn(); return true;
    }
  };
}

module.exports = { fixture, flush, id };
