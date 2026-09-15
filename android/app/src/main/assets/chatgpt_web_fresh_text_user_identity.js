(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, signature: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root) root.__elonChatGptFreshTextUserIdentity = api;
})(typeof window === 'object' ? window : null, function (message) {
  'use strict';
  return JSON.stringify([message?.id, message?.author?.role,
    message?.channel ?? null, message?.recipient ?? 'all', message?.content?.content_type, message?.content?.parts,
    ...['attachments', 'system_hints', 'contextual_retry_message', 'is_contextual_retry_user_message',
      'is_visually_hidden_from_conversation', 'is_visually_hidden_reasoning_group', 'debug_internal_only']
      .map(key => { const value = message?.metadata?.[key];
        return value == null || value === false || Array.isArray(value) && !value.length ? null : value; })]);
});
