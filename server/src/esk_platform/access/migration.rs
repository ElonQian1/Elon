use anyhow::Result;
use rusqlite::Connection;

/// Independent delegated read authorization; never migrates identities or asset balances.
pub(crate) fn migration_v289(conn: &Connection) -> Result<()> {
    conn.execute_batch("SAVEPOINT asset_access_v289")?;
    let result = create_tables(conn).and_then(|()| create_guards(conn));
    if let Err(error) = result {
        conn.execute_batch("ROLLBACK TO asset_access_v289; RELEASE asset_access_v289")?;
        return Err(error);
    }
    conn.execute_batch("RELEASE asset_access_v289")?;
    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS asset_access_subjects (
           user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
           client_id TEXT NOT NULL CHECK(client_id IN ('quant.android','quant.web','quant.ai')),
           subject TEXT NOT NULL UNIQUE CHECK(length(subject) = 68),
           PRIMARY KEY(user_id, client_id)
         );
         CREATE TABLE IF NOT EXISTS asset_access_grants (
           grant_id TEXT PRIMARY KEY NOT NULL CHECK(length(grant_id) = 36),
           user_id TEXT NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
           client_id TEXT NOT NULL CHECK(client_id IN ('quant.android','quant.web','quant.ai')),
           subject TEXT NOT NULL REFERENCES asset_access_subjects(subject) ON DELETE RESTRICT,
           parent_session_hash TEXT NOT NULL CHECK(length(parent_session_hash) = 64),
           scopes_json TEXT NOT NULL CHECK(length(scopes_json) BETWEEN 20 AND 128),
           created_at_unix INTEGER NOT NULL CHECK(typeof(created_at_unix)='integer' AND created_at_unix > 0),
           expires_at_unix INTEGER NOT NULL CHECK(typeof(expires_at_unix)='integer'
             AND expires_at_unix > created_at_unix AND expires_at_unix <= created_at_unix + 3600),
           revoked_at_unix INTEGER CHECK(revoked_at_unix IS NULL OR
             (typeof(revoked_at_unix)='integer' AND revoked_at_unix >= created_at_unix))
         );
         CREATE INDEX IF NOT EXISTS idx_asset_access_grants_user
           ON asset_access_grants(user_id, created_at_unix DESC, grant_id DESC);
         CREATE INDEX IF NOT EXISTS idx_asset_access_grants_session ON asset_access_grants(parent_session_hash);
         CREATE TABLE IF NOT EXISTS asset_access_codes (
           code_hash TEXT PRIMARY KEY NOT NULL CHECK(length(code_hash)=64),
           grant_id TEXT NOT NULL UNIQUE REFERENCES asset_access_grants(grant_id) ON DELETE RESTRICT,
           redirect_uri TEXT NOT NULL CHECK(length(redirect_uri) BETWEEN 1 AND 2048),
           state_hash TEXT NOT NULL CHECK(length(state_hash)=64),
           code_challenge TEXT NOT NULL CHECK(length(code_challenge)=43),
           created_at_unix INTEGER NOT NULL CHECK(typeof(created_at_unix)='integer' AND created_at_unix > 0),
           expires_at_unix INTEGER NOT NULL CHECK(typeof(expires_at_unix)='integer'
             AND expires_at_unix > created_at_unix AND expires_at_unix <= created_at_unix + 120),
           consumed_at_unix INTEGER CHECK(consumed_at_unix IS NULL OR
             (typeof(consumed_at_unix)='integer' AND consumed_at_unix >= created_at_unix))
         );
         CREATE TABLE IF NOT EXISTS asset_access_tokens (
           token_hash TEXT PRIMARY KEY NOT NULL CHECK(length(token_hash)=64),
           grant_id TEXT NOT NULL UNIQUE REFERENCES asset_access_grants(grant_id) ON DELETE RESTRICT,
           created_at_unix INTEGER NOT NULL CHECK(typeof(created_at_unix)='integer' AND created_at_unix > 0),
           expires_at_unix INTEGER NOT NULL CHECK(typeof(expires_at_unix)='integer' AND expires_at_unix > created_at_unix),
           revoked_at_unix INTEGER CHECK(revoked_at_unix IS NULL OR
             (typeof(revoked_at_unix)='integer' AND revoked_at_unix >= created_at_unix))
         );
         CREATE TABLE IF NOT EXISTS asset_access_audit (
           audit_id TEXT PRIMARY KEY NOT NULL,
           grant_id TEXT NOT NULL REFERENCES asset_access_grants(grant_id) ON DELETE RESTRICT,
           action TEXT NOT NULL CHECK(action IN ('authorized','exchanged','revoked')),
           created_at_unix INTEGER NOT NULL CHECK(typeof(created_at_unix)='integer' AND created_at_unix > 0),
           UNIQUE(grant_id, action)
         );"
    )?;
    Ok(())
}

