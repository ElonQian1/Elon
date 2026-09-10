import { createHash } from 'node:crypto';

export const CHALLENGE_DOMAIN = 'esk.game.session.challenge.v1';
export const OBSERVATION_DOMAIN = 'esk.game.session.observation.v1';
export const MAX_GRANT_MS = 900_000n;
export const MAX_OBSERVATION_AGE_MS = 10_000n;
export const CLOCK_SKEW_MS = 2_000n;
const SCOPE_ORDER = ['play', 'inventory_read', 'redeem', 'principal_withdraw'];
const ACTIONS = {
  authenticate: [[], 'play'], inventory: [[], 'inventory_read'],
  order: [['order_id'], 'inventory_read'], quote: [['asset_id', 'policy_id'], 'redeem'],
  accept_quote: [['quote_id', 'idempotency_key'], 'redeem'],
  principal_withdraw: [['position_id', 'idempotency_key'], 'principal_withdraw'],
};

export class GameAccessError extends Error {
  constructor(code) { super(code); this.name = 'GameAccessError'; this.code = code; }
}
export function requireCondition(condition, code = 'invalid_contract') {
  if (!condition) throw new GameAccessError(code);
}
export function exactObject(value, fields) {
  requireCondition(value !== null && typeof value === 'object' && !Array.isArray(value));
  const keys = Object.keys(value);
  requireCondition(keys.length === fields.length && keys.every(key => fields.includes(key)));
}
export function identifier(value) {
  requireCondition(typeof value === 'string' && value.length >= 1 && value.length <= 128
    && !/[^A-Za-z0-9_.:-]/.test(value));
  return value;
}
export function hex(value, bytes) {
  requireCondition(typeof value === 'string' && value.length > 0 && value.length === bytes * 2
    && !/[^0-9a-f]/.test(value));
  return value;
}
export function units(value) {
  requireCondition(typeof value === 'string' && value.length >= 1 && value.length <= 19
    && !/[^0-9]/.test(value) && (value.length === 1 || value[0] !== '0'));
  const integer = BigInt(value);
  requireCondition(integer <= 9_223_372_036_854_775_807n);
  return integer;
}
export function canonical(values) { return Buffer.from(JSON.stringify(values), 'utf8'); }
export function digest(bytes) { return createHash('sha256').update(bytes).digest('hex'); }

export function actionContract(action) {
  requireCondition(action !== null && typeof action === 'object');
  requireCondition(typeof action.kind === 'string' && Object.hasOwn(ACTIONS, action.kind));
  const [fields, scope] = ACTIONS[action.kind];
  exactObject(action, ['kind', ...fields]);
  return { values: [action.kind, ...fields.map(field => identifier(action[field]))], scope };
}
export function challengeValues(challenge) {
  exactObject(challenge, ['domain', 'main_issuer', 'audience', 'stage', 'nonce', 'credential_digest', 'action']);
  requireCondition(challenge.domain === CHALLENGE_DOMAIN && challenge.audience === 'esk-game'
    && challenge.stage === 'platform_recorded');
  return [challenge.domain, identifier(challenge.main_issuer), challenge.audience, challenge.stage,
    hex(challenge.nonce, 32), hex(challenge.credential_digest, 32), digest(canonical(actionContract(challenge.action).values))];
}
export function grantValues(grant) {
  exactObject(grant, ['main_user_id', 'main_session_id', 'grant_id', 'revision', 'not_before_ms', 'expires_at_ms', 'scopes']);
  const revision = units(grant.revision);
  const from = units(grant.not_before_ms);
  const until = units(grant.expires_at_ms);
  requireCondition(revision > 0n && revision <= 4_294_967_295n && until > from && until - from <= MAX_GRANT_MS);
  requireCondition(Array.isArray(grant.scopes) && grant.scopes.length >= 1 && grant.scopes.length <= 4 && grant.scopes[0] === 'play');
  let previous = -1;
  for (const scope of grant.scopes) {
    const index = SCOPE_ORDER.indexOf(scope);
    requireCondition(index > previous);
    previous = index;
  }
  return [identifier(grant.main_user_id), identifier(grant.main_session_id), identifier(grant.grant_id),
    grant.revision, grant.not_before_ms, grant.expires_at_ms, grant.scopes.join(',')];
}

/** Encoding is not authorization: issuer must obtain the grant from a current atomic DB read. */
export function observationMessage(observation) {
  exactObject(observation, ['challenge', 'grant', 'observed_at_ms', 'key_id', 'signature_hex']);
  hex(observation.signature_hex, 64);
  units(observation.observed_at_ms);
  return canonical([OBSERVATION_DOMAIN, identifier(observation.key_id),
    digest(canonical(challengeValues(observation.challenge))), ...grantValues(observation.grant), observation.observed_at_ms]);
}

export function evidenceDigest(message, signatureHex) {
  return digest(canonical(['esk.game.session.evidence.v1', digest(message), hex(signatureHex, 64)]));
}
