use anyhow::Result;
use rusqlite::Connection;

/// Private, append-only platform evidence for authenticated Sui address control.
///
/// A binding row is also the durable consumption marker for its challenge. None
/// of these tables changes ESK balances or represents a chain transaction.
pub(crate) fn migration_v290(conn: &Connection) -> Result<()> {
    conn.execute_batch("SAVEPOINT esk_platform_sui_address_binding_v290")?;
    let result = create_tables(conn).and_then(|()| {
        for (table, collision_predicate) in [
            (
                "esk_platform_sui_subjects",
                "existing.user_id=NEW.user_id OR existing.subject_commitment=NEW.subject_commitment",
            ),
            (
                "esk_platform_sui_address_binding_challenges",
                "existing.challenge_id=NEW.challenge_id",
            ),
            (
                "esk_platform_sui_address_bindings",
                "existing.binding_id=NEW.binding_id OR existing.challenge_id=NEW.challenge_id \
                 OR existing.user_id=NEW.user_id OR existing.address=NEW.address",
            ),
        ] {
            install_append_only_guards(conn, table, collision_predicate)?;
        }
        Ok(())
    });
    if let Err(error) = result {
        conn.execute_batch(
            "ROLLBACK TO esk_platform_sui_address_binding_v290;
             RELEASE esk_platform_sui_address_binding_v290",
        )?;
        return Err(error);
    }
    if let Err(error) = conn.execute_batch("RELEASE esk_platform_sui_address_binding_v290") {
        let _ = conn.execute_batch(
            "ROLLBACK TO esk_platform_sui_address_binding_v290;
             RELEASE esk_platform_sui_address_binding_v290",
        );
        return Err(error.into());
    }
    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS esk_platform_sui_subjects (
          user_id                TEXT PRIMARY KEY NOT NULL,
          subject_commitment     TEXT NOT NULL UNIQUE
            CHECK(length(subject_commitment)=71
              AND substr(subject_commitment,1,7)='sha256:'
              AND substr(subject_commitment,8) NOT GLOB '*[^0-9a-f]*'
              AND subject_commitment<>'sha256:0000000000000000000000000000000000000000000000000000000000000000'),
          created_session_id     TEXT NOT NULL,
          created_at             TEXT NOT NULL CHECK(julianday(created_at) IS NOT NULL),
          UNIQUE(user_id, subject_commitment),
          FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE RESTRICT
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS esk_platform_sui_address_binding_challenges (
          challenge_id           TEXT PRIMARY KEY NOT NULL
            CHECK(length(challenge_id)=37 AND substr(challenge_id,1,5)='eab1_'
              AND substr(challenge_id,6) NOT GLOB '*[^0-9a-f]*'),
          user_id                TEXT NOT NULL,
          subject_commitment     TEXT NOT NULL,
          created_session_id     TEXT NOT NULL,
          schema                 TEXT NOT NULL
            CHECK(schema='yilong.esk.sui.address_binding_challenge.v1'),
          network                TEXT NOT NULL CHECK(network='testnet'),
          purpose                TEXT NOT NULL CHECK(purpose='user_asset_migration'),
          address                TEXT NOT NULL
            CHECK(length(address)=66 AND substr(address,1,2)='0x'
              AND substr(address,3) NOT GLOB '*[^0-9a-f]*'
              AND address<>'0x0000000000000000000000000000000000000000000000000000000000000000'),
          ttl_seconds            INTEGER NOT NULL
            CHECK(typeof(ttl_seconds)='integer' AND ttl_seconds BETWEEN 120 AND 900),
          nonce_base64           TEXT NOT NULL
            CHECK(length(nonce_base64)=44
              AND nonce_base64 NOT GLOB '*[^A-Za-z0-9+/=]*'
              AND substr(nonce_base64,-1)='='),
          issued_at              TEXT NOT NULL CHECK(julianday(issued_at) IS NOT NULL),
          expires_at             TEXT NOT NULL CHECK(julianday(expires_at) IS NOT NULL),
          message_base64         TEXT NOT NULL
            CHECK(length(message_base64) BETWEEN 1 AND 8192
              AND message_base64 NOT GLOB '*[^A-Za-z0-9+/=]*'),
          message_sha256         TEXT NOT NULL
            CHECK(length(message_sha256)=71
              AND substr(message_sha256,1,7)='sha256:'
              AND substr(message_sha256,8) NOT GLOB '*[^0-9a-f]*'
              AND message_sha256<>'sha256:0000000000000000000000000000000000000000000000000000000000000000'),
          recorded_at            TEXT NOT NULL CHECK(julianday(recorded_at) IS NOT NULL),
          FOREIGN KEY(user_id, subject_commitment)
            REFERENCES esk_platform_sui_subjects(user_id, subject_commitment) ON DELETE RESTRICT,
          CHECK(substr(message_sha256,8,32)=substr(challenge_id,6)),
          CHECK(julianday(expires_at)>julianday(issued_at)),
          CHECK(abs((julianday(expires_at)-julianday(issued_at))*86400-ttl_seconds)<0.01),
          CHECK(julianday(recorded_at)>=julianday(issued_at)
            AND julianday(recorded_at)<julianday(expires_at))
        ) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS idx_esk_platform_sui_binding_challenges_user
          ON esk_platform_sui_address_binding_challenges(user_id, recorded_at DESC, challenge_id);

        CREATE TABLE IF NOT EXISTS esk_platform_sui_address_bindings (
          binding_id             TEXT PRIMARY KEY NOT NULL
            CHECK(length(binding_id)=39 AND substr(binding_id,1,7)='eskpsb_'
              AND substr(binding_id,8) NOT GLOB '*[^0-9a-f]*'),
          challenge_id           TEXT NOT NULL UNIQUE,
          user_id                TEXT NOT NULL UNIQUE,
          address                TEXT NOT NULL UNIQUE,
          network                TEXT NOT NULL CHECK(network='testnet'),
          subject_commitment     TEXT NOT NULL,
          message_sha256         TEXT NOT NULL,
          signature_scheme       TEXT NOT NULL
            CHECK(signature_scheme IN ('ed25519','secp256k1','secp256r1')),
          signature_sha256       TEXT NOT NULL
            CHECK(length(signature_sha256)=71
              AND substr(signature_sha256,1,7)='sha256:'
              AND substr(signature_sha256,8) NOT GLOB '*[^0-9a-f]*'),
          response_digest        TEXT NOT NULL
            CHECK(length(response_digest)=71
              AND substr(response_digest,1,7)='sha256:'
              AND substr(response_digest,8) NOT GLOB '*[^0-9a-f]*'),
          binding_receipt_sha256 TEXT NOT NULL
            CHECK(length(binding_receipt_sha256)=71
              AND substr(binding_receipt_sha256,1,7)='sha256:'
              AND substr(binding_receipt_sha256,8) NOT GLOB '*[^0-9a-f]*'),
          wallet_response_json   TEXT NOT NULL
            CHECK(length(wallet_response_json) BETWEEN 2 AND 65536
              AND json_valid(wallet_response_json)=1
              AND json_type(wallet_response_json)='object'),
          completed_session_id   TEXT NOT NULL,
          verified_at            TEXT NOT NULL CHECK(julianday(verified_at) IS NOT NULL),
          bound_at               TEXT NOT NULL CHECK(julianday(bound_at) IS NOT NULL),
          FOREIGN KEY(challenge_id)
            REFERENCES esk_platform_sui_address_binding_challenges(challenge_id) ON DELETE RESTRICT,
          FOREIGN KEY(user_id, subject_commitment)
            REFERENCES esk_platform_sui_subjects(user_id, subject_commitment) ON DELETE RESTRICT
        ) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS idx_esk_platform_sui_bindings_user_bound
          ON esk_platform_sui_address_bindings(user_id, bound_at DESC, binding_id);

        CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sui_subject_insert_binding
        BEFORE INSERT ON esk_platform_sui_subjects
        WHEN NOT EXISTS (
          SELECT 1 FROM users u JOIN sessions s ON s.id=NEW.created_session_id
           WHERE u.id=NEW.user_id AND u.status='active' AND u.id<>'local-owner'
             AND s.user_id=u.id AND s.revoked_at IS NULL
             AND julianday(s.expires_at)>julianday(NEW.created_at)
        ) BEGIN
          SELECT RAISE(ABORT, 'ESK Sui subject authentication is invalid');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sui_challenge_insert_binding
        BEFORE INSERT ON esk_platform_sui_address_binding_challenges
        WHEN NOT EXISTS (
          SELECT 1 FROM esk_platform_sui_subjects subject
            JOIN users u ON u.id=subject.user_id
            JOIN sessions s ON s.id=NEW.created_session_id
           WHERE subject.user_id=NEW.user_id
             AND subject.subject_commitment=NEW.subject_commitment
             AND u.status='active' AND u.id<>'local-owner'
             AND s.user_id=u.id AND s.revoked_at IS NULL
             AND julianday(s.expires_at)>julianday(NEW.recorded_at)
        ) BEGIN
          SELECT RAISE(ABORT, 'ESK Sui challenge authentication is invalid');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sui_challenge_reuse_required
        BEFORE INSERT ON esk_platform_sui_address_binding_challenges
        WHEN EXISTS (
          SELECT 1 FROM esk_platform_sui_address_binding_challenges challenge
           WHERE challenge.user_id=NEW.user_id AND challenge.address=NEW.address
              AND julianday(challenge.issued_at)<=julianday(NEW.recorded_at)
              AND julianday(challenge.expires_at)>julianday(NEW.recorded_at)
        ) BEGIN
          SELECT RAISE(ABORT, 'ESK Sui live challenge must be reused');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sui_challenge_rate_limit
        BEFORE INSERT ON esk_platform_sui_address_binding_challenges
        WHEN (
          SELECT COUNT(*) FROM esk_platform_sui_address_binding_challenges challenge
           WHERE challenge.user_id=NEW.user_id
              AND julianday(challenge.issued_at)<=julianday(NEW.recorded_at)
              AND julianday(challenge.expires_at)>julianday(NEW.recorded_at)
        )>=3 OR (
          SELECT COUNT(*) FROM esk_platform_sui_address_binding_challenges challenge
           WHERE challenge.user_id=NEW.user_id
             AND julianday(challenge.recorded_at)>julianday(NEW.recorded_at,'-24 hours')
             AND julianday(challenge.recorded_at)<=julianday(NEW.recorded_at)
        )>=20 BEGIN
          SELECT RAISE(ABORT, 'ESK Sui challenge rate limited');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_esk_platform_sui_binding_insert_binding
        BEFORE INSERT ON esk_platform_sui_address_bindings
        WHEN NOT EXISTS (
          SELECT 1 FROM esk_platform_sui_address_binding_challenges challenge
            JOIN esk_platform_sui_subjects subject
              ON subject.user_id=challenge.user_id
             AND subject.subject_commitment=challenge.subject_commitment
            JOIN users u ON u.id=challenge.user_id
            JOIN sessions s ON s.id=NEW.completed_session_id
           WHERE challenge.challenge_id=NEW.challenge_id
             AND challenge.user_id=NEW.user_id
             AND challenge.address=NEW.address
             AND challenge.network=NEW.network
             AND challenge.subject_commitment=NEW.subject_commitment
             AND challenge.message_sha256=NEW.message_sha256
             AND u.status='active' AND u.id<>'local-owner'
             AND s.user_id=u.id AND s.revoked_at IS NULL
             AND julianday(s.expires_at)>julianday(NEW.bound_at)
             AND julianday(NEW.verified_at)>=julianday(challenge.issued_at)
             AND julianday(NEW.verified_at)<julianday(challenge.expires_at)
             AND julianday(NEW.bound_at)>=julianday(NEW.verified_at)
             AND julianday(NEW.bound_at)<julianday(challenge.expires_at)
             AND (SELECT COUNT(*) FROM json_each(NEW.wallet_response_json))=4
             AND json_extract(NEW.wallet_response_json,'$.schema')
               ='yilong.esk.sui.address_binding_wallet_response.v1'
             AND json_extract(NEW.wallet_response_json,'$.challenge_id')=NEW.challenge_id
             AND json_extract(NEW.wallet_response_json,'$.message_base64')=challenge.message_base64
             AND json_type(NEW.wallet_response_json,'$.signature')='text'
             AND length(json_extract(NEW.wallet_response_json,'$.signature')) BETWEEN 4 AND 8192
        ) BEGIN
          SELECT RAISE(ABORT, 'ESK Sui address binding is invalid');
        END;
        "#,
    )?;
    Ok(())
}

fn install_append_only_guards(
    conn: &Connection,
    table: &str,
    collision_predicate: &str,
) -> Result<()> {
    // SQLite implements INSERT OR REPLACE as a delete followed by an insert.
    // Recursive triggers are intentionally disabled for the Store connection,
    // so a DELETE guard alone cannot make replacement writes append-only.
    conn.execute_batch(&format!(
        "CREATE TRIGGER IF NOT EXISTS trg_{table}_no_replacement_insert
         BEFORE INSERT ON {table}
         WHEN EXISTS (
           SELECT 1 FROM {table} existing WHERE {collision_predicate}
         ) BEGIN
           SELECT RAISE(ABORT, 'ESK Sui address binding records are append-only');
         END;"
    ))?;
    for (suffix, operation) in [("update", "UPDATE"), ("delete", "DELETE")] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER IF NOT EXISTS trg_{table}_no_{suffix}
             BEFORE {operation} ON {table} BEGIN
               SELECT RAISE(ABORT, 'ESK Sui address binding records are append-only');
             END;"
        ))?;
    }
    Ok(())
}