fn create_guards(conn: &Connection) -> Result<()> {
    for table in [
        "asset_access_subjects",
        "asset_access_grants",
        "asset_access_codes",
        "asset_access_tokens",
        "asset_access_audit",
    ] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS trg_{table}_no_delete BEFORE DELETE ON {table}
             BEGIN SELECT RAISE(ABORT,'asset access history cannot be deleted'); END;"
        ))?;
    }
    for table in ["asset_access_subjects", "asset_access_audit"] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS trg_{table}_no_update BEFORE UPDATE ON {table}
             BEGIN SELECT RAISE(ABORT,'asset access history is immutable'); END;"
        ))?;
    }
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS trg_asset_access_grant_binding BEFORE INSERT ON asset_access_grants
         WHEN NOT EXISTS (SELECT 1 FROM asset_access_subjects sub JOIN users u ON u.id=sub.user_id
           JOIN sessions s ON s.user_id=u.id
           WHERE sub.user_id=NEW.user_id AND sub.client_id=NEW.client_id AND sub.subject=NEW.subject
             AND u.status='active' AND u.id<>'local-owner' AND s.token_hash=NEW.parent_session_hash
             AND s.revoked_at IS NULL
             AND julianday(s.expires_at) >= julianday(NEW.expires_at_unix,'unixepoch'))
         BEGIN SELECT RAISE(ABORT,'asset access grant binding invalid'); END;
         CREATE TRIGGER IF NOT EXISTS trg_asset_access_grant_immutable BEFORE UPDATE ON asset_access_grants
         WHEN NEW.grant_id<>OLD.grant_id OR NEW.user_id<>OLD.user_id OR NEW.client_id<>OLD.client_id
           OR NEW.subject<>OLD.subject OR NEW.parent_session_hash<>OLD.parent_session_hash
           OR NEW.scopes_json<>OLD.scopes_json OR NEW.created_at_unix<>OLD.created_at_unix
           OR NEW.expires_at_unix<>OLD.expires_at_unix
           OR (OLD.revoked_at_unix IS NOT NULL AND NEW.revoked_at_unix IS NOT OLD.revoked_at_unix)
         BEGIN SELECT RAISE(ABORT,'asset access grant is immutable'); END;
         CREATE TRIGGER IF NOT EXISTS trg_asset_access_code_binding BEFORE INSERT ON asset_access_codes
         WHEN NOT EXISTS (SELECT 1 FROM asset_access_grants g WHERE g.grant_id=NEW.grant_id
           AND g.revoked_at_unix IS NULL AND g.created_at_unix=NEW.created_at_unix
           AND NEW.expires_at_unix<=g.expires_at_unix)
         BEGIN SELECT RAISE(ABORT,'asset access code binding invalid'); END;
         CREATE TRIGGER IF NOT EXISTS trg_asset_access_code_immutable BEFORE UPDATE ON asset_access_codes
         WHEN NEW.code_hash<>OLD.code_hash OR NEW.grant_id<>OLD.grant_id OR NEW.redirect_uri<>OLD.redirect_uri
           OR NEW.state_hash<>OLD.state_hash OR NEW.code_challenge<>OLD.code_challenge
           OR NEW.created_at_unix<>OLD.created_at_unix OR NEW.expires_at_unix<>OLD.expires_at_unix
           OR OLD.consumed_at_unix IS NOT NULL OR NEW.consumed_at_unix IS NULL
           OR NEW.consumed_at_unix>=OLD.expires_at_unix
         BEGIN SELECT RAISE(ABORT,'asset access code is single use'); END;
         CREATE TRIGGER IF NOT EXISTS trg_asset_access_token_binding BEFORE INSERT ON asset_access_tokens
         WHEN NOT EXISTS (SELECT 1 FROM asset_access_grants g JOIN asset_access_codes c ON c.grant_id=g.grant_id
           WHERE g.grant_id=NEW.grant_id AND g.revoked_at_unix IS NULL
             AND c.consumed_at_unix=NEW.created_at_unix AND NEW.expires_at_unix=g.expires_at_unix)
         BEGIN SELECT RAISE(ABORT,'asset access token binding invalid'); END;
         CREATE TRIGGER IF NOT EXISTS trg_asset_access_token_immutable BEFORE UPDATE ON asset_access_tokens
         WHEN NEW.token_hash<>OLD.token_hash OR NEW.grant_id<>OLD.grant_id
           OR NEW.created_at_unix<>OLD.created_at_unix OR NEW.expires_at_unix<>OLD.expires_at_unix
           OR (OLD.revoked_at_unix IS NOT NULL AND NEW.revoked_at_unix IS NOT OLD.revoked_at_unix)
         BEGIN SELECT RAISE(ABORT,'asset access token is immutable'); END;"
    )?;
    Ok(())
}
