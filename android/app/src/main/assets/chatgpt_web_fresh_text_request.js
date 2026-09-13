(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 4, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextRequest = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const UUID = /^[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}$/i;
  const SLUG = /^[a-z0-9][a-z0-9._-]{0,127}$/i;
  const fail = code => { throw Error(code); };

  function body(context) {
    if (!context || (context.newConversation === true ? context.conversationId !== null ||
        context.parentId !== 'client-created-root' || context.parentRole !== 'root' || context.historyDisabled || context.doNotRemember :
        !UUID.test(context.conversationId || '') || !UUID.test(context.parentId || '')) ||
        !SLUG.test(context.model || '') || typeof context.historyDisabled !== 'boolean' ||
        typeof context.doNotRemember !== 'boolean' ||
        context.projectId != null && (typeof context.projectId !== 'string' ||
          !/^g-p-[a-f0-9]{32}$/i.test(context.projectId) || context.doNotRemember) ||
        context.effort != null && !SLUG.test(context.effort) ||
        context.serviceTier != null && !SLUG.test(context.serviceTier) ||
        ![null, 'search', 'picture_v2'].includes(context.tool ?? null) ||
        context.requestedDefaultModel != null && (context.newConversation !== true ||
          typeof context.requestedDefaultModel !== 'string' || !SLUG.test(context.requestedDefaultModel))) fail('context_invalid');
    const date = new Date();
    // AB omits conversation_id for a root request; only the server assigns it.
    // No previous request body, proof, conduit or model override is copied.
    return {
      action: 'next', ...(context.newConversation ? {} : { conversation_id: context.conversationId }),
      parent_message_id: context.parentId, model: context.model,
      client_prepare_state: 'none', timezone_offset_min: date.getTimezoneOffset(),
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      conversation_mode: context.projectId == null ? { kind: 'primary_assistant' }
        : { kind: 'gizmo_interaction', gizmo_id: context.projectId },
      system_hints: context.tool ? [context.tool] : [],
      supports_buffering: true, supported_encodings: ['v1'],
      ...(context.historyDisabled ? { history_and_training_disabled: true } : {}),
      ...(context.doNotRemember ? { is_do_not_remember: true } : {}),
      ...(context.effort != null ? { thinking_effort: context.effort } : {}),
      ...(context.serviceTier != null ? { service_tier: context.serviceTier } : {})
    };
  }

  function create(context, command) {
    if (!/^mcp_[a-z0-9]{1,32}$/.test(command?.requestId || '') ||
        typeof command.prompt !== 'string' || !command.prompt.trim() || command.prompt.length > 20000) fail('command_invalid');
    const preparedBody = body(context);
    const suppliedHeaders = context.projectHeaders ?? {};
    if (typeof suppliedHeaders !== 'object' || Array.isArray(suppliedHeaders) ||
        Object.entries(suppliedHeaders).some(([key, value]) => context.projectId == null ||
          key !== 'x-openai-locked-chats-pin' || typeof value !== 'string' || !value ||
          value.length > 65536 || /[\r\n]/.test(value))) fail('context_invalid');
    const projectHeaders = Object.freeze({ ...suppliedHeaders });
    const tool = context.tool ?? null;
    // The website prepares using the selected hint, but Search dispatch uses a
    // force flag. User metadata retains the selection for history/UI continuity.
    const dispatchBody = { ...preparedBody,
      ...(context.requestedDefaultModel != null ? { requested_default_model: context.requestedDefaultModel,
        one_off_model_override: true } : {}),
      ...(tool ? { enable_message_followups: true } : {}),
      ...(tool === 'search' ? { system_hints: [], force_use_search: true,
        client_reported_search_source: 'conversation_composer_web_icon' } : {}) };
    const userMessageId = page.crypto.randomUUID(), turnId = page.crypto.randomUUID();
    if (!UUID.test(userMessageId) || !UUID.test(turnId)) fail('identifier_invalid');
    let consumed = false, stopConduit = null, stopConsumed = false;
    return Object.freeze({ userMessageId, turnId,
      preparationBody: () => JSON.parse(JSON.stringify(preparedBody)),
      preparationHeaders: () => ({ ...projectHeaders }),
      securityMetadata: () => ({ systemHints: [...dispatchBody.system_hints],
        ...(preparedBody.conversation_mode.gizmo_id ? { conversationMode: { ...preparedBody.conversation_mode } } : {}) }),
      consumeStop(excludeAsyncTypes, current) {
        if (!consumed || !stopConduit || stopConsumed) fail('stop_ownership_unavailable');
        if (!Array.isArray(excludeAsyncTypes) || ![JSON.stringify([]), JSON.stringify(['pro_mode'])]
          .includes(JSON.stringify(excludeAsyncTypes))) fail('stop_scope_invalid');
        if (current() !== true) fail('context_changed');
        const conversationId = context.conversationId;
        if (!UUID.test(conversationId || '')) fail('stop_ownership_unavailable');
        stopConsumed = true;
        const conduit = stopConduit; stopConduit = null;
        return { requestBody: { conversation_id: conversationId, exclude_async_types: [...excludeAsyncTypes] },
          additionalHeaders: { 'x-conduit-token': conduit, 'x-oai-turn-trace-id': turnId } };
      },
      consume(preparation, security, headersFromSecurity, current) {
        if (consumed) fail('preparation_consumed');
        if (current() !== true) fail('context_changed');
        const conduit = preparation?.conduit_token, requirements = security?.chatReq;
        if (typeof conduit !== 'string' || !conduit || conduit.length > 65536 || /[\r\n]/.test(conduit)) fail('prepare_unconfirmed');
        if (requirements?.force_login) fail('login_required');
        if (!requirements || (!requirements.token && !requirements.prepare_token) ||
            typeof headersFromSecurity !== 'function') fail('security_unavailable');
        const headers = headersFromSecurity(requirements, security.turnstileToken, security.proofToken);
        if (!headers || typeof headers !== 'object' || !Object.keys(headers).length ||
            Object.entries(headers).some(([key, value]) => !/^openai-sentinel-[a-z-]+$/i.test(key) ||
              typeof value !== 'string' || !value || value.length > 65536 || /[\r\n]/.test(value))) fail('security_invalid');
        if (current() !== true) fail('context_changed');
        consumed = true;
        stopConduit = conduit;
        return {
          headers: { ...headers, ...projectHeaders, 'x-conduit-token': conduit, 'x-oai-turn-trace-id': turnId },
          body: { ...dispatchBody, conversation_mode: { ...dispatchBody.conversation_mode },
            system_hints: [...dispatchBody.system_hints], client_prepare_state: 'success', messages: [{
            id: userMessageId, author: { role: 'user' }, create_time: Date.now() / 1000,
            content: { content_type: 'text', parts: [command.prompt] },
            ...(tool ? { metadata: { system_hints: [tool] } } : {})
          }] }
        };
      }
    });
  }
  return Object.freeze({ create });
});
