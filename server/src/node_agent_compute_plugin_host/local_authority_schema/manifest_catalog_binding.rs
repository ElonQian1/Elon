/// V6 adds one immutable local binding for each accepted manifest-catalog revision. The first row
/// may bind the legacy scalar already present in `authority_meta`; every successor must advance it.
/// This schema only admits an empty installed inventory. Non-empty catalog reauthorization remains
/// a later, separately receipted transition.
pub(super) const MANIFEST_CATALOG_BINDING_SCHEMA_V6: &str = r#"
CREATE TABLE manifest_catalog_binding_receipts (
    catalog_revision                    INTEGER PRIMARY KEY CHECK (
        catalog_revision > 0 AND catalog_revision <= 9007199254740991
    ),
    manifest_catalog_revision_before    INTEGER NOT NULL CHECK (
        manifest_catalog_revision_before >= 0
        AND manifest_catalog_revision_before <= 9007199254740991
        AND catalog_revision >= manifest_catalog_revision_before
    ),
    request_digest                      TEXT NOT NULL UNIQUE CHECK (
        length(request_digest) = 64
        AND request_digest NOT GLOB '*[^0-9a-f]*'
    ),
    request_id                          TEXT NOT NULL UNIQUE CHECK (
        length(request_id) > 0 AND length(request_id) <= 160
        AND request_id = trim(request_id)
        AND request_id NOT GLOB '*[^0-9A-Za-z._:/-]*'
    ),
    installation_id_digest              TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    catalog_json                        TEXT NOT NULL CHECK (
        length(CAST(catalog_json AS BLOB)) > 0
        AND length(CAST(catalog_json AS BLOB)) <= 4194304
        AND json_valid(catalog_json)
    ),
    catalog_digest                      TEXT NOT NULL UNIQUE CHECK (
        length(catalog_digest) = 64
        AND catalog_digest NOT GLOB '*[^0-9a-f]*'
    ),
    signed_catalog_json                 TEXT NOT NULL CHECK (
        length(CAST(signed_catalog_json AS BLOB)) > 0
        AND length(CAST(signed_catalog_json AS BLOB)) <= 4194304
        AND json_valid(signed_catalog_json)
    ),
    signed_catalog_envelope_digest      TEXT NOT NULL UNIQUE CHECK (
        length(signed_catalog_envelope_digest) = 64
        AND signed_catalog_envelope_digest NOT GLOB '*[^0-9a-f]*'
    ),
    control_signing_key_id              TEXT NOT NULL CHECK (
        length(CAST(control_signing_key_id AS BLOB)) > 0
        AND length(CAST(control_signing_key_id AS BLOB)) <= 160
        AND control_signing_key_id = trim(control_signing_key_id)
    ),
    control_signing_key_fingerprint     TEXT NOT NULL CHECK (
        length(control_signing_key_fingerprint) = 64
        AND control_signing_key_fingerprint NOT GLOB '*[^0-9a-f]*'
    ),
    signed_manifests_json               TEXT NOT NULL CHECK (
        length(CAST(signed_manifests_json AS BLOB)) > 0
        AND length(CAST(signed_manifests_json AS BLOB)) <= 4194304
        AND json_valid(signed_manifests_json)
        AND json_type(signed_manifests_json) = 'array'
    ),
    signed_manifest_set_digest          TEXT NOT NULL CHECK (
        length(signed_manifest_set_digest) = 64
        AND signed_manifest_set_digest NOT GLOB '*[^0-9a-f]*'
    ),
    catalog_entry_count                 INTEGER NOT NULL CHECK (
        catalog_entry_count >= 0 AND catalog_entry_count <= 256
    ),
    node_profile_digest                 TEXT NOT NULL CHECK (
        length(node_profile_digest) = 64
        AND node_profile_digest NOT GLOB '*[^0-9a-f]*'
    ),
    target_id                           TEXT NOT NULL CHECK (
        length(target_id) > 0 AND length(target_id) <= 256
        AND target_id = trim(target_id)
    ),
    host_api_protocol_id                TEXT NOT NULL CHECK (
        length(host_api_protocol_id) > 0 AND length(host_api_protocol_id) <= 256
        AND host_api_protocol_id = trim(host_api_protocol_id)
    ),
    host_api_revision                   INTEGER NOT NULL CHECK (
        host_api_revision > 0 AND host_api_revision <= 4294967295
    ),
    keyring_bundle_revision             INTEGER NOT NULL CHECK (
        keyring_bundle_revision > 0
        AND keyring_bundle_revision <= 9007199254740991
    ),
    publisher_keyring_revision          INTEGER NOT NULL CHECK (
        publisher_keyring_revision > 0
        AND publisher_keyring_revision <= 9007199254740991
    ),
    publisher_keyring_digest            TEXT NOT NULL CHECK (
        length(publisher_keyring_digest) = 64
        AND publisher_keyring_digest NOT GLOB '*[^0-9a-f]*'
    ),
    control_keyring_revision            INTEGER NOT NULL CHECK (
        control_keyring_revision > 0
        AND control_keyring_revision <= 9007199254740991
    ),
    control_keyring_digest              TEXT NOT NULL CHECK (
        length(control_keyring_digest) = 64
        AND control_keyring_digest NOT GLOB '*[^0-9a-f]*'
    ),
    state_revision_before               INTEGER NOT NULL CHECK (
        state_revision_before >= 0 AND state_revision_before < 9007199254740991
    ),
    state_revision_after                INTEGER NOT NULL UNIQUE CHECK (
        state_revision_after = state_revision_before + 1
    ),
    inventory_revision                  INTEGER NOT NULL CHECK (
        inventory_revision >= 0 AND inventory_revision <= 9007199254740991
    ),
    inventory_digest                    TEXT NOT NULL CHECK (
        length(inventory_digest) = 64
        AND inventory_digest NOT GLOB '*[^0-9a-f]*'
    ),
    authority_epoch_before              INTEGER NOT NULL CHECK (
        authority_epoch_before >= 0 AND authority_epoch_before < 9007199254740991
    ),
    authority_epoch_after               INTEGER NOT NULL UNIQUE CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch                 INTEGER NOT NULL CHECK (
        process_owner_epoch > 0 AND process_owner_epoch <= 9007199254740991
    ),
    trusted_time_before_ms              INTEGER NOT NULL CHECK (
        trusted_time_before_ms >= 0 AND trusted_time_before_ms < 9007199254740991
    ),
    clock_status_before                 TEXT NOT NULL CHECK (clock_status_before = 'trusted'),
    authority_updated_at_ms_before      INTEGER NOT NULL CHECK (
        authority_updated_at_ms_before >= 0
        AND authority_updated_at_ms_before < 9007199254740991
    ),
    bound_at_ms                         INTEGER NOT NULL CHECK (
        bound_at_ms <= 9007199254740991
        AND bound_at_ms > trusted_time_before_ms
        AND bound_at_ms > authority_updated_at_ms_before
    ),
    receipt_json                        TEXT NOT NULL CHECK (
        length(receipt_json) > 0 AND length(receipt_json) <= 131072
        AND json_valid(receipt_json)
    ),
    receipt_digest                      TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64
        AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    FOREIGN KEY (
        keyring_bundle_revision,
        publisher_keyring_revision, publisher_keyring_digest,
        control_keyring_revision, control_keyring_digest
    ) REFERENCES keyring_bundles (
        bundle_revision,
        publisher_revision, publisher_digest,
        control_revision, control_digest
    ) ON DELETE RESTRICT,
    FOREIGN KEY (keyring_bundle_revision)
        REFERENCES keyring_seals(bundle_revision) ON DELETE RESTRICT
);

