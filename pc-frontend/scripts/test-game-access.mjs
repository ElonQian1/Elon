import test from 'node:test'
import assert from 'node:assert/strict'
import { parseGameRequest, gameLoginReturn, authorizedCallback, scopeLabels } from '../src/features/game-access/contract.ts'
import { loginSession, readJson } from '../src/features/game-access/session.ts'

const request = () => ({ schema: 'esk.game.access.authorize.v1', client_id: 'esk-game.web',
  redirect_uri: 'https://game.example.test/api/account/callback', state: 's'.repeat(64),
  code_challenge: 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM', code_challenge_method: 'S256',
  scopes: ['play', 'inventory_read'], expires_in_seconds: 900 })
const query = value => `?${new URLSearchParams({ request: JSON.stringify(value) })}`

test('valid game request preserves the exact scope, state, PKCE and callback contract', () => {
  assert.deepEqual(parseGameRequest(query(request())), request())
  assert.equal(gameLoginReturn(`/game-access${query(request())}`), `/game-access${query(request())}`)
})

test('wallet binding is separately displayed and only accepted when explicitly requested', () => {
  const requested = { ...request(), scopes: ['play', 'inventory_read', 'wallet_bind'] }
  assert.deepEqual(parseGameRequest(query(requested)), requested)
  assert.match(scopeLabels.wallet_bind, /确认.*签名.*绑定/)
  assert.deepEqual(parseGameRequest(query(request())).scopes, ['play', 'inventory_read'])
  assert.throws(() => parseGameRequest(query({ ...requested, scopes: ['play', 'wallet_bind', 'inventory_read'] })))
})
test('login return cannot become an external or arbitrary in-app redirect', () => {
  for (const value of [null, '', '//evil.example', 'https://evil.example', '/account', '/game-access',
    '/game-access?request={}', `/game-access${query(request())}&next=https://evil.example`]) {
    assert.equal(gameLoginReturn(value), null)
  }
})
test('unsafe callback addresses, newline aliases and caller-supplied consent are rejected', () => {
  for (const redirect_uri of ['http://game.example.test/api/account/callback', 'https://game.example.test/api/account/callback?x=1',
    'https://u@game.example.test/api/account/callback', 'https://GAME.example.test/api/account/callback',
    'https://game.example.test:443/api/account/callback', 'https://game.example.test/api/account/callback#x']) {
    assert.throws(() => parseGameRequest(query({ ...request(), redirect_uri })))
  }
  for (const extra of [{ state: `${'s'.repeat(64)}\n` }, { code_challenge: `${'s'.repeat(42)}\n` },
    { code_challenge: 'B'.repeat(43) }, { explicit_consent: true }, { expires_in_seconds: 901 }]) {
    assert.throws(() => parseGameRequest(query({ ...request(), ...extra })))
  }
})
test('unknown, reordered or duplicated scopes and queries cannot grant additional authority', () => {
  for (const scopes of [[], ['redeem'], ['play', 'admin'], ['play', 'play'], ['play', 'principal_withdraw', 'redeem']]) {
    assert.throws(() => parseGameRequest(query({ ...request(), scopes })))
  }
  assert.throws(() => parseGameRequest(`${query(request())}&request={}`))
  assert.throws(() => parseGameRequest(`${query(request())}&debug=1`))
})
test('authorized redirect accepts only a matching fresh response and contains no main token', () => {
  const now = 1700000000000
  const reply = { schema: 'esk.game.access.code.v1', redirect_uri: request().redirect_uri,
    state: request().state, code: `egc_${'42'.repeat(32)}`, scopes: request().scopes, code_expires_at_ms: String(now + 120000) }
  const target = new URL(authorizedCallback(reply, request(), now))
  assert.equal(target.origin, 'https://game.example.test')
  assert.deepEqual([...target.searchParams.keys()], ['code', 'state'])
  for (const extra of [{ state: 'wrong' }, { redirect_uri: 'https://evil.example' }, { scopes: ['play', 'redeem'] },
    { code: `${reply.code}\n` }, { code_expires_at_ms: String(now) }, { code_expires_at_ms: String(now + 123000) }]) {
    assert.throws(() => authorizedCallback({ ...reply, ...extra }, request(), now))
  }
})

test('main login accepts an original session and rejects expired or virtual identities', () => {
  const now = 1700000000000
  const session = { token: '42'.repeat(32), expires_at: new Date(now + 60000).toISOString(),
    user: { id: 'original-main-user', account: 'holder@example.test', nickname: 'Holder' } }
  assert.deepEqual(loginSession(session, now), session)
  for (const extra of [{ token: 'short' }, { token: `${session.token}\n` }, { expires_at: 'invalid' },
    { expires_at: new Date(now).toISOString() }, { user: { ...session.user, id: 'local-owner' } },
    { user: { ...session.user, nickname: {} } }]) assert.throws(() => loginSession({ ...session, ...extra }, now))
})

test('account responses are bounded before buffering and reject HTML and invalid UTF-8', async () => {
  const json = value => new Response(value, { headers: { 'content-type': 'application/json; charset=utf-8' } })
  assert.deepEqual(await readJson(json('{"ok":true}')), { ok: true })
  await assert.rejects(readJson(new Response('<html>login</html>', { headers: { 'content-type': 'text/html' } })))
  await assert.rejects(readJson(json(new Uint8Array([0xff]))))
  let cancelled = false
  const stream = new ReadableStream({ pull(controller) { controller.enqueue(new Uint8Array(16385)) }, cancel() { cancelled = true } })
  await assert.rejects(readJson(json(stream)))
  assert.equal(cancelled, true)
})
