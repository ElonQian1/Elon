import { createHash, createPrivateKey, createPublicKey, sign } from 'node:crypto';
import { challengeValues, canonical, digest, observationMessage, evidenceDigest } from '../src/contract.js';

// Public deterministic TEST ONLY identity. Never use its key as a deployed trust root.
const seed = createHash('sha256').update('esk-game-contract-public-synthetic-test-key-v1').digest();
const privateKey = createPrivateKey({ key: Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]), format: 'der', type: 'pkcs8' });
const publicKey = createPublicKey(privateKey).export({ format: 'der', type: 'spki' }).subarray(-32).toString('hex');

export function signedFixture(change = () => {}) {
  const observation = {
    challenge: {
      domain: 'esk.game.session.challenge.v1', main_issuer: 'synthetic-main', audience: 'esk-game', stage: 'platform_recorded',
      nonce: 'ab'.repeat(32), credential_digest: 'cd'.repeat(32),
      action: { kind: 'accept_quote', quote_id: 'synthetic-quote-1', idempotency_key: 'synthetic-request-1' },
    },
    grant: {
      main_user_id: 'synthetic-user-alice', main_session_id: 'synthetic-session-1', grant_id: 'synthetic-grant-1',
      revision: '2', not_before_ms: '1700000000000', expires_at_ms: '1700000900000', scopes: ['play', 'inventory_read', 'redeem'],
    },
    observed_at_ms: '1700000001000', key_id: 'synthetic-key-v1', signature_hex: '00'.repeat(64),
  };
  change(observation);
  const message = observationMessage(observation);
  observation.signature_hex = sign(null, message, privateKey).toString('hex');
  return {
    schema: 'esk.game.session.interoperability_fixture.v1', synthetic: true,
    main_issuer: 'synthetic-main', key_id: 'synthetic-key-v1', public_key_hex: publicKey,
    now_ms: '1700000001500', expected_challenge: structuredClone(observation.challenge), observation,
    challenge_digest: digest(canonical(challengeValues(observation.challenge))),
    message_utf8: message.toString('utf8'), authorization_digest: evidenceDigest(message, observation.signature_hex),
  };
}