CREATE TRIGGER manifest_catalog_binding_catalog_projection
BEFORE INSERT ON manifest_catalog_binding_receipts
WHEN NOT json_valid(NEW.catalog_json)
 OR json_extract(NEW.catalog_json, '$.schema')
        IS NOT 'elon.compute_plugin.manifest_catalog.v1'
 OR json_extract(NEW.catalog_json, '$.catalog_revision') IS NOT NEW.catalog_revision
 OR json_extract(NEW.catalog_json, '$.target_id') IS NOT NEW.target_id
 OR json_extract(NEW.catalog_json, '$.host_api_protocol_id') IS NOT NEW.host_api_protocol_id
 OR json_extract(NEW.catalog_json, '$.host_api_revision') IS NOT NEW.host_api_revision
 OR json_extract(NEW.catalog_json, '$.keyring_bundle_revision')
        IS NOT NEW.keyring_bundle_revision
 OR json_extract(NEW.catalog_json, '$.publisher_keyring.revision')
        IS NOT NEW.publisher_keyring_revision
 OR json_extract(NEW.catalog_json, '$.publisher_keyring.digest')
        IS NOT NEW.publisher_keyring_digest
 OR json_extract(NEW.catalog_json, '$.control_keyring.revision')
        IS NOT NEW.control_keyring_revision
 OR json_extract(NEW.catalog_json, '$.control_keyring.digest')
        IS NOT NEW.control_keyring_digest
 OR json_type(NEW.catalog_json, '$.entries') IS NOT 'array'
 OR json_array_length(NEW.catalog_json, '$.entries') IS NOT NEW.catalog_entry_count
 OR json_extract(NEW.signed_catalog_json, '$.schema')
        IS NOT 'elon.compute_plugin.signed_manifest_catalog.v1'
 OR json_extract(NEW.signed_catalog_json, '$.canonicalization') IS NOT 'rfc8785_jcs'
 OR json_extract(NEW.signed_catalog_json, '$.catalog_digest_algorithm') IS NOT 'sha256'
 OR json_extract(NEW.signed_catalog_json, '$.catalog_digest') IS NOT NEW.catalog_digest
 OR json_extract(NEW.signed_catalog_json, '$.signature.algorithm') IS NOT 'ed25519'
 OR json_extract(NEW.signed_catalog_json, '$.signature.signing_key_id')
        IS NOT NEW.control_signing_key_id
 OR json_extract(NEW.signed_catalog_json, '$.catalog.schema')
        IS NOT 'elon.compute_plugin.manifest_catalog.v1'
 OR json_extract(NEW.signed_catalog_json, '$.catalog.catalog_revision')
        IS NOT NEW.catalog_revision
 OR json_extract(NEW.signed_catalog_json, '$.catalog.target_id') IS NOT NEW.target_id
 OR json_extract(NEW.signed_catalog_json, '$.catalog.host_api_protocol_id')
        IS NOT NEW.host_api_protocol_id
 OR json_extract(NEW.signed_catalog_json, '$.catalog.host_api_revision')
        IS NOT NEW.host_api_revision
 OR json_extract(NEW.signed_catalog_json, '$.catalog.keyring_bundle_revision')
        IS NOT NEW.keyring_bundle_revision
 OR json_extract(NEW.signed_catalog_json, '$.catalog.publisher_keyring.revision')
        IS NOT NEW.publisher_keyring_revision
 OR json_extract(NEW.signed_catalog_json, '$.catalog.publisher_keyring.digest')
        IS NOT NEW.publisher_keyring_digest
 OR json_extract(NEW.signed_catalog_json, '$.catalog.control_keyring.revision')
        IS NOT NEW.control_keyring_revision
 OR json_extract(NEW.signed_catalog_json, '$.catalog.control_keyring.digest')
        IS NOT NEW.control_keyring_digest
 OR json_type(NEW.signed_catalog_json, '$.catalog.entries') IS NOT 'array'
 OR json_array_length(NEW.signed_catalog_json, '$.catalog.entries')
        IS NOT NEW.catalog_entry_count
 OR json_type(NEW.signed_manifests_json) IS NOT 'array'
 OR json_array_length(NEW.signed_manifests_json) IS NOT NEW.catalog_entry_count
