(function (root, factory) {
  'use strict';
  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptHistoryParentDiagnostic = api;
})(typeof window === 'object' ? window : null, function () {
  'use strict';
  const SCHEMA = 'elon.history_parent.v1';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const own = (value, key) => value && Object.prototype.hasOwnProperty.call(value, key);
  const object = value => value && typeof value === 'object' && !Array.isArray(value);
  const empty = code => ({ schema: SCHEMA, code, user_found: false, nodes: [], terminal: 'not_observed' });
  const kind = id => id == null ? 'null' : id === '' ? 'empty_root' :
    id === 'client-created-root' ? 'client_root' : typeof id === 'string' && UUID.test(id) ? 'uuid' : 'other';
  const role = message => message == null ? 'none' :
    ['root', 'system', 'developer', 'user', 'assistant', 'tool'].includes(message.author?.role)
      ? message.author.role : 'other';

  function describe(payload) {
    if (!object(payload?.mapping)) return empty('mapping_missing');
    const mapping = payload.mapping, seen = new Set();
    let cursor = payload.current_node, user;
    while (typeof cursor === 'string' && seen.size < 4096 && !seen.has(cursor) && own(mapping, cursor)) {
      seen.add(cursor);
      const node = mapping[cursor];
      if (!object(node) || node.id !== cursor) break;
      if (role(node.message) === 'user' && node.message.id === cursor) { user = node; break; }
      cursor = node.parent;
    }
    if (!user) return empty('user_missing');
    const value = { ...empty('observed'), user_found: true };
    seen.clear();
    cursor = user.id;
    while (value.nodes.length < 16) {
      if (cursor == null) { value.terminal = 'null_parent'; break; }
      if (typeof cursor !== 'string' || !own(mapping, cursor)) { value.terminal = 'missing_parent'; break; }
      if (seen.has(cursor)) { value.terminal = 'cycle'; break; }
      seen.add(cursor);
      const node = mapping[cursor];
      if (!object(node)) { value.terminal = 'invalid_node'; break; }
      const parent = own(mapping, node.parent) ? mapping[node.parent] : null;
      const content = node.message?.content?.content_type;
      value.nodes.push({ role: role(node.message), id_kind: kind(cursor), parent_kind: kind(node.parent),
        node_id_matches: node.id === cursor, message_id_matches: node.message?.id === cursor,
        hidden: node.message?.metadata?.is_visually_hidden_from_conversation === true,
        content_kind: ['text', 'multimodal_text', 'code', 'execution_output', 'user_editable_context',
          'system_error'].includes(content) ? content : content == null ? 'none' : 'other',
        reciprocal: Array.isArray(parent?.children) && parent.children.includes(cursor),
        children: Array.isArray(node.children) ? Math.min(4096, node.children.length) : 0 });
      cursor = node.parent;
    }
    if (value.terminal === 'not_observed') value.terminal = 'limit';
    return value;
  }

  const flights = new WeakMap();
  function inspect(page) {
    if (flights.has(page)) return flights.get(page);
    const result = read(page).finally(() => { if (flights.get(page) === result) flights.delete(page); });
    flights.set(page, result);
    return result;
  }

  async function read(page) {
    const href = page.location?.href, token = page.__elonChatGptDocumentToken, document = page.document;
    let url;
    try { url = new URL(href); } catch (_) { return empty('route_unsupported'); }
    const match = /^\/c\/([a-f0-9-]{36})$/i.exec(url.pathname);
    if (url.origin !== 'https://chatgpt.com' || !match || !UUID.test(match[1]) || url.search || url.hash ||
        url.username || url.password) return empty('route_unsupported');
    if (!document || !/^doc_[A-Za-z0-9_-]{1,160}$/.test(token || '')) return empty('document_unavailable');
    const bindings = page.__elonChatGptPrivateRuntimeBindings;
    if (bindings?.state?.().profile_id !== 'web_20260912' || typeof bindings.load !== 'function') {
      return empty('runtime_unavailable');
    }
    const controller = new page.AbortController();
    let timer;
    const current = () => page.document === document && page.location?.href === href &&
      page.__elonChatGptDocumentToken === token && page.__elonChatGptPrivateRuntimeBindings === bindings &&
      bindings.state?.().profile_id === 'web_20260912';
    try {
      // The reviewed history loader calls its raw network callback before tree
      // normalization, and never hydrates when shouldApplyResponse returns false.
      return await Promise.race([new Promise(resolve => {
        timer = page.setTimeout(() => { controller.abort(); resolve(empty('timeout')); }, 5000);
      }), (async () => {
        const [shared, history] = await Promise.all([bindings.load('shared'), bindings.load('conversation')]);
        if (!current() || controller.signal.aborted) return empty('owner_changed');
        const contract = page.__elonChatGptPrivateModelContract?.create(page);
        const account = () => contract?.withRuntimeIdentity({}, shared)?.account;
        const owner = account();
        if (!owner) return empty('identity_unavailable');
        if (typeof history?.textHydrateHistory !== 'function') return empty('runtime_unavailable');
        let output = empty('payload_missing');
        await history.textHydrateHistory(match[1], { forceNetworkFetch: true, skipIfExisting: false,
          signal: controller.signal, source: 'native_history_parent_v1', shouldApplyResponse: () => false,
          onConversationLoadedFromNetwork(payload) {
            if (!current() || controller.signal.aborted || account() !== owner) return;
            output = payload?.conversation_id === match[1] ? describe(payload) : empty('conversation_mismatch');
          } });
        return current() && !controller.signal.aborted && account() === owner ? output : empty('owner_changed');
      })()]);
    } catch (_) { return empty(current() ? 'read_failed' : 'owner_changed'); }
    finally { page.clearTimeout(timer); controller.abort(); }
  }

  function handle(page, action, command, respond) {
    if (action !== 'private_protocol_probe' || command.value !== 'history_parent') return false;
    inspect(page).then(value => respond(action, value.code === 'observed', JSON.stringify(value)),
      () => respond(action, false, JSON.stringify(empty('read_failed'))));
    return true;
  }
  return Object.freeze({ version: 1, describe, inspect, handle });
});
