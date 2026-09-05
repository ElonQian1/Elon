import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { createAssetAccessClient } from '../src/client.js';

const fixture = JSON.parse(await readFile(new URL('../../../contracts/assets/asset-access-v1.fixture.json', import.meta.url)));
const clone = value => structuredClone(value);
const reply = (value, status = 200) => new Response(JSON.stringify(value), { status,
  headers: { 'content-type': 'application/json' } });
const codeValue = `aac_${'1'.repeat(64)}`;

function environment(options = {}) {
  let time = Date.parse(fixture.metadata.clock);
  const calls = [], queue = [];
  const client = createAssetAccessClient({ baseUrl: 'https://assets.example', clientId: 'quant.web',
    clock: () => time, fetch: async (url, init) => {
      calls.push({ url, init });
      assert.equal(init.redirect, 'error'); assert.equal(init.credentials, 'omit');
      assert.equal(init.cache, 'no-store'); assert.equal(init.referrerPolicy, 'no-referrer');
      const next = queue.shift();
      assert.ok(next, 'unexpected HTTP call');
      return typeof next === 'function' ? next(url, init) : next;
    }, ...options });
  return { client, calls, queue, advance: milliseconds => { time += milliseconds; } };
}

async function authorize(env, selected = fixture.valid.token.scopes, tokenPatch = {}) {
  const request = await env.client.authorizationRequest({ redirectUri: 'https://assets.example/quant/asset-access/callback',
    scopes: selected, explicitConsent: true });
  const response = { schema: 'yilong.asset_access.authorization_code.v1', code: codeValue,
    state: request.state, client_id: request.client_id, redirect_uri: request.redirect_uri,
    code_expires_at: '2030-01-01T00:01:00Z', grant_id: fixture.valid.token.grant_id,
    expires_at: '2030-01-01T00:15:00Z', scopes: selected };
  env.queue.push(reply({ ...clone(fixture.valid.token), scopes: selected, ...tokenPatch }));
  const result = await env.client.exchangeCode(response);
  const posted = JSON.parse(env.calls.at(-1).init.body);
  assert.equal(createHash('sha256').update(posted.code_verifier).digest('base64url'), request.code_challenge);
  assert.equal(posted.state, request.state);
  assert.equal(env.calls.at(-1).init.headers.Authorization, undefined);
  assert.equal(result.status, 'authorized');
  assert.ok(!JSON.stringify(result).includes(fixture.valid.token.access_token));
  return { request, response, posted };
}

test('PKCE exchange, identity, paginated assets and revoke use one consent and isolated headers', async () => {
  const env = environment();
  const { request, response, posted } = await authorize(env);
  const serialized = JSON.stringify(env.client);
  for (const secret of [fixture.valid.token.access_token, codeValue, posted.code_verifier, request.state]) {
    assert.ok(!serialized.includes(secret));
  }
  assert.deepEqual(Object.keys(env.client), []);
  await assert.rejects(env.client.exchangeCode(response), { code: 'authorization_required' });
  env.queue.push(reply(fixture.valid.identity), reply(fixture.valid.asset_first),
    reply(fixture.valid.asset_second), reply(fixture.valid.revoked));
  assert.equal((await env.client.identity()).nickname, 'Synthetic example');
  const first = await env.client.readAssets({ limit: 1 });
  assert.equal(first.page.balance.available_base_units, '9007199254740993');
  assert.equal(first.restarted, false);
  const second = await env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor });
  assert.equal(second.page.progress.range_start, '2');
  assert.equal(second.page.progress.requests[0].status, 'canceled');
  assert.deepEqual(await env.client.revoke(), fixture.valid.revoked);
  assert.equal(env.client.state.status, 'unauthenticated');
  assert.equal(env.client.state.has_snapshot, false);
  for (const call of env.calls.slice(1)) {
    assert.equal(call.init.headers.Authorization, `Bearer ${fixture.valid.token.access_token}`);
    assert.equal(call.init.headers['X-Elon-Asset-Client'], 'quant.web');
    assert.ok(!call.url.includes(fixture.valid.token.access_token));
  }
  assert.equal(env.calls.length, 5);
});

