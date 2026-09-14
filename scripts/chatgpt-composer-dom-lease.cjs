'use strict';

const selectors = ['#prompt-textarea', '[data-testid="prompt-textarea"]',
  'form [contenteditable="true"]', 'form textarea',
  'main [contenteditable="true"]', 'textarea[placeholder]'];

// Local research WebView only. React retains its editor; no state, request,
// credential, or transport function is replaced by this reversible CSS lease.
function pageOperation(action, nonce, version, selectors) {
  const key = '__elonComposerDomAcceptanceLease';
  const current = window[key], bridge = window.__elonChatGptBridge;
  const eligible = location.origin === 'https://chatgpt.com' &&
    window.__elonChatGptPrivateResearchEnabled === true && bridge?.version === version;
  if (action === 'probe') return { eligible, owned: current?.nonce === nonce,
    empty_route: location.pathname === '/' && !location.search && !location.hash };
  if (action === 'restore') {
    if (!current) return { restored: true, existed: false };
    if (current.nonce !== nonce) throw new Error('composer_lease_wrong_owner');
    return current.restore();
  }
  if (!eligible) throw new Error('composer_lease_identity_unavailable');
  if (action === 'state') {
    if (current?.nonce !== nonce) throw new Error('composer_lease_missing');
    return current.state();
  }
  if (action !== 'hide' || !/^[a-f0-9]{32}$/.test(nonce)) throw new Error('composer_lease_invalid_action');
  if (current) throw new Error('composer_lease_already_present');
  if (location.pathname !== '/' || location.search || location.hash) throw new Error('composer_lease_not_new_chat');
  const token = window.__elonChatGptDocumentToken;
  if (typeof token !== 'string' || !token) throw new Error('composer_lease_document_missing');
  const visible = () => Array.from(document.querySelectorAll(selectors.join(','))).filter(node => {
    const rect = node.getBoundingClientRect(), style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  }).length;
  const initialVisible = visible();
  if (!initialVisible) throw new Error('composer_lease_baseline_not_visible');
  const style = document.createElement('style');
  style.textContent = selectors.join(',') + '{display:none!important;visibility:hidden!important}';
  document.head.appendChild(style);
  let samples = 0, visibleSamples = 0, invalidSamples = 0, restored = false;
  let interval, timeout;
  const sameDocument = () => window.__elonChatGptDocumentToken === token;
  const sample = () => {
    samples++;
    if (visible() > 0) visibleSamples++;
    if (!sameDocument() || !style.isConnected) invalidSamples++;
  };
  const restore = () => {
    clearInterval(interval); clearTimeout(timeout);
    style.remove(); restored = true;
    if (window[key]?.nonce === nonce) delete window[key];
    return { restored: true, existed: true };
  };
  const state = () => {
    sample();
    return { schema: 'elon.composer_dom_lease.v1', active: !restored && sameDocument() && style.isConnected,
      initial_visible: initialVisible, visible_now: visible(), samples,
      visible_samples: visibleSamples, invalid_samples: invalidSamples };
  };
  window[key] = { nonce, state, restore };
  interval = setInterval(sample, 250);
  timeout = setTimeout(restore, 240000);
  return state();
}

function endpointUrl(value) {
  const url = new URL(value);
  if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port ||
      url.username || url.password || url.pathname !== '/' || url.search || url.hash) {
    throw new Error('composer_cdp_endpoint_invalid');
  }
  return url;
}

function socketUrl(value, endpoint) {
  const url = new URL(value);
  if (url.protocol !== 'ws:' || !['127.0.0.1', 'localhost'].includes(url.hostname) ||
      url.port !== endpoint.port || url.username || url.password ||
      !/^\/devtools\/page\/[a-zA-Z0-9_-]+$/.test(url.pathname) || url.search || url.hash) {
    throw new Error('composer_cdp_socket_invalid');
  }
  url.hostname = '127.0.0.1';
  return url.href;
}

function evaluate(url, expression) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(url);
    const finish = (error, value) => {
      clearTimeout(timeout); socket.close();
      if (error) reject(new Error(error)); else resolve(value);
    };
    const timeout = setTimeout(() => finish('composer_cdp_timeout'), 8000);
    socket.addEventListener('open', () => socket.send(JSON.stringify({ id: 1,
      method: 'Runtime.evaluate', params: { expression, returnByValue: true, awaitPromise: true } })));
    socket.addEventListener('message', event => {
      let message;
      try { message = JSON.parse(String(event.data)); } catch { return finish('composer_cdp_invalid_result'); }
      if (message.id !== 1) return;
      if (message.error || message.result?.exceptionDetails) return finish('composer_cdp_evaluation_failed');
      finish(null, message.result?.result?.value);
    });
    socket.addEventListener('error', () => finish('composer_cdp_socket_failed'));
  });
}

async function run(endpointValue, action, nonce, version) {
  const endpoint = endpointUrl(endpointValue);
  if (!['hide', 'state', 'restore'].includes(action) || !/^[a-f0-9]{32}$/.test(nonce) ||
      !Number.isInteger(version) || version < 1) throw new Error('composer_cdp_arguments_invalid');
  const response = await fetch(new URL('json', endpoint), { signal: AbortSignal.timeout(4000), redirect: 'error' });
  if (!response.ok) throw new Error('composer_cdp_listing_failed');
  const targets = await response.json();
  if (!Array.isArray(targets) || targets.length > 20) throw new Error('composer_cdp_listing_invalid');
  const candidates = [];
  const expression = operation => `(${pageOperation.toString()})(${JSON.stringify(operation)},` +
    `${JSON.stringify(nonce)},${version},${JSON.stringify(selectors)})`;
  for (const target of targets) {
    let url;
    try { url = new URL(target.url); } catch { continue; }
    if (url.origin !== 'https://chatgpt.com' || !target.webSocketDebuggerUrl) continue;
    const socket = socketUrl(target.webSocketDebuggerUrl, endpoint);
    const probe = await evaluate(socket, expression('probe'));
    if (action === 'hide' ? probe?.eligible && probe.empty_route : probe?.owned) candidates.push(socket);
  }
  if (action === 'restore' && candidates.length === 0) return { restored: true, existed: false };
  if (candidates.length !== 1) throw new Error('composer_cdp_target_not_unique');
  return evaluate(candidates[0], expression(action));
}

module.exports = { selectors, pageOperation, endpointUrl, socketUrl, run };
if (require.main === module) {
  run(process.argv[2], process.argv[3], process.argv[4], Number(process.argv[5]))
    .then(result => process.stdout.write(JSON.stringify(result)))
    .catch(error => { process.stderr.write(/^composer_[a-z_]+$/.test(error.message)
      ? error.message : 'composer_cdp_failed'); process.exitCode = 1; });
}
