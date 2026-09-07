(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 4, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateConversationShare) {
    root.__elonChatGptPrivateConversationShare = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const contract = (options?.contract || page.__elonChatGptPrivateConversationShareContract).create(page, options);
  const project = (options?.project || page.__elonChatGptPrivateProjectConversationShare)?.create(page, contract);
  const transport = page.__elonChatGptPrivateTransport;
  let active = null, last = null, cooldown = 0;
  const management = (options?.management || page.__elonChatGptPrivateSharedLinks)?.create(page, contract,
    { now: options?.now, invalidateCreated: () => { last = null; } });
  const outcome = (ok, code, attempted, url) => Object.freeze({ ok, code, attempted, ...(url ? { url } : {}) });

  async function execute(job) {
    let attempted = false;
    try {
      if (job.management) return management ? await management.run(job.management, job.confirmed) :
        outcome(false, 'share_list_unavailable', false);
      if (job.path?.startsWith('/g/')) {
        if (!project) throw new Error('share_project_scope_unconfirmed');
        return outcome(true, 'project_share_link_ready', false, await project.resolve(job.path, job.readSnapshot));
      }
      const binding = await contract.capture(job.path, job.readSnapshot);
      if (last && Date.now() - last.at < 60000 && contract.current(last.binding) &&
          binding.id === last.binding.id && binding.node === last.binding.node) return last.outcome;
      const headers = { Accept: 'application/json', 'Content-Type': 'application/json' };
      for (const [name, value] of Object.entries(transport.copySameOriginRequestHeaders?.() || {})) {
        if (['authorization', 'chatgpt-account-id', 'oai-device-id', 'oai-language', 'oai-client-version',
          'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
      }
      const request = async (path, method, body) => {
        if (!contract.current(binding)) throw new Error('share_context_changed');
        management?.invalidate();
        attempted = true;
        return (await page.__elonChatGptPrivateJsonRequest.request(page, path, {
          method, headers, credentials: 'include', cache: 'no-store', redirect: 'error', body: JSON.stringify(body),
          __elonPrivateTransport: 'conversation_share_v2',
        }, { timeoutMs: 7000, maxBytes: 256 * 1024, mode: 'json' })).payload;
      };
      // The redesigned flow publishes in create; the legacy modal requires a separate PATCH.
      const redesigned = binding.variant !== 'control';
      const response = await request(redesigned ? '/backend-api/share/v2/create' : '/backend-api/share/create', 'POST', {
        current_node_id: binding.node, conversation_id: binding.id, is_anonymous: true,
      });
      if (!contract.current(binding)) return outcome(false, 'share_result_unconfirmed', true);
      const link = contract.created(response, binding.node, binding.variant);
      if (!link) return outcome(false, 'share_result_unconfirmed', true);
      if (contract.moderation(link.moderation) === 'blocked') return outcome(false, 'share_moderation_blocked', true);
      const published = redesigned ? response : await request('/backend-api/share/' + encodeURIComponent(link.id), 'PATCH', {
          highlighted_message_id: link.highlighted, title: link.title, is_public: true,
          is_visible: true, is_anonymous: true, current_node_id: binding.node,
        });
      if (!contract.current(binding)) return outcome(false, 'share_result_unconfirmed', true);
      const state = contract.moderation(published?.moderation_state);
      if (state !== 'allowed') return outcome(false, state === 'blocked' ?
        'share_moderation_blocked' : 'share_result_unconfirmed', true);
      const result = outcome(true, 'share_link_ready', true, link.url);
      last = { binding, at: Date.now(), outcome: result };
      return result;
    } catch (error) {
      const code = String(error?.message || '');
      if (/^http_(401|403)$/.test(code)) page.__elonChatGptPrivateAuthContext?.invalidate?.('conversation_share_rejected');
      return outcome(false, attempted ? (/^http_\d{3}$/.test(code) ? 'share_' + code : 'share_result_unconfirmed') :
        (/^share_[a-z_]+$/.test(code) ? code : 'share_context_unavailable'), attempted);
    }
  }

  function start(path, confirmed, readSnapshot) {
    const managed = path && typeof path === 'object' ? path : null;
    if (confirmed !== true && managed?.operation !== 'list') return Promise.resolve(outcome(false, 'user_confirmation_required', false));
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true || !transport ||
        !page.__elonChatGptPrivateJsonRequest?.request) return Promise.resolve(outcome(false, 'share_context_unavailable', false));
    if (active || page.__elonChatGptPrivateConversationDelete?.busy?.() ||
        page.__elonChatGptPrivateConversationMutation?.state?.().state === 'busy') {
      return Promise.resolve(outcome(false, 'share_busy', false));
    }
    if (Date.now() < cooldown && managed?.operation !== 'list') return Promise.resolve(outcome(false, 'share_cooldown', false));
    const job = { path, readSnapshot, management: managed, confirmed }; active = job;
    return execute(job).then(result => {
      if (!result.ok && result.attempted) cooldown = Date.now() + 45000;
      return result;
    }).finally(() => { if (active === job) active = null; });
  }

  function handle(action, command, respond, readSnapshot) {
    if (action !== 'share_conversation') return false;
    let value = command?.value;
    if (typeof value === 'string' && value.startsWith('{')) {
      try { if (value.length > 1024) throw new Error(); value = JSON.parse(value); }
      catch (_) { respond(action, false, 'share_invalid_selection'); return true; }
    }
    start(value, command?.selected, readSnapshot).then(result => {
      // Preserve the audience-specific prefix; never confuse a members-only link with a public one.
      respond(action, result.ok, result.data ? JSON.stringify(result.data) :
        result.ok && result.url ? result.code + ':' + result.url : result.code);
    }).catch(() => respond(action, false, 'share_result_unconfirmed'));
    return true;
  }
  return Object.freeze({ version: 4, start, handle, busy: () => active !== null });
});