test('summary-only scope excludes progress, profile and unrelated routes', async () => {
  const env = environment(); await authorize(env, ['esk.summary.read']);
  const identity = { ...fixture.valid.identity, scopes: ['esk.summary.read'] }; delete identity.nickname;
  env.queue.push(reply(identity), reply(fixture.valid.asset_summary));
  assert.equal((await env.client.identity()).nickname, undefined);
  assert.equal((await env.client.readAssets()).page.balance.total_base_units, '9223372036854775807');
  assert.equal(new URL(env.calls.at(-1).url).searchParams.get('include_progress'), 'false');
  await assert.rejects(env.client.readAssets({ includeProgress: true }), { code: 'invalid_query' });
  assert.equal(env.client.request, undefined);
});

test('valid zero balance is preserved as a successful page', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_zero));
  assert.equal((await env.client.readAssets()).page.balance.total_base_units, '0');
  assert.equal(env.client.state.status, 'authorized');
});

for (const invalid of fixture.invalid_assets) {
  test(`rejects shared vector ${invalid.name} and clears credentials and prior data`, async () => {
    const env = environment(); await authorize(env);
    env.queue.push(reply(fixture.valid.asset_first)); await env.client.readAssets({ limit: 1 });
    const data = clone(fixture.valid.asset_first);
    let target = data;
    for (const key of invalid.path.slice(0, -1)) target = target[key];
    target[invalid.path.at(-1)] = invalid.value;
    env.queue.push(reply(data));
    await assert.rejects(env.client.readAssets({ limit: 1 }));
    assert.equal(env.client.state.status, 'unauthenticated');
    assert.equal(env.client.state.has_snapshot, false);
    await assert.rejects(env.client.identity(), { code: 'authorization_required' });
  });
}

test('snapshot_changed clears the old page and retries the first page exactly once', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_first));
  const first = await env.client.readAssets({ limit: 1 });
  env.queue.push(reply({ code: 'asset_access_snapshot_changed' }, 409), reply(fixture.valid.asset_zero));
  const result = await env.client.readAssets({ cursor: first.page.progress.next_cursor, limit: 1 });
  assert.equal(result.restarted, true); assert.equal(result.page.progress.request_count, '0');
  assert.equal(new URL(env.calls.at(-1).url).searchParams.has('cursor'), false);
  assert.equal(env.client.state.status, 'authorized');
});

test('even a 200 response with a new snapshot forces first-page restart', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_first));
  const first = await env.client.readAssets({ limit: 1 });
  env.queue.push(reply({ ...fixture.valid.asset_second, snapshot_digest: 'd'.repeat(64) }), reply(fixture.valid.asset_zero));
  assert.equal((await env.client.readAssets({ cursor: first.page.progress.next_cursor, limit: 1 })).restarted, true);
});

for (const status of [401, 403, 500]) {
  test(`HTTP ${status} clears token and snapshot without exposing error text`, async () => {
    const env = environment(); await authorize(env);
    env.queue.push(reply(fixture.valid.asset_zero)); await env.client.readAssets();
    env.queue.push(reply({ code: 'attacker-value', error: fixture.valid.token.access_token }, status));
    await assert.rejects(env.client.identity(), error => !error.message.includes('aat_'));
    assert.equal(env.client.state.status, 'unauthenticated');
    assert.equal(env.client.state.has_snapshot, false);
  });
}

test('expiration prevents any new HTTP read and wipes data', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_zero)); await env.client.readAssets();
  const before = env.calls.length;
  env.advance(900000);
  await assert.rejects(env.client.identity(), { code: 'expired' });
  assert.equal(env.calls.length, before); assert.equal(env.client.state.has_snapshot, false);
});

test('wrong transaction binding fails before exchange and consumes the local transaction', async () => {
  for (const field of ['state', 'client_id', 'redirect_uri']) {
    const env = environment();
    const request = await env.client.authorizationRequest({ redirectUri: 'https://assets.example/quant/asset-access/callback', explicitConsent: true });
    const response = { schema: 'yilong.asset_access.authorization_code.v1', code: codeValue,
      state: request.state, client_id: 'quant.web', redirect_uri: request.redirect_uri,
      code_expires_at: '2030-01-01T00:01:00Z', grant_id: fixture.valid.token.grant_id,
      expires_at: '2030-01-01T00:15:00Z', scopes: request.scopes, [field]: 'incorrect-binding' };
    await assert.rejects(env.client.exchangeCode(response), { code: 'invalid_response' });
    assert.equal(env.calls.length, 0); assert.equal(env.client.state.status, 'unauthenticated');
  }
});

