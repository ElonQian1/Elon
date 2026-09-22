(function (root, factory) {
  'use strict';
  const api = Object.freeze({ version: 1, create: factory });
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root?.location?.origin === 'https://chatgpt.com') root.__elonChatGptGroupProjectIdentity = api;
})(typeof window === 'object' ? window : null, function (page) {
  'use strict';
  const validId = value => typeof value === 'string' && /^[A-Za-z0-9_-]{1,256}$/.test(value);
  const identity = () => page.__elonChatGptPrivateConversationShareContract.create(page).identity();
  function unavailable(reason) {
    return Object.assign(Error('project_identity_unavailable'), { identityReason: reason });
  }
  async function bind() {
    const token = page.__elonChatGptDocumentToken, document = page.document;
    if (page.location.origin !== 'https://chatgpt.com' || !/^doc_[a-z0-9_]{3,80}$/.test(token || '')) throw unavailable('document');
    if (!identity()) await page.__elonChatGptPrivateAuthContext.acquireRequestHeaders();
    const before = identity();
    if (!before) throw unavailable('account');
    const current = () => document === page.document && token === page.__elonChatGptDocumentToken &&
      page.location.origin === 'https://chatgpt.com' && identity() === before;
    const read = async (path, headers) => {
      if (!current()) throw Error('project_identity_changed');
      let response;
      try {
        response = await page.__elonChatGptPrivateJsonRequest.request(page, path, {
          method: 'GET', credentials: 'include', cache: 'no-store', redirect: 'error', headers,
          __elonPrivateTransport: 'group_project_identity_v1',
        }, { timeoutMs: 5000, maxBytes: 1024 * 1024 });
      } catch (error) {
        // A missing identity endpoint is never evidence that the user's project was deleted.
        const raw = error?.message || '';
        const code = /^http_(401|403)$/.test(raw) ? 'project_auth_required' :
          raw === 'http_429' ? 'project_rate_limited' : 'project_unavailable';
        const reason = ['timeout', 'invalid_json', 'response_too_large'].includes(raw) ? raw :
          /^http_\d+$/.test(raw) ? 'http' : 'network';
        throw Object.assign(Error(code), code === 'project_unavailable' ? { identityReason: reason } : {});
      }
      if (!current()) throw Error('project_identity_changed');
      return response.payload;
    };
    const session = await read('/api/auth/session', { Accept: 'application/json' });
    const userId = session?.user?.id, accountId = session?.account?.id;
    if (session?.error || !validId(userId) || !validId(accountId) ||
        typeof session.accessToken !== 'string' || !/^\S{8,65536}$/.test(session.accessToken)) throw Error('project_auth_required');
    if (session.account.structure !== 'personal') throw unavailable('workspace');
    const headers = { Accept: 'application/json' };
    for (const [name, value] of Object.entries(page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders() || {})) {
      if (['oai-device-id', 'oai-language', 'oai-client-version', 'oai-client-build-number'].includes(name.toLowerCase())) headers[name] = String(value);
      // Never send into a different workspace while the page is switching accounts.
      if (name.toLowerCase() === 'chatgpt-account-id' && value && value !== accountId) throw Error('project_identity_changed');
    }
    headers.Authorization = 'Bearer ' + session.accessToken;
    headers['ChatGPT-Account-ID'] = accountId;
    const principal = JSON.stringify([userId, accountId]);
    // The authenticated cookie session supplies both identities and workspace type.
    // Re-check it per operation so logout cannot reuse a cached, still-valid bearer.
    const bytes = await page.crypto.subtle.digest('SHA-256', new TextEncoder().encode(principal));
    const accountScope = Array.from(new Uint8Array(bytes), b => b.toString(16).padStart(2, '0')).join('');
    if (!current()) throw Error('project_identity_changed');
    return Object.freeze({ accountScope, accountId, headers: Object.freeze(headers), current });
  }
  return Object.freeze({ version: 1, bind });
});
