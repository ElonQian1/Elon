(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 2, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptGroupProject = api;
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  options = options || {};
  const policy = options.policy || page.__elonChatGptGroupProjectPolicy;
  const identity = page.__elonChatGptGroupProjectIdentity.create(page);
  let busy = false;
  async function request(owner, path, body) {
    if (!owner.current()) throw Error('project_identity_changed');
    const headers = { ...owner.headers };
    if (body) headers['Content-Type'] = 'application/json';
    const response = await page.__elonChatGptPrivateJsonRequest.request(page, path, {
      method: body ? 'POST' : 'GET', headers, credentials: 'include', cache: 'no-store', redirect: 'error',
      ...(body ? { body: JSON.stringify(body) } : {}), __elonPrivateTransport: 'group_project_v1',
    }, { timeoutMs: 12000, maxBytes: 4 * 1024 * 1024 });
    if (!owner.current()) throw Error('project_identity_changed');
    return response.payload;
  }
  async function read(owner, input, requireMarker = false) {
    if (!policy.project.test(input.projectId || '')) throw Error('project_input_invalid');
    const tag = policy.marker(input.bindingId, input.generation);
    const data = await request(owner, '/backend-api/gizmos/' + input.projectId);
    const result = policy.resource(data, input.projectId, requireMarker ? tag : null);
    if (input.conversationId) {
      if (!policy.uuid.test(input.conversationId)) throw Error('project_input_invalid');
      const thread = await request(owner, '/backend-api/conversation/' + input.conversationId);
      return { ...result, ...policy.conversation(thread, input.conversationId, result.projectId) };
    }
    return result;
  }
  async function reconcile(owner, input) {
    const tag = policy.marker(input.bindingId, input.generation), matches = [];
    let cursor = null;
    const seen = new Set();
    for (let index = 0; index < 10; index++) {
      const query = new URLSearchParams({ conversations_per_gizmo: '0', owned_only: 'true', limit: '100' });
      if (cursor) query.set('cursor', cursor);
      const list = await request(owner, '/backend-api/gizmos/snorlax/sidebar?' + query);
      if (!Array.isArray(list?.items) || list.items.length > 100) throw Error('project_response_invalid');
      for (const item of list.items) {
        const candidate = item?.gizmo;
        if (candidate?.gizmo?.instructions?.split('\n')[0] !== tag) continue;
        const id = candidate.gizmo.id;
        if (!policy.project.test(id)) throw Error('project_response_invalid');
        // Re-read permissions and memory scope; directory data cannot authorize sending.
        matches.push(await read(owner, { ...input, projectId: id, conversationId: null }, true));
      }
      if (list.cursor == null) {
        if (matches.length !== 1) throw Error(matches.length ? 'project_binding_ambiguous' : 'project_create_unresolved');
        return matches[0];
      }
      if (typeof list.cursor !== 'string' || list.cursor.length > 2048 || seen.has(list.cursor)) throw Error('project_response_invalid');
      cursor = list.cursor; seen.add(cursor);
    }
    throw Error('project_reconciliation_incomplete');
  }
  async function run(input) {
    if (busy) return { ok: false, code: 'project_busy' };
    busy = true;
    let writeStarted = false;
    try {
      if (!input || !['identity', 'read', 'create', 'reconcile'].includes(input.operation)) throw Error('project_input_invalid');
      const owner = await identity.bind();
      if (input.operation === 'identity') return { ok: true, code: 'project_identity_ready', accountScope: owner.accountScope };
      if (!policy.scope.test(input.accountScope || '') || input.accountScope !== owner.accountScope) throw Error('project_identity_changed');
      let result;
      if (input.operation === 'create') {
        const body = policy.createBody(input);
        // Caller journals create_begin before this irreversible request. Never replay an unknown write.
        writeStarted = true;
        const response = await request(owner, '/backend-api/projects', body);
        if (response?.error) throw Error('project_create_rejected');
        const id = response?.resource?.gizmo?.id;
        if (!policy.project.test(id || '')) throw Error('project_response_invalid');
        result = await read(owner, { ...input, projectId: id, conversationId: null }, true);
      } else result = await (input.operation === 'read' ? read(owner, input) : reconcile(owner, input));
      return { ok: true, code: 'project_ready', accountScope: owner.accountScope, ...result };
    } catch (error) {
      const raw = String(error?.message || '');
      const code = writeStarted ? 'project_create_unknown' : raw === 'http_404' ? 'project_not_found' :
        /^http_(401|403)$/.test(raw) ? 'project_auth_required' : raw === 'http_429' ? 'project_rate_limited' :
        /^project_[a-z_]+$/.test(raw) ? raw : 'project_unavailable';
      return { ok: false, code, ...(input?.operation === 'create' && !writeStarted ? { notSent: true } : {}),
        ...(['document', 'runtime', 'account', 'workspace', 'timeout', 'invalid_json', 'response_too_large', 'http', 'network'].includes(error?.identityReason)
        ? { identityReason: error.identityReason } : {}) };
    } finally { busy = false; }
  }
  return Object.freeze({ version: 2, run });
});
