use anyhow::Result;
use rusqlite::Connection;

pub(super) fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_service_actor_authorizations (
            actor_authorization_id TEXT PRIMARY KEY CHECK(
                length(trim(actor_authorization_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_revision INTEGER NOT NULL CHECK(
                actor_authorization_revision BETWEEN 1 AND 9007199254740991
            ),
            actor_authorization_schema TEXT NOT NULL CHECK(
                actor_authorization_schema='compute_federation.service_actor_authorization.v1'
            ),
            actor_authorization_digest TEXT NOT NULL UNIQUE CHECK(
                length(actor_authorization_digest)=64
                AND actor_authorization_digest NOT GLOB '*[^0-9a-f]*'
            ),
            actor_authorization_json TEXT NOT NULL CHECK(
                json_valid(actor_authorization_json)
                AND length(CAST(actor_authorization_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            provider_id TEXT NOT NULL,
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160
            ),
            service_actor_id TEXT NOT NULL CHECK(
                length(trim(service_actor_id)) BETWEEN 1 AND 160
            ),
            service_actor_kind TEXT NOT NULL CHECK(
                service_actor_kind='platform_dispatch_service'
            ),
            allowed_route_kinds_json TEXT NOT NULL CHECK(
                json_valid(allowed_route_kinds_json)
                AND json_type(allowed_route_kinds_json)='array'
            ),
            allowed_actor_phases_json TEXT NOT NULL CHECK(
                json_valid(allowed_actor_phases_json)
                AND json_type(allowed_actor_phases_json)='array'
            ),
            issued_by_user_id TEXT NOT NULL CHECK(
                length(trim(issued_by_user_id)) BETWEEN 1 AND 160
            ),
            issued_at TEXT NOT NULL,
            valid_until TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            CHECK(length(issued_at)=30 AND substr(issued_at,20,1)='.'
                AND substr(issued_at,30,1)='Z' AND julianday(issued_at) IS NOT NULL),
            CHECK(length(valid_until)=30 AND substr(valid_until,20,1)='.'
                AND substr(valid_until,30,1)='Z' AND julianday(valid_until) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(issued_at<=recorded_at AND recorded_at<valid_until),
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_dispatch_actor_receipts (
            actor_receipt_id TEXT PRIMARY KEY CHECK(
                length(trim(actor_receipt_id)) BETWEEN 1 AND 160
            ),
            actor_receipt_schema TEXT NOT NULL CHECK(
                actor_receipt_schema='compute_federation.attempt_dispatch_actor.v1'
            ),
            actor_receipt_digest TEXT NOT NULL UNIQUE CHECK(
                length(actor_receipt_digest)=64
                AND actor_receipt_digest NOT GLOB '*[^0-9a-f]*'
            ),
            actor_receipt_json TEXT NOT NULL CHECK(
                json_valid(actor_receipt_json)
                AND length(CAST(actor_receipt_json AS BLOB))<=262144
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            actor_phase TEXT NOT NULL CHECK(actor_phase IN ('dispatch','application')),
            command_id TEXT NOT NULL,
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            provider_id TEXT NOT NULL,
            provider_owner_account_id TEXT NOT NULL CHECK(
                length(trim(provider_owner_account_id)) BETWEEN 1 AND 160
            ),
            service_actor_id TEXT NOT NULL CHECK(
                length(trim(service_actor_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_id TEXT NOT NULL CHECK(
                length(trim(actor_authorization_id)) BETWEEN 1 AND 160
            ),
            actor_authorization_digest TEXT NOT NULL CHECK(
                length(actor_authorization_digest)=64
                AND actor_authorization_digest NOT GLOB '*[^0-9a-f]*'
            ),
            route_authorization_id TEXT NOT NULL,
            route_authorization_digest TEXT NOT NULL CHECK(
                length(route_authorization_digest)=64
            ),
            ack_id TEXT,
            ack_digest TEXT,
            application_id TEXT,
            application_digest TEXT,
            issued_at TEXT NOT NULL,
            valid_until TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            UNIQUE(command_id, actor_phase),
            CHECK(command_digest NOT GLOB '*[^0-9a-f]*'
                AND actor_authorization_digest NOT GLOB '*[^0-9a-f]*'
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'
                AND (ack_digest IS NULL OR ack_digest NOT GLOB '*[^0-9a-f]*')
                AND (application_digest IS NULL
                    OR application_digest NOT GLOB '*[^0-9a-f]*')),
            CHECK(
                (actor_phase='dispatch' AND ack_id IS NULL AND ack_digest IS NULL
                    AND application_id IS NULL AND application_digest IS NULL)
                OR (actor_phase='application'
                    AND ack_id IS NOT NULL AND ack_digest IS NOT NULL
                    AND application_id IS NOT NULL AND application_digest IS NOT NULL
                    AND length(trim(ack_id)) BETWEEN 1 AND 160
                    AND length(ack_digest)=64
                    AND length(trim(application_id)) BETWEEN 1 AND 160
                    AND length(application_digest)=64)
            ),
            CHECK(length(issued_at)=30 AND substr(issued_at,20,1)='.'
                AND substr(issued_at,30,1)='Z' AND julianday(issued_at) IS NOT NULL),
            CHECK(length(valid_until)=30 AND substr(valid_until,20,1)='.'
                AND substr(valid_until,30,1)='Z' AND julianday(valid_until) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(issued_at<=recorded_at AND recorded_at<valid_until),
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(actor_authorization_id)
                REFERENCES compute_service_actor_authorizations(actor_authorization_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(ack_id) REFERENCES compute_attempt_dispatch_acks(ack_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(application_id)
                REFERENCES compute_attempt_dispatch_applications(application_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
        );

        CREATE TABLE IF NOT EXISTS compute_attempt_lease_authority_bindings (
            lease_authority_id TEXT NOT NULL CHECK(
                length(trim(lease_authority_id)) BETWEEN 1 AND 160
            ),
            authority_revision INTEGER NOT NULL CHECK(
                authority_revision BETWEEN 1 AND 9007199254740991
            ),
            authority_schema TEXT NOT NULL CHECK(
                authority_schema='compute_federation.attempt_lease_authority.v1'
            ),
            lease_authority_digest TEXT NOT NULL UNIQUE CHECK(
                length(lease_authority_digest)=64
                AND lease_authority_digest NOT GLOB '*[^0-9a-f]*'
            ),
            authority_json TEXT NOT NULL CHECK(
                json_valid(authority_json)
                AND length(CAST(authority_json AS BLOB))<=524288
            ),
            canonicalization TEXT NOT NULL CHECK(canonicalization='rfc8785_jcs'),
            digest_algorithm TEXT NOT NULL CHECK(digest_algorithm='sha256'),
            non_bearer_authority_ref TEXT NOT NULL CHECK(
                length(trim(non_bearer_authority_ref)) BETWEEN 1 AND 512
            ),
            authority_hint TEXT NOT NULL CHECK(
                length(trim(authority_hint)) BETWEEN 1 AND 160
            ),
            command_id TEXT NOT NULL,
            command_digest TEXT NOT NULL CHECK(length(command_digest)=64),
            plan_id TEXT NOT NULL,
            plan_digest TEXT NOT NULL CHECK(length(plan_digest)=64),
            ack_id TEXT NOT NULL,
            ack_digest TEXT NOT NULL CHECK(length(ack_digest)=64),
            application_id TEXT NOT NULL,
            application_digest TEXT NOT NULL CHECK(length(application_digest)=64),
            application_actor_receipt_id TEXT NOT NULL,
            application_actor_receipt_digest TEXT NOT NULL CHECK(
                length(application_actor_receipt_digest)=64
            ),
            lease_id TEXT NOT NULL,
            lease_digest TEXT NOT NULL CHECK(length(lease_digest)=64),
            provider_id TEXT NOT NULL,
            executor_id TEXT NOT NULL CHECK(length(trim(executor_id)) BETWEEN 1 AND 160),
            fencing_generation INTEGER NOT NULL CHECK(
                fencing_generation BETWEEN 1 AND 9007199254740991
            ),
            route_authorization_id TEXT NOT NULL,
            route_authorization_digest TEXT NOT NULL CHECK(
                length(route_authorization_digest)=64
            ),
            authority_kind TEXT NOT NULL CHECK(
                length(trim(authority_kind)) BETWEEN 1 AND 80
            ),
            delivery_mode TEXT NOT NULL CHECK(
                length(trim(delivery_mode)) BETWEEN 1 AND 80
            ),
            audience TEXT NOT NULL CHECK(length(trim(audience)) BETWEEN 1 AND 255),
            scopes_json TEXT NOT NULL CHECK(
                json_valid(scopes_json) AND json_type(scopes_json)='array'
                AND length(CAST(scopes_json AS BLOB))<=65536
            ),
            scope_count INTEGER NOT NULL CHECK(
                scope_count BETWEEN 1 AND 9007199254740991
                AND json_array_length(scopes_json)=scope_count
            ),
            scopes_digest TEXT NOT NULL CHECK(
                length(scopes_digest)=64 AND scopes_digest NOT GLOB '*[^0-9a-f]*'
            ),
            issued_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            recorded_at TEXT NOT NULL,
            PRIMARY KEY(lease_authority_id, authority_revision),
            CHECK(command_digest NOT GLOB '*[^0-9a-f]*'
                AND plan_digest NOT GLOB '*[^0-9a-f]*'
                AND ack_digest NOT GLOB '*[^0-9a-f]*'
                AND application_digest NOT GLOB '*[^0-9a-f]*'
                AND application_actor_receipt_digest NOT GLOB '*[^0-9a-f]*'
                AND lease_digest NOT GLOB '*[^0-9a-f]*'
                AND route_authorization_digest NOT GLOB '*[^0-9a-f]*'),
            UNIQUE(application_id),
            UNIQUE(lease_id, fencing_generation),
            CHECK(length(issued_at)=30 AND substr(issued_at,20,1)='.'
                AND substr(issued_at,30,1)='Z' AND julianday(issued_at) IS NOT NULL),
            CHECK(length(expires_at)=30 AND substr(expires_at,20,1)='.'
                AND substr(expires_at,30,1)='Z' AND julianday(expires_at) IS NOT NULL),
            CHECK(length(recorded_at)=30 AND substr(recorded_at,20,1)='.'
                AND substr(recorded_at,30,1)='Z' AND julianday(recorded_at) IS NOT NULL),
            CHECK(issued_at<=recorded_at AND recorded_at<expires_at),
            FOREIGN KEY(command_id) REFERENCES compute_attempt_dispatch_commands(command_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(plan_id) REFERENCES compute_attempt_execution_plans(plan_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(ack_id) REFERENCES compute_attempt_dispatch_acks(ack_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(application_id)
                REFERENCES compute_attempt_dispatch_applications(application_id)
                ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
            FOREIGN KEY(application_actor_receipt_id)
                REFERENCES compute_attempt_dispatch_actor_receipts(actor_receipt_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(lease_id) REFERENCES compute_attempt_activations(lease_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(provider_id) REFERENCES compute_providers(provider_id)
                ON DELETE RESTRICT,
            FOREIGN KEY(route_authorization_id)
                REFERENCES compute_route_authorization_receipts(route_authorization_id)
                ON DELETE RESTRICT
        );

        CREATE TRIGGER IF NOT EXISTS trg_compute_service_actor_authorization_projection
        BEFORE INSERT ON compute_service_actor_authorizations
        WHEN json_extract(NEW.actor_authorization_json,'$.schema')
                IS NOT NEW.actor_authorization_schema
          OR json_extract(NEW.actor_authorization_json,'$.actor_authorization_id')
                IS NOT NEW.actor_authorization_id
          OR json_extract(NEW.actor_authorization_json,'$.actor_authorization_revision')
                IS NOT NEW.actor_authorization_revision
          OR json_extract(NEW.actor_authorization_json,'$.actor_authorization_digest')
                IS NOT NEW.actor_authorization_digest
          OR json_extract(NEW.actor_authorization_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.actor_authorization_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.actor_authorization_json,'$.authorization.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(NEW.actor_authorization_json,
                '$.authorization.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.actor_authorization_json,'$.authorization.service_actor_id')
                IS NOT NEW.service_actor_id
          OR json_extract(NEW.actor_authorization_json,'$.authorization.service_actor_kind')
                IS NOT NEW.service_actor_kind
          OR json_extract(NEW.actor_authorization_json,'$.authorization.allowed_route_kinds')
                IS NOT NEW.allowed_route_kinds_json
          OR json_extract(NEW.actor_authorization_json,'$.authorization.allowed_actor_phases')
                IS NOT NEW.allowed_actor_phases_json
          OR json_extract(NEW.actor_authorization_json,'$.authorization.issued_by_user_id')
                IS NOT NEW.issued_by_user_id
          OR json_extract(NEW.actor_authorization_json,'$.authorization.issued_at')
                IS NOT NEW.issued_at
          OR json_extract(NEW.actor_authorization_json,'$.authorization.valid_until')
                IS NOT NEW.valid_until
          OR json_extract(NEW.actor_authorization_json,'$.authorization.recorded_at')
                IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute service actor authorization projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_service_actor_authorization_exact_source
        BEFORE INSERT ON compute_service_actor_authorizations
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_providers provider
             WHERE provider.provider_id=NEW.provider_id
               AND provider.owner_account_id=NEW.provider_owner_account_id
               AND provider.owner_account_id=NEW.issued_by_user_id
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute service actor authorization source mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_actor_projection
        BEFORE INSERT ON compute_attempt_dispatch_actor_receipts
        WHEN json_extract(NEW.actor_receipt_json,'$.schema') IS NOT NEW.actor_receipt_schema
          OR json_extract(NEW.actor_receipt_json,'$.actor_receipt_id')
                IS NOT NEW.actor_receipt_id
          OR json_extract(NEW.actor_receipt_json,'$.actor_receipt_digest')
                IS NOT NEW.actor_receipt_digest
          OR json_extract(NEW.actor_receipt_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.actor_receipt_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.actor_receipt_json,'$.actor_phase') IS NOT NEW.actor_phase
          OR json_extract(NEW.actor_receipt_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.actor_receipt_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.actor_receipt_json,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.actor_receipt_json,'$.provider_owner_account_id')
                IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.actor_receipt_json,'$.service_actor_id') IS NOT NEW.service_actor_id
          OR json_extract(NEW.actor_receipt_json,'$.actor_authorization_id')
                IS NOT NEW.actor_authorization_id
          OR json_extract(NEW.actor_receipt_json,'$.actor_authorization_digest')
                IS NOT NEW.actor_authorization_digest
          OR json_extract(NEW.actor_receipt_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.actor_receipt_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_type(NEW.actor_receipt_json,'$.ack_id') IS NULL
          OR json_extract(NEW.actor_receipt_json,'$.ack_id') IS NOT NEW.ack_id
          OR json_type(NEW.actor_receipt_json,'$.ack_digest') IS NULL
          OR json_extract(NEW.actor_receipt_json,'$.ack_digest') IS NOT NEW.ack_digest
          OR json_type(NEW.actor_receipt_json,'$.application_id') IS NULL
          OR json_extract(NEW.actor_receipt_json,'$.application_id') IS NOT NEW.application_id
          OR json_type(NEW.actor_receipt_json,'$.application_digest') IS NULL
          OR json_extract(NEW.actor_receipt_json,'$.application_digest')
                IS NOT NEW.application_digest
          OR json_extract(NEW.actor_receipt_json,'$.issued_at') IS NOT NEW.issued_at
          OR json_extract(NEW.actor_receipt_json,'$.valid_until') IS NOT NEW.valid_until
          OR json_extract(NEW.actor_receipt_json,'$.recorded_at') IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt dispatch actor projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_authority_projection
        BEFORE INSERT ON compute_attempt_lease_authority_bindings
        WHEN json_extract(NEW.authority_json,'$.schema') IS NOT NEW.authority_schema
          OR json_extract(NEW.authority_json,'$.lease_authority_id')
                IS NOT NEW.lease_authority_id
          OR json_extract(NEW.authority_json,'$.authority_revision')
                IS NOT NEW.authority_revision
          OR json_extract(NEW.authority_json,'$.lease_authority_digest')
                IS NOT NEW.lease_authority_digest
          OR json_extract(NEW.authority_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.authority_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.authority_json,'$.command_id') IS NOT NEW.command_id
          OR json_extract(NEW.authority_json,'$.command_digest') IS NOT NEW.command_digest
          OR json_extract(NEW.authority_json,'$.plan_id') IS NOT NEW.plan_id
          OR json_extract(NEW.authority_json,'$.plan_digest') IS NOT NEW.plan_digest
          OR json_extract(NEW.authority_json,'$.ack_id') IS NOT NEW.ack_id
          OR json_extract(NEW.authority_json,'$.ack_digest') IS NOT NEW.ack_digest
          OR json_extract(NEW.authority_json,'$.application_id') IS NOT NEW.application_id
          OR json_extract(NEW.authority_json,'$.application_digest')
                IS NOT NEW.application_digest
          OR json_extract(NEW.authority_json,'$.application_actor_receipt_id')
                IS NOT NEW.application_actor_receipt_id
          OR json_extract(NEW.authority_json,'$.application_actor_receipt_digest')
                IS NOT NEW.application_actor_receipt_digest
          OR json_extract(NEW.authority_json,'$.lease_id') IS NOT NEW.lease_id
          OR json_extract(NEW.authority_json,'$.lease_digest') IS NOT NEW.lease_digest
          OR json_extract(NEW.authority_json,'$.provider_id') IS NOT NEW.provider_id
          OR json_extract(NEW.authority_json,'$.executor_id') IS NOT NEW.executor_id
          OR json_extract(NEW.authority_json,'$.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.authority_json,'$.route_authorization_id')
                IS NOT NEW.route_authorization_id
          OR json_extract(NEW.authority_json,'$.route_authorization_digest')
                IS NOT NEW.route_authorization_digest
          OR json_extract(NEW.authority_json,'$.non_bearer_authority_ref')
                IS NOT NEW.non_bearer_authority_ref
          OR json_extract(NEW.authority_json,'$.authority_hint') IS NOT NEW.authority_hint
          OR json_extract(NEW.authority_json,'$.authority_kind') IS NOT NEW.authority_kind
          OR json_extract(NEW.authority_json,'$.delivery_mode') IS NOT NEW.delivery_mode
          OR json_extract(NEW.authority_json,'$.audience') IS NOT NEW.audience
          OR json_extract(NEW.authority_json,'$.scopes') IS NOT NEW.scopes_json
          OR json_extract(NEW.authority_json,'$.scopes_digest') IS NOT NEW.scopes_digest
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.scopes_json) scope
                 WHERE scope.type!='text' OR length(trim(scope.value)) NOT BETWEEN 1 AND 160
          )
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.scopes_json) left_scope
                JOIN json_each(NEW.scopes_json) right_scope
                  ON left_scope.key<right_scope.key
                 WHERE left_scope.value>=right_scope.value
          )
          OR json_extract(NEW.authority_json,'$.issued_at') IS NOT NEW.issued_at
          OR json_extract(NEW.authority_json,'$.expires_at') IS NOT NEW.expires_at
          OR json_extract(NEW.authority_json,'$.recorded_at') IS NOT NEW.recorded_at
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt lease authority projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_actor_exact_source
        BEFORE INSERT ON compute_attempt_dispatch_actor_receipts
        WHEN NEW.actor_phase='application' AND NOT EXISTS (
            SELECT 1
              FROM compute_attempt_dispatch_commands command
              JOIN compute_route_authorization_receipts route
                ON route.route_authorization_id=NEW.route_authorization_id
               AND route.route_authorization_digest=NEW.route_authorization_digest
              JOIN compute_service_actor_authorizations authority
                ON authority.actor_authorization_id=NEW.actor_authorization_id
             WHERE command.command_id=NEW.command_id
               AND command.command_digest=NEW.command_digest
               AND command.provider_id=NEW.provider_id
               AND command.activated_by_user_id=NEW.provider_owner_account_id
               AND route.provider_id=NEW.provider_id
               AND route.provider_owner_account_id=NEW.provider_owner_account_id
               AND route.executor_id=command.executor_id
               AND route.adapter_id=command.adapter_id
               AND route.adapter_binding_digest=command.adapter_binding_digest
               AND authority.actor_authorization_digest=NEW.actor_authorization_digest
               AND authority.provider_id=NEW.provider_id
               AND authority.provider_owner_account_id=NEW.provider_owner_account_id
               AND authority.service_actor_id=NEW.service_actor_id
               AND authority.issued_at<=NEW.issued_at
               AND NEW.recorded_at<authority.valid_until
               AND NEW.recorded_at<route.cleanup_expires_at
               AND EXISTS (
                    SELECT 1 FROM json_each(authority.allowed_actor_phases_json) allowed
                     WHERE allowed.type='text' AND allowed.value=NEW.actor_phase
               )
        )
        BEGIN
            SELECT RAISE(ABORT, 'compute attempt actor receipt lacks exact authority');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_actor_no_update
        BEFORE UPDATE ON compute_attempt_dispatch_actor_receipts
        BEGIN SELECT RAISE(ABORT, 'compute attempt dispatch actors are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_dispatch_actor_no_delete
        BEFORE DELETE ON compute_attempt_dispatch_actor_receipts
        BEGIN SELECT RAISE(ABORT, 'compute attempt dispatch actors are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_authority_no_update
        BEFORE UPDATE ON compute_attempt_lease_authority_bindings
        BEGIN SELECT RAISE(ABORT, 'compute attempt lease authorities are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_attempt_lease_authority_no_delete
        BEFORE DELETE ON compute_attempt_lease_authority_bindings
        BEGIN SELECT RAISE(ABORT, 'compute attempt lease authorities are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_service_actor_authorizations_no_update
        BEFORE UPDATE ON compute_service_actor_authorizations
        BEGIN SELECT RAISE(ABORT, 'compute service actor authorizations are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_compute_service_actor_authorizations_no_delete
        BEFORE DELETE ON compute_service_actor_authorizations
        BEGIN SELECT RAISE(ABORT, 'compute service actor authorizations are append-only'); END;
        "#,
    )?;
    Ok(())
}