test('unsafe base URLs and callbacks fail before any request', async () => {
  for (const baseUrl of ['http://public.example', 'http://127.0.0.1:1234',
    'https://user:password@assets.example', 'https://assets.example/#fragment',
    'https://assets.example/?token=secret', 'https://assets.example/path', 'https://assets.example\\evil']) {
    assert.throws(() => environment({ baseUrl }));
  }
  assert.doesNotThrow(() => environment({ baseUrl: 'http://127.0.0.1:1234', allowLoopbackHttp: true }));
  assert.throws(() => environment({ baseUrl: 'http://localhost:1234', allowLoopbackHttp: true }));
  const env = environment();
  for (const redirectUri of ['http://public.example/callback', 'https://user:password@web.example',
    'https://assets.example/quant/asset-access/callback#fragment', 'https://assets.example/quant/asset-access/callback?next=evil']) {
    await assert.rejects(env.client.authorizationRequest({ redirectUri, explicitConsent: true }));
  }
  await assert.rejects(env.client.authorizationRequest({ redirectUri: 'https://web.example' }), { code: 'consent_required' });
  assert.equal(env.calls.length, 0);
});

test('redirects and bounded response failures clear the credential', async () => {
  for (const response of [new Response(null, { status: 302, headers: { location: 'https://evil.example' } }),
    new Response('x'.repeat(140000), { headers: { 'content-type': 'application/json' } }),
    new Response('<html>no</html>', { headers: { 'content-type': 'text/html' } })]) {
    const env = environment(); await authorize(env); env.queue.push(response);
    await assert.rejects(env.client.identity()); assert.equal(env.client.state.status, 'unauthenticated');
  }
});

test('timeouts include fetch and streamed response reading', async () => {
  for (const makeResponse of [() => new Promise(() => {}), () => new Response(new ReadableStream({
    start(controller) { controller.enqueue(new TextEncoder().encode('{')); }
  }), { headers: { 'content-type': 'application/json' } })]) {
    const env = environment({ timeoutMs: 25 }); await authorize(env); env.queue.push(makeResponse);
    await assert.rejects(env.client.identity(), { code: 'timeout' });
    assert.equal(env.client.state.status, 'unauthenticated');
  }
});

test('clear during a pending read cannot resurrect a session or old snapshot', async () => {
  const env = environment(); await authorize(env);
  let resolve;
  env.queue.push(() => new Promise(done => { resolve = done; }));
  const pending = env.client.readAssets();
  env.client.clear(); resolve(reply(fixture.valid.asset_zero));
  await assert.rejects(pending, { code: 'cleared' });
  assert.equal(env.client.state.status, 'unauthenticated'); assert.equal(env.client.state.has_snapshot, false);
});

test('malformed tokens, added scopes and mismatched expirations cannot establish a session', async () => {
  for (const patch of [{ access_token: [fixture.valid.token.access_token] }, { token_type: 'bearer' },
    { audience: 'administrator' }, { client_id: 'quant.ai' }, { grant_id: 'another-grant' },
    { expires_at: '2030-01-01T01:15:00Z' }, { expires_in: 3601 },
    { scopes: ['esk.summary.read', 'trade.write'] }, { refresh_token: 'not-supported' }]) {
    const env = environment();
    await assert.rejects(authorize(env, fixture.valid.token.scopes, patch));
    assert.equal(env.client.state.status, 'unauthenticated');
  }
});

test('profile leakage, changing identity and malformed digest types fail closed', async () => {
  for (const patch of [{ subject: 'different-subject' }, { user_id: 'private-main-id' },
    { scopes: ['esk.summary.read'] }, { expires_at: '2030-02-30T00:15:00Z' }]) {
    const env = environment(); await authorize(env);
    env.queue.push(reply({ ...fixture.valid.identity, ...patch }));
    await assert.rejects(env.client.identity()); assert.equal(env.client.state.status, 'unauthenticated');
  }
  const env = environment(); await authorize(env);
  env.queue.push(reply({ ...fixture.valid.asset_zero, snapshot_digest: ['b'.repeat(64)] }));
  await assert.rejects(env.client.readAssets()); assert.equal(env.client.state.status, 'unauthenticated');
});

