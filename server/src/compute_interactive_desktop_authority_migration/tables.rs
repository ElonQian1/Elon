use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_interactive_desktop_authority_versions (
            authority_record_schema TEXT NOT NULL CHECK(
                authority_record_schema=
                    'compute_federation.interactive_desktop.authority_record.v1'
            ),
            authority_record_digest TEXT NOT NULL CHECK(
                length(authority_record_digest)=64
                AND authority_record_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authority_record_json TEXT NOT NULL CHECK(
                json_valid(authority_record_json)
                AND json_type(authority_record_json)='object'
                AND length(CAST(authority_record_json AS BLOB)) BETWEEN 2 AND 1048576
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            session_id TEXT NOT NULL CHECK(
                session_id=trim(session_id)
                AND length(CAST(session_id AS BLOB)) BETWEEN 1 AND 160
            ),
            session_root_digest TEXT NOT NULL CHECK(
                length(session_root_digest)=64
                AND session_root_digest NOT GLOB '*[^0-9a-f]*'
            ),
            session_revision INTEGER NOT NULL CHECK(
                session_revision BETWEEN 1 AND 9007199254740991
            ),
            session_digest TEXT NOT NULL CHECK(
                length(session_digest)=64
                AND session_digest NOT GLOB '*[^0-9a-f]*'
            ),
            session_state TEXT NOT NULL CHECK(
                session_state IN (
                    'requested','reserved','host_leased','viewer_granted','connecting',
                    'active','reconnecting','ending','ended','canceled','failed'
                )
            ),
            is_terminal INTEGER NOT NULL CHECK(is_terminal IN (0,1)),
            session_reservation_id TEXT NOT NULL CHECK(
                session_reservation_id=trim(session_reservation_id)
                AND length(CAST(session_reservation_id AS BLOB)) BETWEEN 1 AND 160
            ),
            session_reservation_revision INTEGER NOT NULL CHECK(
                session_reservation_revision BETWEEN 1 AND 9007199254740991
            ),
            session_reservation_digest TEXT NOT NULL CHECK(
                length(session_reservation_digest)=64
                AND session_reservation_digest NOT GLOB '*[^0-9a-f]*'
            ),
            binding_digest TEXT NOT NULL CHECK(
                length(binding_digest)=64
                AND binding_digest NOT GLOB '*[^0-9a-f]*'
            ),
            provider_id TEXT NOT NULL CHECK(
                provider_id=trim(provider_id)
                AND length(CAST(provider_id AS BLOB)) BETWEEN 1 AND 160
            ),
            provider_policy_revision INTEGER NOT NULL CHECK(
                provider_policy_revision BETWEEN 1 AND 9007199254740991
            ),
            provider_digest TEXT NOT NULL CHECK(
                length(provider_digest)=64
                AND provider_digest NOT GLOB '*[^0-9a-f]*'
            ),
            provider_owner_account_id TEXT NOT NULL CHECK(
                provider_owner_account_id=trim(provider_owner_account_id)
                AND length(CAST(provider_owner_account_id AS BLOB)) BETWEEN 1 AND 160
            ),
            consumer_account_id TEXT NOT NULL CHECK(
                consumer_account_id=trim(consumer_account_id)
                AND length(CAST(consumer_account_id AS BLOB)) BETWEEN 1 AND 160
            ),
            host_lease_id TEXT NOT NULL CHECK(
                host_lease_id=trim(host_lease_id)
                AND length(CAST(host_lease_id AS BLOB)) BETWEEN 1 AND 160
            ),
            host_lease_digest TEXT NOT NULL CHECK(
                length(host_lease_digest)=64
                AND host_lease_digest NOT GLOB '*[^0-9a-f]*'
            ),
            fencing_generation INTEGER NOT NULL CHECK(
                fencing_generation BETWEEN 1 AND 9007199254740991
            ),
            viewer_grant_id TEXT NOT NULL CHECK(
                viewer_grant_id=trim(viewer_grant_id)
                AND length(CAST(viewer_grant_id AS BLOB)) BETWEEN 1 AND 160
            ),
            viewer_grant_digest TEXT NOT NULL CHECK(
                length(viewer_grant_digest)=64
                AND viewer_grant_digest NOT GLOB '*[^0-9a-f]*'
            ),
            viewer_grant_generation INTEGER NOT NULL CHECK(
                viewer_grant_generation BETWEEN 1 AND 9007199254740991
            ),
            media_epoch_id TEXT NOT NULL CHECK(
                media_epoch_id=trim(media_epoch_id)
                AND length(CAST(media_epoch_id AS BLOB)) BETWEEN 1 AND 160
            ),
            media_epoch_digest TEXT NOT NULL CHECK(
                length(media_epoch_digest)=64
                AND media_epoch_digest NOT GLOB '*[^0-9a-f]*'
            ),
            media_epoch_sequence INTEGER NOT NULL CHECK(
                media_epoch_sequence BETWEEN 1 AND 9007199254740991
            ),
            control_epoch_id TEXT NOT NULL CHECK(
                control_epoch_id=trim(control_epoch_id)
                AND length(CAST(control_epoch_id AS BLOB)) BETWEEN 1 AND 160
            ),
            control_epoch_digest TEXT NOT NULL CHECK(
                length(control_epoch_digest)=64
                AND control_epoch_digest NOT GLOB '*[^0-9a-f]*'
            ),
            control_epoch_sequence INTEGER NOT NULL CHECK(
                control_epoch_sequence BETWEEN 1 AND 9007199254740991
            ),
            selected_surface_digest TEXT NOT NULL CHECK(
                length(selected_surface_digest)=64
                AND selected_surface_digest NOT GLOB '*[^0-9a-f]*'
            ),
            viewer_transport_identity_digest TEXT NOT NULL CHECK(
                length(viewer_transport_identity_digest)=64
                AND viewer_transport_identity_digest NOT GLOB '*[^0-9a-f]*'
            ),
            recorded_at_ms INTEGER NOT NULL CHECK(
                recorded_at_ms BETWEEN 0 AND 9007199254740991
            ),
            PRIMARY KEY(session_id,session_revision),
            UNIQUE(authority_record_digest),
            UNIQUE(session_id,session_digest),
            UNIQUE(
                session_id,session_revision,session_digest,authority_record_digest
            ),
            CHECK(
                (is_terminal=1 AND session_state IN ('ended','canceled','failed'))
                OR (is_terminal=0 AND session_state NOT IN ('ended','canceled','failed'))
            ),
            FOREIGN KEY(provider_id,provider_policy_revision)
                REFERENCES compute_provider_versions(provider_id,policy_revision)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_owner_account_id) REFERENCES users(id) ON DELETE RESTRICT,
            FOREIGN KEY(consumer_account_id) REFERENCES users(id) ON DELETE RESTRICT
        ) WITHOUT ROWID;

        CREATE TABLE IF NOT EXISTS compute_interactive_desktop_authority_heads (
            session_id TEXT PRIMARY KEY CHECK(
                session_id=trim(session_id)
                AND length(CAST(session_id AS BLOB)) BETWEEN 1 AND 160
            ),
            session_root_digest TEXT NOT NULL CHECK(
                length(session_root_digest)=64
                AND session_root_digest NOT GLOB '*[^0-9a-f]*'
            ),
            current_session_revision INTEGER NOT NULL CHECK(
                current_session_revision BETWEEN 1 AND 9007199254740991
            ),
            current_session_digest TEXT NOT NULL CHECK(
                length(current_session_digest)=64
                AND current_session_digest NOT GLOB '*[^0-9a-f]*'
            ),
            current_authority_record_digest TEXT NOT NULL UNIQUE CHECK(
                length(current_authority_record_digest)=64
                AND current_authority_record_digest NOT GLOB '*[^0-9a-f]*'
            ),
            session_state TEXT NOT NULL CHECK(
                session_state IN (
                    'requested','reserved','host_leased','viewer_granted','connecting',
                    'active','reconnecting','ending','ended','canceled','failed'
                )
            ),
            is_terminal INTEGER NOT NULL CHECK(is_terminal IN (0,1)),
            created_at_ms INTEGER NOT NULL CHECK(
                created_at_ms BETWEEN 0 AND 9007199254740991
            ),
            updated_at_ms INTEGER NOT NULL CHECK(
                updated_at_ms BETWEEN created_at_ms AND 9007199254740991
            ),
            CHECK(
                (is_terminal=1 AND session_state IN ('ended','canceled','failed'))
                OR (is_terminal=0 AND session_state NOT IN ('ended','canceled','failed'))
            ),
            FOREIGN KEY(
                session_id,current_session_revision,current_session_digest,
                current_authority_record_digest
            ) REFERENCES compute_interactive_desktop_authority_versions(
                session_id,session_revision,session_digest,authority_record_digest
            ) ON DELETE RESTRICT
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS idx_interactive_desktop_authority_provider
            ON compute_interactive_desktop_authority_versions(
                provider_id,provider_policy_revision,session_id,session_revision
            );

        CREATE INDEX IF NOT EXISTS idx_interactive_desktop_authority_lease
            ON compute_interactive_desktop_authority_versions(
                host_lease_id,viewer_grant_id,media_epoch_id,control_epoch_id
            );
        "#,
    )?;
    Ok(())
}
