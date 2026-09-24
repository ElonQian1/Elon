(function (page) {
  'use strict';
  if (page?.location?.origin !== 'https://chatgpt.com' || page.__elonConversationReader?.version === 3) return;
  const projection = page.__elonConversationProjection;
  const jobs = new Map(), snapshots = new Map();
  const ttl = 180000;
  const fail = code => { throw new Error(code); };
  const ownerKey = headers => JSON.stringify(Object.entries(headers || {}).filter(([key]) =>
    /authorization|account-id|workspace-id/i.test(key)).sort());
  const errorCode = error => ['auth_missing', 'auth_http_401'].includes(error?.message) ? 'login_required' :
    error?.message === 'auth_http_403' ? 'http_403' : ['invalid_conversation_id', 'conversation_mismatch', 'unsupported_conversation',
    'source_incomplete', 'source_limit', 'invalid_branch', 'branch_ambiguous', 'unsupported_message_content',
    'invalid_cursor', 'cursor_expired', 'identity_changed', 'login_required', 'reader_unavailable', 'request_conflict',
    'reader_busy', 'auth_cooldown', 'auth_unavailable', 'http_401', 'http_403', 'http_404', 'timeout', 'response_too_large'].includes(error?.message)
    ? error.message : 'conversation_read_failed';
  const transport = () => page.__elonChatGptPrivateTransport;
  const authContext = () => page.__elonChatGptPrivateAuthContext;
  function copiedHeaders() {
    return transport()?.copySameOriginRequestHeaders?.() || authContext()?.copyRequestHeaders?.();
  }
  async function identity() {
    if (!projection) fail('reader_unavailable');
    // A read does not require composer, voice, layout or the full semantic UI bridge.
    // Prefer observed account/workspace headers; otherwise use the existing page-local auth module.
    let headers = copiedHeaders();
    if (!headers && authContext()?.acquireRequestHeaders) headers = await authContext().acquireRequestHeaders();
    if (!headers && transport()?.acquireSameOriginRequestHeaders) headers = await transport().acquireSameOriginRequestHeaders();
    if (!headers && !authContext() && !transport()) fail('reader_unavailable');
    if (!headers || !Object.keys(headers).length) fail('login_required');
    // Retained only in the page closure, never included in native results or revisions.
    const owner = ownerKey(headers);
    if (owner === '[]') fail('login_required');
    return { headers, owner };
  }
  async function read(input) {
    const id = input.conversation_id;
    if (!projection?.ID.test(id)) fail('invalid_conversation_id');
    const auth = await identity();
    if (input.cursor) {
      const match = /^([a-f0-9]{32})\.(\d{1,6})$/.exec(input.cursor);
      if (!match) fail('invalid_cursor');
      const saved = snapshots.get(match[1]);
      if (!saved || saved.expires < Date.now()) fail('cursor_expired');
      if (saved.owner !== auth.owner || saved.snapshot.conversation_id !== id) fail('identity_changed');
      saved.expires = Date.now() + ttl;
      return { page: projection.page(saved.snapshot, match[1], Number(match[2])), owner: auth.owner };
    }
    const request = page.__elonChatGptPrivateJsonRequest;
    if (!request?.request) fail('reader_unavailable');
    const deadline = Date.now() + 30000, path = '/backend-api/conversations/' + encodeURIComponent(id);
    let bytes = 0;
    const get = async url => {
      if ((await identity()).owner !== auth.owner) fail('identity_changed');
      const remaining = deadline - Date.now();
      if (remaining <= 0) fail('timeout');
      const response = await request.request(page, url, {
        method: 'GET', credentials: 'include', cache: 'no-store', headers: { ...auth.headers, Accept: 'application/json' },
        __elonPrivateTransport: 'authorized_conversation_read',
      }, { timeoutMs: Math.min(18000, remaining), maxBytes: 4 * 1024 * 1024 });
      if ((await identity()).owner !== auth.owner) fail('identity_changed');
      bytes += JSON.stringify(response.payload).length;
      if (bytes > 16 * 1024 * 1024) fail('source_limit');
      return response.payload;
    };
    const value = projection.normalize(await get(path), id, true);
    let source = value;
    if (value.page_info != null) {
      let batch = projection.messagePage(value, id), rows = [...batch.rows];
      const cursors = new Set(), ids = new Set(rows.map(row => row.id));
      while (batch.cursor) {
        if (cursors.has(batch.cursor)) fail('source_incomplete');
        if (cursors.size >= 100) fail('source_limit');
        cursors.add(batch.cursor);
        batch = projection.messagePage(await get(path + '/messages?before=' + encodeURIComponent(batch.cursor) + '&include_has_versions=true'), id);
        for (const row of batch.rows) {
          if (ids.has(row.id)) fail('invalid_branch');
          ids.add(row.id);
        }
        if (ids.size > 20000) fail('source_limit');
        rows = [...batch.rows, ...rows];
      }
      if (cursors.size) {
        // Detect a changed default branch or edits while collecting older pages.
        const latest = projection.normalize(await get(path), id, true);
        const stamp = value => JSON.stringify([value.current_node, value.update_time, value.messages, value.page_info]);
        if (stamp(value) !== stamp(latest)) fail('source_incomplete');
      }
      source = { ...value, messages: rows, page_info: { has_previous_page: false } };
    }
    const snapshot = projection.project(source, id);
    const revision = [...page.crypto.getRandomValues(new Uint8Array(16))].map(v => v.toString(16).padStart(2, '0')).join('');
    if (snapshots.size >= 2) snapshots.delete(snapshots.keys().next().value);
    snapshots.set(revision, { snapshot, owner: auth.owner, expires: Date.now() + ttl });
    return { page: projection.page(snapshot, revision), owner: auth.owner };
  }
  function run(input) {
    for (const [key, value] of jobs) if (value.expires < Date.now()) jobs.delete(key);
    for (const [key, value] of snapshots) if (value.expires < Date.now()) snapshots.delete(key);
    if (!input || Object.keys(input).some(key => !['conversation_id', 'cursor', 'request_id'].includes(key)) ||
        !/^[a-zA-Z0-9_-]{8,80}$/.test(input.request_id || '') || !projection?.ID.test(input.conversation_id) ||
        typeof (input.cursor || '') !== 'string' || (input.cursor || '').length > 100) return { status: 'failed', error: 'invalid_request' };
    const signature = JSON.stringify([input.conversation_id, input.cursor || '']);
    const existing = jobs.get(input.request_id);
    if (existing) {
      if (existing.signature !== signature) return { status: 'failed', error: 'request_conflict' };
      if (existing.result.status === 'ready' && ownerKey(copiedHeaders()) !== existing.owner) {
        jobs.clear(); snapshots.clear();
        return { status: 'failed', error: 'identity_changed' };
      }
      return existing.result;
    }
    if (jobs.size >= 8) jobs.delete([...jobs].find(([, job]) => job.result.status !== 'pending')?.[0]);
    if (jobs.size >= 8) return { status: 'failed', error: 'reader_busy' };
    const job = { signature, expires: Date.now() + ttl, result: { status: 'pending', request_id: input.request_id } };
    jobs.set(input.request_id, job);
    read(input).then(result => { job.owner = result.owner; job.result = { status: 'ready', request_id: input.request_id, page: result.page }; })
      .catch(error => { job.result = { status: 'failed', request_id: input.request_id, error: errorCode(error) }; });
    return job.result;
  }
  page.__elonConversationReader = Object.freeze({ version: 3, run });
})(typeof window === 'object' ? window : null);