BEGIN
    SELECT RAISE(ABORT, 'manifest catalog JSON does not match its binding projection');
END;

CREATE TRIGGER manifest_catalog_binding_revision_monotonic
BEFORE INSERT ON manifest_catalog_binding_receipts
WHEN EXISTS (
    SELECT 1 FROM manifest_catalog_binding_receipts
    WHERE catalog_revision >= NEW.catalog_revision
)
BEGIN
    SELECT RAISE(ABORT, 'manifest catalog revision cannot roll back or fork');
END;

CREATE TRIGGER manifest_catalog_binding_insert_fenced
BEFORE INSERT ON manifest_catalog_binding_receipts
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN keyring_bundles AS bundle
      ON bundle.bundle_revision = NEW.keyring_bundle_revision
     AND bundle.publisher_revision = NEW.publisher_keyring_revision
     AND bundle.publisher_digest = NEW.publisher_keyring_digest
     AND bundle.control_revision = NEW.control_keyring_revision
     AND bundle.control_digest = NEW.control_keyring_digest
    JOIN keyring_seals AS seal
      ON seal.bundle_revision = bundle.bundle_revision
    WHERE meta.singleton = 1
      AND meta.schema_version = 3
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND meta.manifest_catalog_revision = NEW.manifest_catalog_revision_before
      AND meta.node_profile_digest = NEW.node_profile_digest
      AND meta.target_id = NEW.target_id
      AND meta.host_api_protocol_id = NEW.host_api_protocol_id
      AND meta.host_api_revision = NEW.host_api_revision
      AND meta.active_bundle_revision = NEW.keyring_bundle_revision
      AND meta.publisher_keyring_revision = NEW.publisher_keyring_revision
      AND meta.publisher_keyring_digest = NEW.publisher_keyring_digest
      AND meta.control_keyring_revision = NEW.control_keyring_revision
      AND meta.control_keyring_digest = NEW.control_keyring_digest
      AND meta.state_revision = NEW.state_revision_before
      AND meta.inventory_revision = NEW.inventory_revision
      AND meta.inventory_digest = NEW.inventory_digest
      AND json_valid(meta.inventory_json)
      AND json_type(meta.inventory_json, '$.plugins') = 'array'
      AND json_array_length(meta.inventory_json, '$.plugins') = 0
      AND meta.authority_epoch = NEW.authority_epoch_before
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.process_owner_epoch > 0
      AND meta.trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND meta.clock_status = NEW.clock_status_before
      AND meta.clock_status = 'trusted'
      AND meta.updated_at_ms = NEW.authority_updated_at_ms_before
      AND meta.updated_at_ms = meta.trusted_time_high_water_ms
      AND NEW.bound_at_ms > meta.trusted_time_high_water_ms
      AND NEW.bound_at_ms > meta.updated_at_ms
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared'
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_owners WHERE state IN ('owned', 'cleanup_pending')
      )
      AND (
          (
              NOT EXISTS (SELECT 1 FROM manifest_catalog_binding_receipts)
              AND NEW.catalog_revision >= meta.manifest_catalog_revision
          )
          OR
          (
              EXISTS (SELECT 1 FROM manifest_catalog_binding_receipts)
              AND meta.manifest_catalog_revision = (
                  SELECT MAX(catalog_revision) FROM manifest_catalog_binding_receipts
              )
              AND NEW.catalog_revision > meta.manifest_catalog_revision
          )
      )
)
BEGIN
    SELECT RAISE(ABORT, 'manifest catalog binding lost its exact authority fence');