test('repeated snapshot failures do not loop or retain old data', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_first));
  const first = await env.client.readAssets({ limit: 1 });
  env.queue.push(reply({ code: 'asset_access_snapshot_changed' }, 409),
    reply({ code: 'asset_access_snapshot_changed' }, 409));
  await assert.rejects(env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor }), { code: 'snapshot_changed' });
  assert.equal(env.calls.length, 4); assert.equal(env.client.state.has_snapshot, false);
  assert.equal(env.client.state.status, 'unauthenticated');
});

test('canceling a stalled stream releases its reader and cannot clear a later session', async () => {
  const env = environment(); await authorize(env);
  let canceled = false;
  env.queue.push(new Response(new ReadableStream({ cancel() { canceled = true; } }),
    { headers: { 'content-type': 'application/json' } }));
  const pending = env.client.identity();
  const rejected = assert.rejects(pending, { code: 'cleared' });
  await new Promise(resolve => setImmediate(resolve));
  env.client.clear();
  await authorize(env);
  await rejected;
  assert.equal(canceled, true); assert.equal(env.client.state.status, 'authorized');
});

test('failed server revocation still clears locally and does not report revoked', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply({ code: 'unavailable' }, 503));
  await assert.rejects(env.client.revoke(), { code: 'request_failed' });
  assert.equal(env.client.state.status, 'unauthenticated');
});

test('concurrent reads are refused without disturbing the in-flight read', async () => {
  const env = environment(); await authorize(env);
  let resolve;
  env.queue.push(() => new Promise(done => { resolve = done; }));
  const reading = env.client.readAssets();
  await assert.rejects(env.client.identity(), { code: 'request_in_progress' });
  resolve(reply(fixture.valid.asset_zero));
  assert.equal((await reading).page.balance.total_base_units, '0');
  assert.equal(env.client.state.status, 'authorized');
});

test('scope order is a set and every authorization gets fresh PKCE material', async () => {
  const env = environment();
  const old = await env.client.authorizationRequest({ redirectUri: 'https://assets.example/quant/asset-access/callback', explicitConsent: true });
  const { request } = await authorize(env, fixture.valid.token.scopes,
    { scopes: [...fixture.valid.token.scopes].reverse() });
  assert.notEqual(old.state, request.state); assert.notEqual(old.code_challenge, request.code_challenge);
  env.queue.push(reply(fixture.valid.identity));
  assert.equal((await env.client.identity()).subject, fixture.valid.identity.subject);
});

test('a slow approval can still receive a grant bounded from issuance, never extend it', async () => {
  const env = environment();
  const request = await env.client.authorizationRequest({ redirectUri: 'https://assets.example/quant/asset-access/callback', explicitConsent: true });
  env.advance(120000);
  const expires = '2030-01-01T00:17:00Z';
  env.queue.push(reply({ ...fixture.valid.token, scopes: request.scopes, expires_at: expires }));
  await env.client.exchangeCode({ schema: 'yilong.asset_access.authorization_code.v1',
    code: codeValue, state: request.state, client_id: request.client_id, redirect_uri: request.redirect_uri,
    code_expires_at: '2030-01-01T00:04:00Z', grant_id: fixture.valid.token.grant_id, expires_at: expires, scopes: request.scopes });
  assert.equal(Date.parse(env.client.state.expires_at), Date.parse(expires));
});

test('response limit applies to UTF-8 bytes, not JavaScript character count', async () => {
  const env = environment({ maxResponseBytes: 1024 }); await authorize(env);
  env.queue.push(new Response(JSON.stringify({ nickname: '中'.repeat(400) }),
    { headers: { 'content-type': 'application/json' } }));
  await assert.rejects(env.client.identity(), { code: 'response_too_large' });
  assert.equal(env.client.state.status, 'unauthenticated');
});

test('oversized declared bodies are canceled without consuming the stream', async () => {
  const env = environment(); await authorize(env);
  let canceled = false;
  env.queue.push(new Response(new ReadableStream({ cancel() { canceled = true; } }),
    { headers: { 'content-type': 'application/json', 'content-length': '999999999' } }));
  await assert.rejects(env.client.identity(), { code: 'response_too_large' });
  assert.equal(canceled, true); assert.equal(env.client.state.status, 'unauthenticated');
});

