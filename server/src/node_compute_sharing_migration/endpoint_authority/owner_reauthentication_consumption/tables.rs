use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE UNIQUE INDEX IF NOT EXISTS
            uq_node_endpoint_owner_reauthentication_identity_digest
            ON node_endpoint_owner_reauthentication_receipts(
                reauthentication_receipt_id, reauthentication_digest
            );

        CREATE UNIQUE INDEX IF NOT EXISTS
            uq_node_endpoint_credential_revocation_identity_digest
            ON node_endpoint_credential_revocations(revocation_id, revocation_digest);

        CREATE TABLE IF NOT EXISTS node_endpoint_owner_reauthentication_consumptions (
            consumption_id TEXT PRIMARY KEY CHECK(
                consumption_id=trim(consumption_id)
                AND length(CAST(consumption_id AS BLOB)) BETWEEN 1 AND 160
            ),
            consumption_schema TEXT NOT NULL CHECK(
                consumption_schema='elon.node_endpoint.owner_reauthentication_consumption.v1'
            ),
            consumption_digest TEXT NOT NULL UNIQUE CHECK(
                length(consumption_digest)=64
                AND consumption_digest NOT GLOB '*[^0-9a-f]*'
            ),
            consumption_json TEXT NOT NULL CHECK(
                json_valid(consumption_json)
                AND json_type(consumption_json)='object'
                AND length(CAST(consumption_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            reauthentication_receipt_id TEXT NOT NULL UNIQUE CHECK(
                reauthentication_receipt_id=trim(reauthentication_receipt_id)
                AND length(CAST(reauthentication_receipt_id AS BLOB)) BETWEEN 1 AND 160
            ),
            reauthentication_digest TEXT NOT NULL CHECK(
                length(reauthentication_digest)=64
                AND reauthentication_digest NOT GLOB '*[^0-9a-f]*'
            ),
            owner_user_id TEXT NOT NULL CHECK(
                owner_user_id=trim(owner_user_id)
                AND length(CAST(owner_user_id AS BLOB)) BETWEEN 1 AND 160
            ),
            authorization_action TEXT NOT NULL CHECK(authorization_action IN (
                'initial_registration','credential_rotation',
                'account_recovery','owner_revocation'
            )),
            credential_mutation_request_id TEXT NOT NULL CHECK(
                credential_mutation_request_id=trim(credential_mutation_request_id)
                AND length(CAST(credential_mutation_request_id AS BLOB)) BETWEEN 1 AND 160
            ),
            credential_mutation_request_digest TEXT NOT NULL CHECK(
                length(credential_mutation_request_digest)=64
                AND credential_mutation_request_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authorization_target_digest TEXT NOT NULL CHECK(
                length(authorization_target_digest)=64
                AND authorization_target_digest NOT GLOB '*[^0-9a-f]*'
            ),
            current_credential_id TEXT NOT NULL CHECK(
                current_credential_id=trim(current_credential_id)
                AND length(CAST(current_credential_id AS BLOB)) BETWEEN 1 AND 160
            ),
            current_credential_revision INTEGER NOT NULL CHECK(
                current_credential_revision BETWEEN 1 AND 9007199254740991
            ),
            current_credential_digest TEXT NOT NULL CHECK(
                length(current_credential_digest)=64
                AND current_credential_digest NOT GLOB '*[^0-9a-f]*'
            ),
            current_credential_status TEXT NOT NULL CHECK(
                current_credential_status IN ('active','revoked')
            ),
            issued_credential_id TEXT,
            issued_credential_revision INTEGER,
            issued_credential_digest TEXT,
            revocation_id TEXT,
            revocation_digest TEXT,
            consumed_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(owner_user_id, credential_mutation_request_id),
            CHECK(
                (issued_credential_id IS NULL
                    AND issued_credential_revision IS NULL
                    AND issued_credential_digest IS NULL)
                OR (issued_credential_id=current_credential_id
                    AND issued_credential_revision=current_credential_revision
                    AND issued_credential_digest=current_credential_digest)
            ),
            CHECK(
                (revocation_id IS NULL AND revocation_digest IS NULL)
                OR (revocation_id IS NOT NULL
                    AND revocation_id=trim(revocation_id)
                    AND length(CAST(revocation_id AS BLOB)) BETWEEN 1 AND 160
                    AND length(revocation_digest)=64
                    AND revocation_digest NOT GLOB '*[^0-9a-f]*')
            ),
            CHECK(
                (authorization_action='initial_registration'
                    AND current_credential_status='active'
                    AND current_credential_revision=1
                    AND issued_credential_id IS NOT NULL
                    AND revocation_id IS NULL)
                OR (authorization_action IN ('credential_rotation','account_recovery')
                    AND current_credential_status='active'
                    AND issued_credential_id IS NOT NULL
                    AND revocation_id IS NOT NULL)
                OR (authorization_action='owner_revocation'
                    AND current_credential_status='revoked'
                    AND issued_credential_id IS NULL
                    AND revocation_id IS NOT NULL)
            ),
            CHECK(consumed_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(consumed_at)=30 AND substr(consumed_at,20,1)='.'
                AND substr(consumed_at,30,1)='Z' AND julianday(consumed_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(consumed_at<=recorded_at),
            FOREIGN KEY(reauthentication_receipt_id, reauthentication_digest)
                REFERENCES node_endpoint_owner_reauthentication_receipts(
                    reauthentication_receipt_id, reauthentication_digest
                ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(owner_user_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(
                current_credential_id, current_credential_revision,
                current_credential_digest
            ) REFERENCES node_endpoint_credential_versions(
                credential_id, credential_revision, credential_digest
            ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(
                issued_credential_id, issued_credential_revision, issued_credential_digest
            ) REFERENCES node_endpoint_credential_versions(
                credential_id, credential_revision, credential_digest
            ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(revocation_id, revocation_digest)
                REFERENCES node_endpoint_credential_revocations(
                    revocation_id, revocation_digest
                ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS idx_node_endpoint_owner_reauth_consumptions_owner
            ON node_endpoint_owner_reauthentication_consumptions(
                owner_user_id, recorded_at DESC
            );
        "#,
    )?;
    Ok(())
}
