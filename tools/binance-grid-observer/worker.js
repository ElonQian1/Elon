'use strict';
importScripts('sanitize.js', 'store.js');
const sanitizer = globalThis.BinanceGridSanitizer;
const store = globalThis.BinanceGridStore;
const STORAGE_KEY = 'binance-grid-observer-session-v1';
const ORIGIN = 'https://www.binance.com';
let state = null;
let loaded = false;
let queue = Promise.resolve();
let pending = 0;

function enqueue(operation) {
  pending += 1;
  const result = queue.then(async () => {
    if (!loaded) {
      const saved = await chrome.storage.session.get(STORAGE_KEY);
      state = store.restore(saved[STORAGE_KEY], sanitizer.sanitizeObservation, Date.now());
      loaded = true;
    }
    return operation();
  });
  queue = result.catch(() => {}).finally(() => { pending -= 1; });
  return result;
}
async function persist() {
  if (state) await chrome.storage.session.set({ [STORAGE_KEY]: state });
  else await chrome.storage.session.remove(STORAGE_KEY);
}
function allowedPage(url) {
  try {
    const parsed = new URL(url);
    return parsed.origin === ORIGIN && !parsed.username && !parsed.password &&
      /^(?:\/[a-z]{2}(?:-[a-zA-Z]{2})?)?\/(?:trading-bots\/futures\/grid(?:\/|$)|futures(?:\/|$))/.test(parsed.pathname);
  } catch { return false; }
}
async function currentTab() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab && Number.isInteger(tab.id) && allowedPage(tab.url) ? tab : null;
}
async function stopRuntime(capture) {
  if (!capture) return;
  const target = { tabId: capture.tabId, documentIds: [capture.documentId] };
  await Promise.allSettled([
    chrome.scripting.executeScript({ target, world: 'MAIN',
      func: () => { globalThis.__binanceGridObserverV1?.stop(); } }),
    chrome.scripting.executeScript({ target, world: 'ISOLATED',
      func: () => { globalThis.__binanceGridBridgeV1?.stop(); } })
  ]);
}
async function reconcile() {
  if (!state?.active) return;
  if (Date.now() >= state.expiresAt) {
    store.stop(state, 'expired'); await stopRuntime(state); return;
  }
  try {
    const results = await chrome.scripting.executeScript({
      target: { tabId: state.tabId, documentIds: [state.documentId] }, world: 'MAIN',
      func: () => {
        const result = globalThis.__binanceGridObserverV1?.status();
        return { active: result?.active === true,
          reason: ['expired', 'navigated', 'stopped'].includes(result?.reason) ? result.reason : 'unavailable' };
      }
    });
    if (results.length !== 1 || results[0].documentId !== state.documentId || results[0].result?.active !== true) {
      store.stop(state, results[0]?.result?.reason || 'navigated');
      await stopRuntime(state);
    }
  } catch { store.stop(state, 'navigated'); await stopRuntime(state); }
}
async function start(tab) {
  if (state) { store.stop(state, 'replaced'); await stopRuntime(state); }
  state = null;
  await persist();
  // The first result pins every following injection to this exact document.
  const bridgeResults = await chrome.scripting.executeScript({
    target: { tabId: tab.id, frameIds: [0] }, world: 'ISOLATED', files: ['sanitize.js', 'bridge.js']
  });
  const documentId = bridgeResults[0]?.documentId;
  if (bridgeResults.length !== 1 || !documentId) return { ok: false, error: 'unsupported_browser' };
  const sessionId = crypto.randomUUID();
  const target = { tabId: tab.id, documentIds: [documentId] };
  const started = await chrome.scripting.executeScript({ target, world: 'ISOLATED',
    func: (id) => globalThis.__binanceGridBridgeV1.start(id), args: [sessionId] });
  if (started[0]?.result?.active !== true) return { ok: false, error: 'wrong_page' };
  state = store.create(tab.id, documentId, sessionId, Date.now());
  try {
    await chrome.scripting.executeScript({ target, world: 'MAIN', files: ['sanitize.js', 'observer.js'] });
    const result = await chrome.scripting.executeScript({ target, world: 'MAIN',
      func: (id) => globalThis.__binanceGridObserverV1.start(id), args: [sessionId] });
    if (result[0]?.result?.active !== true) throw new Error('start_failed');
    await persist();
    return { ok: true, state: store.status(state, Date.now()) };
  } catch {
    store.stop(state, 'unavailable'); await stopRuntime(state); await persist();
    return { ok: false, error: 'operation_failed' };
  }
}
async function handleUi(action) {
  const tab = await currentTab();
  if (!tab) return { ok: false, error: 'wrong_page' };
  if (action === 'start') return start(tab);
  let matches = state?.tabId === tab.id;
  if (matches) {
    try {
      const documents = await chrome.scripting.executeScript({ target: { tabId: tab.id, frameIds: [0] },
        world: 'ISOLATED', func: () => true });
      matches = documents.length === 1 && documents[0].documentId === state.documentId;
    } catch { matches = false; }
    if (!matches) {
      store.stop(state, 'navigated'); await stopRuntime(state); await persist();
    }
  }
  if (action === 'status') {
    if (matches) { await reconcile(); await persist(); }
    return { ok: true, state: store.status(matches ? state : null, Date.now()) };
  }
  if (!matches) return { ok: false, error: 'not_started' };
  if (action === 'stop' || action === 'clear') {
    store.stop(state, action === 'clear' ? 'cleared' : 'stopped');
    await stopRuntime(state);
    if (action === 'clear') { state.records = []; state.observations = 0; state.dropped = 0; }
    await persist();
    return { ok: true, state: store.status(state, Date.now()) };
  }
  if (action === 'export') {
    await reconcile(); await persist();
    if (!state.records.length) return { ok: false, error: 'no_samples' };
    return { ok: true, state: store.status(state, Date.now()), report: store.report(state, Date.now()) };
  }
  return { ok: false, error: 'operation_failed' };
}
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message?.type === 'observer-ui') {
    // MAIN messages can never request start/stop/export or access session storage.
    if (sender.id !== chrome.runtime.id || sender.tab || sender.url !== chrome.runtime.getURL('popup.html') ||
        !['start', 'stop', 'clear', 'status', 'export'].includes(message.action)) return false;
    enqueue(() => handleUi(message.action)).then(sendResponse,
      () => sendResponse({ ok: false, error: 'operation_failed' }));
    return true;
  }
  if (message?.type === 'observer-sample') {
    if (sender.id !== chrome.runtime.id || sender.origin !== ORIGIN || sender.frameId !== 0 ||
        !allowedPage(sender.url) || pending >= 64) return false;
    enqueue(async () => {
      const accepted = store.accept(state, sender, message, sanitizer.sanitizeObservation, Date.now());
      if (accepted || state?.active) await persist();
      return { ok: accepted };
    }).then(sendResponse, () => sendResponse({ ok: false }));
    return true;
  }
  return false;
});
chrome.tabs.onUpdated.addListener((tabId, change) => {
  if (change.status !== 'loading' && !change.url) return;
  enqueue(async () => {
    if (state?.tabId === tabId && state.active) {
      store.stop(state, 'navigated'); await stopRuntime(state); await persist();
    }
  }).catch(() => {});
});
chrome.tabs.onRemoved.addListener((tabId) => {
  enqueue(async () => {
    if (state?.tabId === tabId) { store.stop(state, 'navigated'); await persist(); }
  }).catch(() => {});
});
