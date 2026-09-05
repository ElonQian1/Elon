import { AssetAccessError } from './transport.js';

export const CLIENTS = Object.freeze(['quant.android', 'quant.web', 'quant.ai']);
export const SCOPES = Object.freeze(['esk.summary.read', 'esk.progress.read', 'profile.read']);
const I64_MAX = 9223372036854775807n;
const fail = () => { throw new AssetAccessError('invalid_response'); };
export function requireCondition(condition) { if (!condition) fail(); }
export function fields(value, required, optional = []) {
  requireCondition(value !== null && typeof value === 'object' && !Array.isArray(value));
  requireCondition(required.every(key => Object.hasOwn(value, key)) &&
    Object.keys(value).every(key => required.includes(key) || optional.includes(key)));
}
function text(value, max = 256) {
  requireCondition(typeof value === 'string' && value.length > 0 && value.length <= max &&
    !/[\u0000-\u001f\u007f]/u.test(value));
}
export function instant(value) {
  requireCondition(typeof value === 'string' && value.length <= 40 &&
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|\+00:00)$/u.test(value));
  const time = Date.parse(value);
  requireCondition(Number.isFinite(time) && new Date(time).toISOString().slice(0, 19) === value.slice(0, 19));
  return time;
}
export function scopes(value) {
  requireCondition(Array.isArray(value) && value.length >= 1 && value.length <= 3 &&
    value.includes('esk.summary.read') && value.every(scope => SCOPES.includes(scope)) &&
    new Set(value).size === value.length);
  return [...value];
}
export function sameScopes(actual, expected) {
  scopes(actual);
  requireCondition(actual.length === expected.length && actual.every(scope => expected.includes(scope)));
}
export function amount(value) {
  requireCondition(typeof value === 'string' && /^(0|[1-9][0-9]{0,18})$/u.test(value));
  const integer = BigInt(value);
  requireCondition(integer <= I64_MAX);
  return integer;
}
export function freezeCopy(value) {
  const clone = JSON.parse(JSON.stringify(value));
  const freeze = item => {
    if (item && typeof item === 'object') { Object.values(item).forEach(freeze); Object.freeze(item); }
    return item;
  };
  return freeze(clone);
}
export function authorizationCode(value, pending, now) {
  fields(value, ['schema', 'code', 'state', 'client_id', 'redirect_uri', 'code_expires_at', 'grant_id', 'expires_at', 'scopes']);
  requireCondition(value.schema === 'yilong.asset_access.authorization_code.v1' &&
    typeof value.code === 'string' && /^aac_[0-9a-f]{64}$/u.test(value.code) && value.state === pending.request.state &&
    value.client_id === pending.request.client_id && value.redirect_uri === pending.request.redirect_uri);
  text(value.grant_id);
  sameScopes(value.scopes, pending.request.scopes);
  const expires = instant(value.expires_at);
  requireCondition(instant(value.code_expires_at) > now && expires > now &&
    expires <= now + pending.request.expires_in * 1000 && instant(value.code_expires_at) <= expires);
  return value;
}
export function tokenResponse(value, code, now) {
  fields(value, ['schema', 'access_token', 'token_type', 'audience', 'subject', 'client_id', 'grant_id', 'expires_in', 'expires_at', 'scopes']);
  requireCondition(value.schema === 'yilong.asset_access.token.v1' && value.token_type === 'Bearer' &&
    value.audience === 'yilong-quant' && typeof value.access_token === 'string' && /^aat_[0-9a-f]{64}$/u.test(value.access_token) &&
    value.client_id === code.client_id && value.grant_id === code.grant_id &&
    Number.isInteger(value.expires_in) && value.expires_in >= 1 && value.expires_in <= 3600);
  text(value.subject);
  sameScopes(value.scopes, code.scopes);
  const expires = instant(value.expires_at);
  requireCondition(expires > now && expires === instant(code.expires_at));
  return freezeCopy(value);
}
function subjectBinding(value, token, now) {
  if (value.subject !== token.subject || value.client_id !== token.client_id) {
    throw new AssetAccessError('subject_changed');
  }
  const expires = instant(value.expires_at);
  requireCondition(expires <= instant(token.expires_at));
  if (expires <= now) throw new AssetAccessError('expired');
}
export function identityResponse(value, token, now) {
  fields(value, ['schema', 'audience', 'subject', 'client_id', 'grant_id', 'expires_at', 'scopes'], ['nickname']);
  requireCondition(value.schema === 'yilong.asset_access.identity.v1' && value.audience === 'yilong-quant' &&
    value.grant_id === token.grant_id);
  subjectBinding(value, token, now);
  sameScopes(value.scopes, token.scopes);
  const profile = token.scopes.includes('profile.read');
  requireCondition(Object.hasOwn(value, 'nickname') === profile);
  if (profile) requireCondition(typeof value.nickname === 'string' && value.nickname.length <= 256);
  return freezeCopy(value);
}
function requestEntry(value) {
  fields(value, ['request_id', 'amount_base_units', 'status', 'created_at', 'canceled_at']);
  text(value.request_id);
  requireCondition(amount(value.amount_base_units) > 0n);
  const created = instant(value.created_at);
  requireCondition(['submitted', 'canceled'].includes(value.status));
  if (value.status === 'submitted') requireCondition(value.canceled_at === null);
  else requireCondition(instant(value.canceled_at) >= created);
}
function progressPage(value, limit) {
  fields(value, ['request_count', 'open_count', 'range_start', 'range_end', 'requests', 'has_more', 'next_cursor']);
  const count = amount(value.request_count), open = amount(value.open_count);
  const start = amount(value.range_start), end = amount(value.range_end);
  requireCondition(open <= count && Array.isArray(value.requests) && value.requests.length <= limit);
  value.requests.forEach(requestEntry);
  requireCondition(new Set(value.requests.map(entry => entry.request_id)).size === value.requests.length);
  requireCondition(value.requests.length === 0 ? count === 0n && start === 0n && end === 0n :
    start >= 1n && end === start + BigInt(value.requests.length) - 1n && end <= count);
  requireCondition(typeof value.has_more === 'boolean' && value.has_more === (end < count));
  if (value.has_more) text(value.next_cursor, 160);
  else requireCondition(value.next_cursor === null);
}
export function assetResponse(value, token, { now, limit, includeProgress, previous, cursor }) {
  fields(value, ['schema', 'subject', 'client_id', 'expires_at', 'asset', 'balance', 'snapshot_digest'], ['progress']);
  requireCondition(value.schema === 'yilong.esk.delegated_asset_page.v1');
  subjectBinding(value, token, now);
  const expected = { asset_id: 'esk', symbol: 'ESK', decimals: 6, source: 'platform_recorded',
    simulated: false, chain_status: 'not_deployed', funds_moved: false };
  fields(value.asset, Object.keys(expected));
  requireCondition(Object.entries(expected).every(([key, expectedValue]) => value.asset[key] === expectedValue));
  fields(value.balance, ['total_base_units', 'reserved_base_units', 'available_base_units']);
  const total = amount(value.balance.total_base_units), reserved = amount(value.balance.reserved_base_units);
  const available = amount(value.balance.available_base_units);
  requireCondition(total === reserved + available && typeof value.snapshot_digest === 'string' && /^[0-9a-f]{64}$/u.test(value.snapshot_digest));
  requireCondition(Object.hasOwn(value, 'progress') === includeProgress);
  if (includeProgress) progressPage(value.progress, limit);
  if (cursor) {
    requireCondition(previous !== null && cursor === previous.progress?.next_cursor);
    if (value.snapshot_digest !== previous.snapshot_digest) throw new AssetAccessError('snapshot_changed', 409);
    requireCondition(Object.keys(value.balance).every(key => value.balance[key] === previous.balance[key]) &&
      value.progress.request_count === previous.progress.request_count &&
      value.progress.open_count === previous.progress.open_count &&
      amount(value.progress.range_start) === amount(previous.progress.range_end) + 1n);
  } else if (includeProgress) requireCondition(amount(value.progress.range_start) <= 1n);
  return freezeCopy(value);
}
export function revokedResponse(value) {
  fields(value, ['schema', 'revoked', 'funds_moved']);
  requireCondition(value.schema === 'yilong.asset_access.revoked.v1' && value.revoked === true && value.funds_moved === false);
  return freezeCopy(value);
}
