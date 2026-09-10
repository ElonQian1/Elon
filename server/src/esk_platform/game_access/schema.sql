CREATE TABLE IF NOT EXISTS game_access_grants (
  grant_id TEXT PRIMARY KEY NOT NULL,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE RESTRICT,
  parent_hash TEXT NOT NULL CHECK(length(parent_hash)=64),
  policy_digest TEXT NOT NULL CHECK(length(policy_digest)=64),
  scopes_json TEXT NOT NULL CHECK(length(scopes_json) BETWEEN 8 AND 128),
  created_ms INTEGER NOT NULL CHECK(typeof(created_ms)='integer' AND created_ms>0),
  expires_ms INTEGER NOT NULL CHECK(typeof(expires_ms)='integer' AND expires_ms>created_ms AND expires_ms<=created_ms+900000),
  revision INTEGER NOT NULL DEFAULT 1 CHECK(revision IN (1,2)),
  revoked_ms INTEGER CHECK(revoked_ms IS NULL OR (typeof(revoked_ms)='integer' AND revoked_ms>=created_ms)),
  code_hash TEXT NOT NULL UNIQUE CHECK(length(code_hash)=64),
  state_hash TEXT NOT NULL CHECK(length(state_hash)=64),
  pkce TEXT NOT NULL CHECK(length(pkce)=43),
  code_expires_ms INTEGER NOT NULL CHECK(code_expires_ms>created_ms AND code_expires_ms<=created_ms+120000 AND code_expires_ms<=expires_ms),
  consumed_ms INTEGER CHECK(consumed_ms IS NULL OR (consumed_ms>=created_ms AND consumed_ms<code_expires_ms)),
  token_hash TEXT UNIQUE CHECK(token_hash IS NULL OR length(token_hash)=64),
  CHECK((consumed_ms IS NULL)=(token_hash IS NULL)),
  CHECK((revoked_ms IS NULL AND revision=1) OR (revoked_ms IS NOT NULL AND revision=2))
);
CREATE INDEX IF NOT EXISTS idx_game_access_user ON game_access_grants(user_id, expires_ms);
CREATE TABLE IF NOT EXISTS game_access_nonces (
  grant_id TEXT NOT NULL REFERENCES game_access_grants(grant_id) ON DELETE RESTRICT,
  nonce TEXT NOT NULL CHECK(length(nonce)=64),
  observed_ms INTEGER NOT NULL CHECK(typeof(observed_ms)='integer' AND observed_ms>0),
  PRIMARY KEY(grant_id,nonce)
);
CREATE TABLE IF NOT EXISTS game_access_audit (
  grant_id TEXT NOT NULL REFERENCES game_access_grants(grant_id) ON DELETE RESTRICT,
  action TEXT NOT NULL CHECK(action IN ('authorized','exchanged','revoked')),
  at_ms INTEGER NOT NULL CHECK(typeof(at_ms)='integer' AND at_ms>0),
  PRIMARY KEY(grant_id,action)
);
CREATE TRIGGER IF NOT EXISTS game_access_grant_binding BEFORE INSERT ON game_access_grants
WHEN NOT EXISTS(SELECT 1 FROM users u JOIN sessions s ON s.user_id=u.id
  WHERE u.id=NEW.user_id AND u.id<>'local-owner' AND u.status='active'
    AND s.id=NEW.session_id AND s.token_hash=NEW.parent_hash AND s.revoked_at IS NULL
    AND julianday(s.expires_at)>=julianday(NEW.expires_ms/1000.0,'unixepoch'))
BEGIN SELECT RAISE(ABORT,'invalid game grant binding'); END;
CREATE TRIGGER IF NOT EXISTS game_access_grant_immutable BEFORE UPDATE ON game_access_grants
WHEN NEW.grant_id<>OLD.grant_id OR NEW.user_id<>OLD.user_id OR NEW.session_id<>OLD.session_id
  OR NEW.parent_hash<>OLD.parent_hash OR NEW.policy_digest<>OLD.policy_digest
  OR NEW.scopes_json<>OLD.scopes_json OR NEW.created_ms<>OLD.created_ms OR NEW.expires_ms<>OLD.expires_ms
  OR NEW.code_hash<>OLD.code_hash OR NEW.state_hash<>OLD.state_hash OR NEW.pkce<>OLD.pkce
  OR NEW.code_expires_ms<>OLD.code_expires_ms
  OR (OLD.consumed_ms IS NOT NULL AND (NEW.consumed_ms IS NOT OLD.consumed_ms OR NEW.token_hash IS NOT OLD.token_hash))
  OR (OLD.revoked_ms IS NOT NULL AND (NEW.revoked_ms IS NOT OLD.revoked_ms OR NEW.revision<>OLD.revision))
  OR (OLD.revoked_ms IS NOT NULL AND OLD.consumed_ms IS NULL AND NEW.consumed_ms IS NOT NULL)
BEGIN SELECT RAISE(ABORT,'immutable game grant'); END;
CREATE TRIGGER IF NOT EXISTS game_access_grant_no_delete BEFORE DELETE ON game_access_grants
BEGIN SELECT RAISE(ABORT,'game grant history retained'); END;
CREATE TRIGGER IF NOT EXISTS game_access_nonce_no_update BEFORE UPDATE ON game_access_nonces
BEGIN SELECT RAISE(ABORT,'nonce consumed once'); END;
CREATE TRIGGER IF NOT EXISTS game_access_nonce_no_delete BEFORE DELETE ON game_access_nonces
BEGIN SELECT RAISE(ABORT,'nonce history retained'); END;
CREATE TRIGGER IF NOT EXISTS game_access_audit_no_update BEFORE UPDATE ON game_access_audit
BEGIN SELECT RAISE(ABORT,'immutable game audit'); END;
CREATE TRIGGER IF NOT EXISTS game_access_audit_no_delete BEFORE DELETE ON game_access_audit
BEGIN SELECT RAISE(ABORT,'immutable game audit'); END;
