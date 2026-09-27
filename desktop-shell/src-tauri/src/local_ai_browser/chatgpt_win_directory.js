(function (root) {
  'use strict';
  // Contract observed in 647288.dfed090bc7.js (2026-09-28): /pins is
  // separate from conversation history and carries typed {item_type,item} rows.
  const VERSION = 1;
  const SAFE_ID = /^[A-Za-z0-9_-]{1,160}$/;
  const PROJECT_ID = /^g-p-[A-Za-z0-9_-]{1,160}$/;
  function decodePins(payload, type) {
    if (!Array.isArray(payload) || payload.length > 200) throw Error('pins_schema_changed');
    return payload.map((pin, order) => {
      if (pin?.item_type !== type || !pin.item) throw Error('pins_schema_changed');
      const item = type === 'project' ? pin.item.gizmo : pin.item;
      const id = item?.id;
      const title = type === 'project' ? item?.display?.name : item?.title;
      if (!(type === 'project' ? PROJECT_ID : SAFE_ID).test(id || '') || typeof title !== 'string') {
        throw Error('pins_schema_changed');
      }
      return { id, title: title.slice(0,160), path: type === 'project' ? `/g/${id}/project` : `/c/${id}`,
        active: false, pinned: true, pinOrder: order, groupLabel: '', activityDates: [],
        pinnedAt: typeof pin.pinned_at === 'string' && Number.isFinite(Date.parse(pin.pinned_at)) ? Date.parse(pin.pinned_at) : null,
        updatedAt: typeof item.update_time === 'number' ? item.update_time : null };
    });
  }
  function mergeRows(rows, pins) {
    if (!pins) return rows;
    const byId = new Map(pins.map(item => [item.id, item]));
    const result = rows.map(row => {
      const pin = byId.get(row.id);
      byId.delete(row.id);
      return { ...row, pinned: Boolean(pin), pinOrder: pin?.pinOrder ?? null, pinnedAt: pin?.pinnedAt ?? null };
    });
    return result.concat([...byId.values()]);
  }
  const contract = { version: VERSION, decodePins, mergeRows };
  if (typeof module === 'object' && module.exports) module.exports = contract;
  if (root?.location?.origin !== 'https://chatgpt.com') return;
  const base = root.__elonChatGptPrivateConversationDirectory;
  if (!base || base.winDirectoryVersion >= VERSION) return;
  const fetch = root.fetch.bind(root);
  let pins = {}, account = '', flight = null, listener = null, generation = 0;
  root.__elonWinDirectoryRecentOnly = true;
  function identity(transport) {
    const headers = transport?.copySameOriginRequestHeaders?.() || {};
    return JSON.stringify(Object.entries(headers).filter(([key]) =>
      /^(authorization|chatgpt-account-id|oai-device-id)$/i.test(key)).sort());
  }
  function snapshot() {
    if (account && identity(root.__elonChatGptPrivateTransport) !== account) pins = {};
    const value = base.snapshot();
    return { ...value, conversations: mergeRows(value.conversations, pins.conversation),
      projects: mergeRows(value.projects, pins.project) };
  }
  function notify() { if (listener) listener(snapshot()); }
  async function readPins() {
    const transport = root.__elonChatGptPrivateTransport;
    const doc = root.__elonChatGptDocumentToken;
    const epoch = generation;
    let timer;
    const headers = await Promise.race([transport.acquireSameOriginRequestHeaders(), new Promise((_, reject) => {
      timer = root.setTimeout(() => reject(Error('identity_timeout')), 7000);
    })]).finally(() => root.clearTimeout(timer));
    const owner = identity(transport);
    if (account && owner !== account) pins = {};
    account = owner;
    const current = () => generation === epoch && root.__elonChatGptDocumentToken === doc &&
      root.__elonChatGptPrivateTransport === transport && identity(transport) === owner;
    const outcomes = await Promise.all(['conversation','project'].map(async type => {
      try {
        const result = await root.__elonChatGptPrivateJsonRequest.request({ fetch,
          AbortController: root.AbortController, setTimeout: root.setTimeout.bind(root),
          clearTimeout: root.clearTimeout.bind(root) }, `/backend-api/pins?item_type=${type}`,
        { method: 'GET', credentials: 'same-origin', headers }, { timeoutMs: 6000, maxBytes: 1024*1024, mode:'text' });
        const rows = decodePins(JSON.parse(result.text), type);
        if (!current()) return false;
        pins[type] = rows;
        return true;
      } catch (_) { return false; }
    }));
    if (current()) notify();
    return outcomes.every(Boolean);
  }
  async function refresh(scope) {
    if (scope && scope !== 'global' && scope !== 'history') return base.refreshScope(scope);
    if (flight) return flight;
    root.__elonWinDirectoryRecentOnly = scope !== 'history';
    flight = Promise.all([base.refresh(), readPins().catch(() => false)]).then(([result, pinsReady]) =>
      ({...result, pinsReady})).finally(() => { flight = null; root.__elonWinDirectoryRecentOnly = true; });
    return flight;
  }
  root.__elonChatGptPrivateConversationDirectory = Object.freeze({ ...base, winDirectoryVersion:VERSION,
    snapshot, refresh, setListener(next) { listener=next; base.setListener(notify); },
    cancelRefresh() { generation++; base.cancelRefresh?.(); } });
})(typeof window === 'object' ? window : null);