END;

CREATE TRIGGER authority_manifest_catalog_binding_receipt_required
BEFORE UPDATE OF manifest_catalog_revision ON authority_meta
WHEN NEW.manifest_catalog_revision IS NOT OLD.manifest_catalog_revision
 AND NOT EXISTS (
    SELECT 1 FROM manifest_catalog_binding_receipts AS receipt
    WHERE receipt.manifest_catalog_revision_before = OLD.manifest_catalog_revision
      AND receipt.catalog_revision = NEW.manifest_catalog_revision
      AND NEW.schema_version = OLD.schema_version
      AND receipt.installation_id_digest = OLD.installation_id_digest
      AND NEW.installation_id_digest = OLD.installation_id_digest
      AND NEW.desired_policy_revision = OLD.desired_policy_revision
      AND NEW.sharing_enabled = OLD.sharing_enabled
      AND NEW.sharing_authorization_ref IS OLD.sharing_authorization_ref
      AND NEW.sharing_authorization_revision IS OLD.sharing_authorization_revision
      AND NEW.sharing_authorization_digest IS OLD.sharing_authorization_digest
      AND receipt.node_profile_digest = OLD.node_profile_digest
      AND NEW.node_profile_digest = OLD.node_profile_digest
      AND receipt.target_id = OLD.target_id
      AND NEW.target_id = OLD.target_id
      AND receipt.host_api_protocol_id = OLD.host_api_protocol_id
      AND NEW.host_api_protocol_id = OLD.host_api_protocol_id
      AND receipt.host_api_revision = OLD.host_api_revision
      AND NEW.host_api_revision = OLD.host_api_revision
      AND receipt.keyring_bundle_revision = OLD.active_bundle_revision
      AND NEW.active_bundle_revision = OLD.active_bundle_revision
      AND receipt.publisher_keyring_revision = OLD.publisher_keyring_revision
      AND receipt.publisher_keyring_digest = OLD.publisher_keyring_digest
      AND NEW.publisher_keyring_revision = OLD.publisher_keyring_revision
      AND NEW.publisher_keyring_digest = OLD.publisher_keyring_digest
      AND receipt.control_keyring_revision = OLD.control_keyring_revision
      AND receipt.control_keyring_digest = OLD.control_keyring_digest
      AND NEW.control_keyring_revision = OLD.control_keyring_revision
      AND NEW.control_keyring_digest = OLD.control_keyring_digest
      AND receipt.state_revision_before = OLD.state_revision
      AND receipt.state_revision_after = NEW.state_revision
      AND receipt.inventory_revision = OLD.inventory_revision
      AND NEW.inventory_revision = OLD.inventory_revision
      AND receipt.inventory_digest = OLD.inventory_digest
      AND NEW.inventory_digest = OLD.inventory_digest
      AND NEW.inventory_json = OLD.inventory_json
      AND receipt.authority_epoch_before = OLD.authority_epoch
      AND receipt.authority_epoch_after = NEW.authority_epoch
      AND receipt.process_owner_epoch = OLD.process_owner_epoch
      AND NEW.process_owner_epoch = OLD.process_owner_epoch
      AND receipt.trusted_time_before_ms = OLD.trusted_time_high_water_ms
      AND NEW.trusted_time_high_water_ms = receipt.bound_at_ms
      AND receipt.clock_status_before = OLD.clock_status
      AND OLD.clock_status = 'trusted'
      AND NEW.clock_status = 'trusted'
      AND receipt.authority_updated_at_ms_before = OLD.updated_at_ms
      AND NEW.updated_at_ms = receipt.bound_at_ms
)
BEGIN
    SELECT RAISE(ABORT, 'manifest catalog revision requires an exact immutable binding receipt');