test('unregistered callback spellings fail rather than being silently canonicalized', async () => {
  const env = environment();
  await assert.rejects(env.client.authorizationRequest({
    redirectUri: 'https://assets.example:443/quant/asset-access/callback', explicitConsent: true,
  }), { code: 'invalid_redirect' });
  const redirectUri = 'https://assets.example/quant/asset-access/callback';
  const request = await env.client.authorizationRequest({ redirectUri, explicitConsent: true });
  assert.equal(request.redirect_uri, redirectUri);
});

test('registered AI callback is allowed in production without permitting plaintext API traffic', async () => {
  const env = environment({ clientId: 'quant.ai' });
  const redirectUri = 'http://127.0.0.1:8765/asset-access/callback';
  const request = await env.client.authorizationRequest({ redirectUri, explicitConsent: true });
  assert.equal(request.client_id, 'quant.ai'); assert.equal(request.redirect_uri, redirectUri);
  assert.equal(env.calls.length, 0);
  for (const invalid of ['http://127.0.0.1:1023/asset-access/callback',
    'http://localhost:8765/asset-access/callback', 'http://[::1]:8765/asset-access/callback',
    'http://127.0.0.1:8765/wrong-path', 'https://assets.example/asset-access/callback']) {
    await assert.rejects(env.client.authorizationRequest({ redirectUri: invalid, explicitConsent: true }));
  }
  assert.throws(() => environment({ clientId: 'quant.ai', baseUrl: 'http://assets.example' }));
});

test('native authorization matches the main Android 15-minute fixed-scope contract', async () => {
  const env = environment({ clientId: 'quant.android' });
  const request = await env.client.authorizationRequest({
    redirectUri: 'com.elon.quant:/asset-access/callback',
    scopes: ['esk.summary.read', 'esk.progress.read'], expiresIn: 900, explicitConsent: true,
  });
  assert.equal(request.client_id, 'quant.android'); assert.equal(request.expires_in, 900);
  assert.deepEqual(request.scopes, ['esk.summary.read', 'esk.progress.read']);
  await assert.rejects(env.client.authorizationRequest({
    redirectUri: 'com.elon.quant:/another-callback', explicitConsent: true,
  }), { code: 'invalid_redirect' });
});

test('HTTP query ceilings are rejected locally without consuming a valid grant', async () => {
  const env = environment(); await authorize(env);
  const calls = env.calls.length;
  await assert.rejects(env.client.readAssets({ limit: 21 }), { code: 'invalid_query' });
  await assert.rejects(env.client.readAssets({ cursor: 'x'.repeat(161) }), { code: 'invalid_query' });
  assert.equal(env.calls.length, calls); assert.equal(env.client.state.status, 'authorized');
});

test('identity may shorten the grant deadline and clears any previously cached page', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_zero)); await env.client.readAssets();
  env.queue.push(reply({ ...fixture.valid.identity, expires_at: '2030-01-01T00:10:00Z' }));
  assert.equal((await env.client.identity()).expires_at, '2030-01-01T00:10:00Z');
  assert.equal(env.client.state.has_snapshot, false);
  assert.equal(Date.parse(env.client.state.expires_at), Date.parse('2030-01-01T00:10:00Z'));
  env.advance(600000);
  await assert.rejects(env.client.readAssets(), { code: 'expired' });
});

test('a shortened asset deadline restarts pagination and cannot be widened by a later response', async () => {
  const env = environment(); await authorize(env);
  env.queue.push(reply(fixture.valid.asset_first));
  const first = await env.client.readAssets({ limit: 1 });
  const shorter = '2030-01-01T00:10:00Z';
  env.queue.push(reply({ ...fixture.valid.asset_second, expires_at: shorter }),
    reply({ ...fixture.valid.asset_first, expires_at: shorter }));
  const result = await env.client.readAssets({ limit: 1, cursor: first.page.progress.next_cursor });
  assert.equal(result.restarted, true); assert.equal(result.page.progress.range_start, '1');
  assert.equal(Date.parse(env.client.state.expires_at), Date.parse(shorter));
  env.queue.push(reply(fixture.valid.identity));
  await assert.rejects(env.client.identity(), { code: 'invalid_response' });
  assert.equal(env.client.state.status, 'unauthenticated');
});
