import { createPublicKey, verify } from 'node:crypto';
import {
  CLOCK_SKEW_MS, MAX_OBSERVATION_AGE_MS, actionContract, canonical,
  challengeValues, evidenceDigest, hex, identifier, observationMessage, requireCondition, units,
} from './contract.js';

/** Public key is pinned by the trusted caller, never learned from an observation. */
export function verifyObservation({ expectedChallenge, observation, mainIssuer, keyId, publicKeyHex, nowMs }) {
  identifier(mainIssuer);
  identifier(keyId);
  hex(publicKeyHex, 32);
  const now = units(nowMs);
  const expected = canonical(challengeValues(expectedChallenge));
  const actual = canonical(challengeValues(observation.challenge));
  requireCondition(expected.equals(actual), 'challenge_mismatch');
  requireCondition(expectedChallenge.main_issuer === mainIssuer && observation.key_id === keyId, 'authority_mismatch');
  const message = observationMessage(observation);
  const observed = units(observation.observed_at_ms);
  const from = units(observation.grant.not_before_ms);
  const until = units(observation.grant.expires_at_ms);
  requireCondition(now >= from && now < until && observed >= from && observed < until
    && observed <= now + CLOCK_SKEW_MS && now < observed + MAX_OBSERVATION_AGE_MS, 'observation_expired');
  requireCondition(observation.grant.scopes.includes(actionContract(expectedChallenge.action).scope), 'scope_missing');
  const publicKey = createPublicKey({
    key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), Buffer.from(publicKeyHex, 'hex')]),
    format: 'der', type: 'spki',
  });
  requireCondition(verify(null, message, publicKey, Buffer.from(observation.signature_hex, 'hex')), 'signature_invalid');
  return Object.freeze({
    schema: 'esk.game.session.verified_observation.v1',
    main_user_id: observation.grant.main_user_id,
    main_session_id: observation.grant.main_session_id,
    grant_id: observation.grant.grant_id,
    revision: observation.grant.revision,
    valid_until_ms: (until < observed + MAX_OBSERVATION_AGE_MS ? until : observed + MAX_OBSERVATION_AGE_MS).toString(),
    authorization_digest: evidenceDigest(message, observation.signature_hex),
    // Deliberately not a bearer, principal, wallet binding, or payment permission.
    wallet_bound: false, funds_moved: false,
  });
}