END;

CREATE TRIGGER manifest_catalog_binding_apply_authority
AFTER INSERT ON manifest_catalog_binding_receipts
BEGIN
    UPDATE authority_meta SET
        state_revision = NEW.state_revision_after,
        manifest_catalog_revision = NEW.catalog_revision,
        authority_epoch = NEW.authority_epoch_after,
        trusted_time_high_water_ms = NEW.bound_at_ms,
        clock_status = 'trusted',
        updated_at_ms = NEW.bound_at_ms
    WHERE singleton = 1
      AND schema_version = 3
      AND installation_id_digest = NEW.installation_id_digest
      AND manifest_catalog_revision = NEW.manifest_catalog_revision_before
      AND node_profile_digest = NEW.node_profile_digest
      AND target_id = NEW.target_id
      AND host_api_protocol_id = NEW.host_api_protocol_id
      AND host_api_revision = NEW.host_api_revision
      AND active_bundle_revision = NEW.keyring_bundle_revision
      AND publisher_keyring_revision = NEW.publisher_keyring_revision
      AND publisher_keyring_digest = NEW.publisher_keyring_digest
      AND control_keyring_revision = NEW.control_keyring_revision
      AND control_keyring_digest = NEW.control_keyring_digest
      AND state_revision = NEW.state_revision_before
      AND inventory_revision = NEW.inventory_revision
      AND inventory_digest = NEW.inventory_digest
      AND json_valid(inventory_json)
      AND json_type(inventory_json, '$.plugins') = 'array'
      AND json_array_length(inventory_json, '$.plugins') = 0
      AND authority_epoch = NEW.authority_epoch_before
      AND process_owner_epoch = NEW.process_owner_epoch
      AND process_owner_epoch > 0
      AND trusted_time_high_water_ms = NEW.trusted_time_before_ms
      AND clock_status = NEW.clock_status_before
      AND clock_status = 'trusted'
      AND updated_at_ms = NEW.authority_updated_at_ms_before
      AND updated_at_ms = trusted_time_high_water_ms
      AND NEW.bound_at_ms > trusted_time_high_water_ms
      AND NEW.bound_at_ms > updated_at_ms
      AND NOT EXISTS (SELECT 1 FROM fetch_claims WHERE state = 'prepared')
      AND NOT EXISTS (
          SELECT 1 FROM candidate_verification_runs WHERE state = 'prepared'
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_owners WHERE state IN ('owned', 'cleanup_pending')
      );
    SELECT RAISE(ABORT, 'manifest catalog binding authority CAS did not update exactly once')
    WHERE changes() <> 1;
END;

CREATE TRIGGER manifest_catalog_binding_receipt_update_forbidden
BEFORE UPDATE ON manifest_catalog_binding_receipts
BEGIN
    SELECT RAISE(ABORT, 'manifest catalog binding receipts are immutable');
END;

CREATE TRIGGER manifest_catalog_binding_receipt_delete_forbidden
BEFORE DELETE ON manifest_catalog_binding_receipts
BEGIN
    SELECT RAISE(ABORT, 'manifest catalog binding receipts are append-only');
END;
"#;
