(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com' && !root.__elonChatGptPrivateConversationShare) {
    root.__elonChatGptPrivateConversationShare = factory(root);
  }
})(typeof window === 'object' ? window : null, function (page, options) {
  'use strict';
  const contract = (options?.contract || page.__elonChatGptPrivateConversationShareContract).create(page, options);
  const transport = page.__elonChatGptPrivateTransport;
  let active = null, last = null, cooldown = 0;
  const outcome = (ok, code, attempted, url) => Object.freeze({ ok, code, attempted, ...(url ? { url } : {}) });

  async function execute(job) {
    let attempted = false;
    try {
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
        attempted = true;
        return (await page.__elonChatGptPrivateJsonRequest.request(page, path, {
          method, headers, credentials: 'include', cache: 'no-store', redirect: 'error', body: JSON.stringify(body),
          __elonPrivateTransport: 'conversation_share_v1',
        }, { timeoutMs: 7000, maxBytes: 256 * 1024, mode: 'json' })).payload;
      };
      // The standard authenticated full-conversation modal uses legacy create + publish.
      // Do not substitute message-slice /share/post or the v2 redesigned/guest route.
      const response = await request('/backend-api/share/create', 'POST', {
        current_node_id: binding.node, conversation_id: binding.id, is_anonymous: true,
      });
      if (!contract.current(binding)) return outcome(false, 'share_result_unconfirmed', true);
      const link = contract.created(response, binding.node);
      if (!link) return outcome(false, 'share_result_unconfirmed', true);
      if (contract.moderation(link.moderation) === 'blocked') return outcome(false, 'share_moderation_blocked', true);
      const published = await request('/backend-api/share/' + encodeURIComponent(link.id), 'PATCH', {
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
    if (confirmed !== true) return Promise.resolve(outcome(false, 'user_confirmation_required', false));
    if (page.__elonChatGptPrivateConversationMutationsEnabled !== true || !transport ||
        !page.__elonChatGptPrivateJsonRequest?.request) return Promise.resolve(outcome(false, 'share_context_unavailable', false));
    if (active || page.__elonChatGptPrivateConversationDelete?.busy?.() ||
        page.__elonChatGptPrivateConversationMutation?.state?.().state === 'busy') {
      return Promise.resolve(outcome(false, 'share_busy', false));
    }
    if (Date.now() < cooldown) return Promise.resolve(outcome(false, 'share_cooldown', false));
    const job = { path, readSnapshot }; active = job;
    return execute(job).then(result => {
      if (!result.ok && result.attempted) cooldown = Date.now() + 45000;
      return result;
    }).finally(() => { if (active === job) active = null; });
  }

  function handle(action, command, respond, readSnapshot) {
    if (action !== 'share_conversation') return false;
    start(command?.value, command?.selected, readSnapshot).then(result => {
      // This is the validated public result, not a credential or conversation body.
      respond(action, result.ok, result.ok ? 'share_link_ready:' + result.url : result.code);
    }).catch(() => respond(action, false, 'share_result_unconfirmed'));
    return true;
  }
  return Object.freeze({ version: 1, start, handle, busy: () => active !== null });
});
