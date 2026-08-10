use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS node_compute_plugin_endpoint_planning_chain_events_v1 (
            event_id TEXT PRIMARY KEY CHECK(
                event_id=trim(event_id) AND length(CAST(event_id AS BLOB)) BETWEEN 1 AND 160
            ),
            event_schema TEXT NOT NULL CHECK(
                event_schema='elon.node_compute_plugin.endpoint_planning_chain_event.v1'
            ),
            bootstrap_id TEXT NOT NULL CHECK(
                bootstrap_id=trim(bootstrap_id)
                AND length(CAST(bootstrap_id AS BLOB)) BETWEEN 1 AND 160
            ),
            message_sequence INTEGER NOT NULL CHECK(message_sequence BETWEEN 1 AND 6),
            message_kind TEXT NOT NULL CHECK(message_kind IN (
                'sharing_request','sharing_observed','preparation_request',
                'preparation_observed','snapshot_request','snapshot_observed'
            )),
            previous_message_sequence INTEGER,
            previous_event_id TEXT,
            next_message_sequence INTEGER,
            next_event_id TEXT,
            message_schema TEXT NOT NULL CHECK(
                message_schema=trim(message_schema)
                AND length(CAST(message_schema AS BLOB)) BETWEEN 1 AND 160
            ),
            message_json TEXT NOT NULL CHECK(
                json_valid(message_json) AND json_type(message_json)='object'
                AND length(CAST(message_json AS BLOB))<=1048576
            ),
            message_digest TEXT NOT NULL UNIQUE CHECK(
                length(message_digest)=64 AND message_digest NOT GLOB '*[^0-9a-f]*'
            ),
            previous_message_digest TEXT NOT NULL CHECK(
                length(previous_message_digest)=64
                AND previous_message_digest NOT GLOB '*[^0-9a-f]*'
            ),
            delivery_id TEXT NOT NULL CHECK(
                delivery_id=trim(delivery_id)
                AND length(CAST(delivery_id AS BLOB)) BETWEEN 1 AND 160
            ),
            agent_id TEXT NOT NULL,
            owner_user_id TEXT NOT NULL,
            install_id TEXT NOT NULL,
            installation_binding_digest TEXT NOT NULL CHECK(length(installation_binding_digest)=64),
            plugin_installation_identity_digest TEXT NOT NULL CHECK(
                length(plugin_installation_identity_digest)=64
                AND plugin_installation_identity_digest NOT GLOB '*[^0-9a-f]*'
            ),
            credential_id TEXT NOT NULL,
            credential_revision INTEGER NOT NULL CHECK(
                credential_revision BETWEEN 1 AND 9007199254740991
            ),
            credential_digest TEXT NOT NULL CHECK(length(credential_digest)=64),
            authentication_receipt_id TEXT NOT NULL,
            authentication_digest TEXT NOT NULL CHECK(length(authentication_digest)=64),
            session_id TEXT NOT NULL,
            session_generation INTEGER NOT NULL CHECK(
                session_generation BETWEEN 1 AND 9007199254740991
            ),
            server_instance_id TEXT NOT NULL,
            agent_version TEXT NOT NULL,
            authenticated_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            protocol_version INTEGER NOT NULL CHECK(protocol_version=14),
            capability_count INTEGER NOT NULL CHECK(capability_count=1),
            capability_set_json TEXT NOT NULL CHECK(
                capability_set_json='["node_endpoint_planning_snapshot_bootstrap_v1"]'
            ),
            capability_set_digest TEXT NOT NULL CHECK(length(capability_set_digest)=64),
            consent_receipt_id TEXT NOT NULL,
            policy_revision INTEGER NOT NULL CHECK(policy_revision BETWEEN 1 AND 9007199254740991),
            policy_digest TEXT NOT NULL CHECK(length(policy_digest)=64),
            policy_snapshot_digest TEXT NOT NULL CHECK(length(policy_snapshot_digest)=64),
            plugin_runtime_requested INTEGER NOT NULL CHECK(plugin_runtime_requested IN (0,1)),
            sharing_delivery_id TEXT NOT NULL,
            sharing_observation_id TEXT,
            sharing_observation_digest TEXT,
            preparation_id TEXT,
            preparation_delivery_id TEXT,
            preparation_request_digest TEXT,
            preparation_observation_id TEXT,
            preparation_observation_digest TEXT,
            planning_delivery_id TEXT,
            planning_request_digest TEXT,
            planning_observation_event_id TEXT,
            planning_observation_digest TEXT,
            accepted INTEGER CHECK(accepted IN (0,1)),
            replayed INTEGER CHECK(replayed IN (0,1)),
            snapshot_ready INTEGER CHECK(snapshot_ready IN (0,1)),
            recorded_at TEXT NOT NULL,
            UNIQUE(bootstrap_id, message_sequence),
            UNIQUE(bootstrap_id, message_sequence, event_id),
            UNIQUE(bootstrap_id, previous_event_id),
            CHECK(
                (message_sequence=1 AND message_kind='sharing_request'
                    AND previous_message_sequence IS NULL AND previous_event_id IS NULL
                    AND previous_message_digest=authentication_digest)
                OR (message_sequence>1 AND previous_message_sequence=message_sequence-1
                    AND previous_event_id IS NOT NULL)
            ),
            CHECK(
                (((message_sequence=2 AND accepted=1 AND plugin_runtime_requested=1)
                    OR (message_sequence=4 AND accepted=1))
                    AND next_message_sequence=message_sequence+1 AND next_event_id IS NOT NULL)
                OR ((message_sequence NOT IN (2,4) OR accepted=0
                    OR (message_sequence=2 AND plugin_runtime_requested=0))
                    AND next_message_sequence IS NULL AND next_event_id IS NULL)
            ),
            CHECK(plugin_runtime_requested=1 OR message_sequence<=2),
            CHECK(
                (message_sequence=1 AND message_kind='sharing_request')
                OR (message_sequence=2 AND message_kind='sharing_observed')
                OR (message_sequence=3 AND message_kind='preparation_request')
                OR (message_sequence=4 AND message_kind='preparation_observed')
                OR (message_sequence=5 AND message_kind='snapshot_request')
                OR (message_sequence=6 AND message_kind='snapshot_observed')
            ),
            CHECK(
                (message_sequence=1 AND sharing_observation_id IS NULL
                    AND sharing_observation_digest IS NULL)
                OR (message_sequence>=2 AND sharing_observation_id IS NOT NULL
                    AND length(sharing_observation_digest)=64)
            ),
            CHECK(
                (message_sequence<=2 AND preparation_id IS NULL
                    AND preparation_delivery_id IS NULL AND preparation_request_digest IS NULL)
                OR (message_sequence>=3 AND preparation_id IS NOT NULL
                    AND preparation_delivery_id IS NOT NULL
                    AND length(preparation_request_digest)=64)
            ),
            CHECK(
                (message_sequence<=3 AND preparation_observation_id IS NULL
                    AND preparation_observation_digest IS NULL)
                OR (message_sequence>=4 AND preparation_observation_id IS NOT NULL
                    AND length(preparation_observation_digest)=64)
            ),
            CHECK(
                (message_sequence<=4 AND planning_delivery_id IS NULL
                    AND planning_request_digest IS NULL)
                OR (message_sequence>=5 AND planning_delivery_id IS NOT NULL
                    AND length(planning_request_digest)=64)
            ),
            CHECK(
                (message_sequence<=5 AND planning_observation_event_id IS NULL
                    AND planning_observation_digest IS NULL)
                OR (message_sequence=6 AND planning_observation_event_id IS NOT NULL
                    AND length(planning_observation_digest)=64)
            ),
            CHECK(
                (message_sequence IN (1,3,5) AND accepted IS NULL
                    AND replayed IS NULL AND snapshot_ready IS NULL)
                OR (message_sequence IN (2,4) AND accepted IS NOT NULL
                    AND replayed IS NOT NULL AND snapshot_ready IS NULL)
                OR (message_sequence=6 AND accepted IS NOT NULL
                    AND replayed IS NOT NULL AND snapshot_ready=0)
            ),
            CHECK(authenticated_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(authenticated_at)=30 AND julianday(authenticated_at) IS NOT NULL),
            CHECK(expires_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(expires_at)=30 AND julianday(expires_at) IS NOT NULL),
            CHECK(recorded_at GLOB '????-??-??T??:??:??.?????????Z'
                AND length(recorded_at)=30 AND julianday(recorded_at) IS NOT NULL),
            CHECK(authenticated_at<=recorded_at AND recorded_at<expires_at),
            FOREIGN KEY(authentication_receipt_id, authentication_digest)
                REFERENCES node_endpoint_session_authentication_receipts(
                    authentication_receipt_id, authentication_digest
                ) ON DELETE RESTRICT,
            FOREIGN KEY(consent_receipt_id)
                REFERENCES node_compute_plugin_sharing_consents(receipt_id) ON DELETE RESTRICT,
            FOREIGN KEY(sharing_observation_id)
                REFERENCES node_compute_plugin_sharing_observations(id) ON DELETE RESTRICT,
            FOREIGN KEY(preparation_observation_id)
                REFERENCES node_compute_plugin_install_plan_preparation_observations(id)
                ON DELETE RESTRICT,
            FOREIGN KEY(planning_observation_event_id)
                REFERENCES node_compute_plugin_install_plan_planning_delivery_events_v2(id)
                ON DELETE RESTRICT,
            FOREIGN KEY(bootstrap_id, previous_message_sequence, previous_event_id)
                REFERENCES node_compute_plugin_endpoint_planning_chain_events_v1(
                    bootstrap_id, message_sequence, event_id
                ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(bootstrap_id, next_message_sequence, next_event_id)
                REFERENCES node_compute_plugin_endpoint_planning_chain_events_v1(
                    bootstrap_id, message_sequence, event_id
                ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        ) WITHOUT ROWID;

        CREATE INDEX IF NOT EXISTS idx_node_endpoint_planning_chain_session
            ON node_compute_plugin_endpoint_planning_chain_events_v1(
                session_id, session_generation, bootstrap_id, message_sequence
            );
        CREATE UNIQUE INDEX IF NOT EXISTS idx_node_endpoint_planning_one_chain_per_receipt
            ON node_compute_plugin_endpoint_planning_chain_events_v1(authentication_receipt_id)
            WHERE message_sequence=1;
        "#,
    )?;
    Ok(())
}
