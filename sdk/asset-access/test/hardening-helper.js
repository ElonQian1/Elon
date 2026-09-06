import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createAssetAccessClient } from '../src/client.js';

export const fixture = JSON.parse(await readFile(new URL(
  '../../../contracts/assets/asset-access-v1.fixture.json', import.meta.url)));
export const clone = value => structuredClone(value);
export const reply = (value, status = 200) => new Response(JSON.stringify(value), {
  status, headers: { 'content-type': 'application/json' },
});
export const rawReply = bytes => new Response(bytes, { headers: { 'content-type': 'application/json' } });

export function environment(options = {}) {
  const calls = [], queue = [];
  let now = Date.parse(fixture.metadata.clock);
  const client = createAssetAccessClient({
    baseUrl: 'https://assets.example', clientId: 'quant.web', clock: () => now,
    fetch: async (url, init) => {
      calls.push({ url, init });
      assert.equal(init.redirect, 'error');
      assert.equal(init.credentials, 'omit');
      const item = queue.shift();
      assert.ok(item, 'unexpected synthetic HTTP call');
      return typeof item === 'function' ? item(url, init) : item;
    }, ...options,
  });
  return { client, calls, queue, advance: amount => { now += amount; } };
}

export async function authorize(env, tokenPatch = {}, tokenReply = null) {
  const request = await env.client.authorizationRequest({
    redirectUri: 'https://assets.example/quant/asset-access/callback',
    scopes: fixture.valid.token.scopes, explicitConsent: true,
  });
  const token = { ...clone(fixture.valid.token), ...tokenPatch };
  env.queue.push(tokenReply ?? reply(token));
  await env.client.exchangeCode({
    schema: 'yilong.asset_access.authorization_code.v1', code: `aac_${'1'.repeat(64)}`,
    state: request.state, client_id: request.client_id, redirect_uri: request.redirect_uri,
    code_expires_at: '2030-01-01T00:01:00Z', grant_id: token.grant_id,
    expires_at: token.expires_at, scopes: request.scopes,
  });
}
