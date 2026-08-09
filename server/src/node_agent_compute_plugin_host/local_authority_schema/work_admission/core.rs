/// V8 persists one immutable source/receipt pair behind a strictly linear current head.
///
/// The head is moved first inside an explicit transaction. Its deferred foreign key then requires
/// the exact receipt before commit, while the receipt insert trigger requires that exact head. This
/// makes both half-transactions fail closed without making historical receipts point at a mutable
/// row. Numeric values are signed authorization maxima, not measured or Host-enforced ceilings.
pub(super) const WORK_ADMISSION_CORE_SCHEMA_V8: &str = r#"
CREATE TABLE compute_plugin_work_admission_receipts (
    work_admission_id                   TEXT PRIMARY KEY CHECK (
        length(work_admission_id) = 68
        AND work_admission_id GLOB 'cpw_[0-9a-f]*'
        AND work_admission_id NOT GLOB 'cpw_*[^0-9a-f]*'
    ),
    installation_id_digest             TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    clock_epoch_digest                  TEXT NOT NULL CHECK (
        length(clock_epoch_digest) = 64 AND clock_epoch_digest NOT GLOB '*[^0-9a-f]*'
    ),
    plugin_id                           TEXT NOT NULL CHECK (
        length(plugin_id) > 0 AND length(plugin_id) <= 256 AND plugin_id = trim(plugin_id)
    ),
    slot_ref                            TEXT NOT NULL CHECK (
        length(slot_ref) > 0 AND length(slot_ref) <= 512 AND slot_ref = trim(slot_ref)
    ),
    release_json                        TEXT NOT NULL CHECK (
        length(CAST(release_json AS BLOB)) > 0
        AND length(CAST(release_json AS BLOB)) <= 65536
        AND json_valid(release_json) AND json_type(release_json) = 'object'
    ),
    install_receipt_id                  TEXT NOT NULL CHECK (
        length(install_receipt_id) > 0 AND length(install_receipt_id) <= 160
    ),
    install_receipt_digest              TEXT NOT NULL CHECK (
        length(install_receipt_digest) = 64
        AND install_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    promotion_receipt_id                TEXT NOT NULL CHECK (
        length(promotion_receipt_id) > 0 AND length(promotion_receipt_id) <= 160
    ),
    promotion_receipt_digest            TEXT NOT NULL CHECK (
        length(promotion_receipt_digest) = 64
        AND promotion_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    source_digest                       TEXT NOT NULL UNIQUE CHECK (
        length(source_digest) = 64 AND source_digest NOT GLOB '*[^0-9a-f]*'
    ),
    plan_action                         TEXT NOT NULL CHECK (plan_action = 'reauthorize_existing'),
    plan_id                             TEXT NOT NULL CHECK (
        length(plan_id) > 0 AND length(plan_id) <= 256 AND plan_id = trim(plan_id)
    ),
    plan_digest                         TEXT NOT NULL CHECK (
        length(plan_digest) = 64 AND plan_digest NOT GLOB '*[^0-9a-f]*'
    ),
    signed_plan_envelope_digest         TEXT NOT NULL CHECK (
        length(signed_plan_envelope_digest) = 64
        AND signed_plan_envelope_digest NOT GLOB '*[^0-9a-f]*'
    ),
    signed_manifest_set_digest          TEXT NOT NULL CHECK (
        length(signed_manifest_set_digest) = 64
        AND signed_manifest_set_digest NOT GLOB '*[^0-9a-f]*'
    ),
    application_request_digest          TEXT NOT NULL CHECK (
        length(application_request_digest) = 64
        AND application_request_digest NOT GLOB '*[^0-9a-f]*'
    ),
    application_receipt_digest          TEXT NOT NULL CHECK (
        length(application_receipt_digest) = 64
        AND application_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    admission_bindings_digest           TEXT NOT NULL CHECK (
        length(admission_bindings_digest) = 64
        AND admission_bindings_digest NOT GLOB '*[^0-9a-f]*'
    ),
    application_inventory_revision      INTEGER NOT NULL CHECK (
        application_inventory_revision > 0
        AND application_inventory_revision <= 9007199254740991
    ),
    policy_revision                     INTEGER NOT NULL CHECK (
        policy_revision > 0 AND policy_revision <= 9007199254740991
    ),
    sharing_authorization_ref           TEXT NOT NULL CHECK (
        length(sharing_authorization_ref) > 0
        AND length(sharing_authorization_ref) <= 256
        AND sharing_authorization_ref = trim(sharing_authorization_ref)
    ),
    sharing_authorization_revision      INTEGER NOT NULL CHECK (
        sharing_authorization_revision = policy_revision
    ),
    sharing_authorization_digest        TEXT NOT NULL CHECK (
        length(sharing_authorization_digest) = 64
        AND sharing_authorization_digest NOT GLOB '*[^0-9a-f]*'
    ),
    policy_binding_receipt_digest       TEXT NOT NULL CHECK (
        length(policy_binding_receipt_digest) = 64
        AND policy_binding_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    policy_revocation_receipt_digest    TEXT NOT NULL CHECK (
        length(policy_revocation_receipt_digest) = 64
        AND policy_revocation_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    node_profile_digest                 TEXT NOT NULL CHECK (
        length(node_profile_digest) = 64 AND node_profile_digest NOT GLOB '*[^0-9a-f]*'
    ),
    manifest_catalog_revision           INTEGER NOT NULL CHECK (
        manifest_catalog_revision > 0 AND manifest_catalog_revision <= 9007199254740991
    ),
    manifest_catalog_digest             TEXT NOT NULL CHECK (
        length(manifest_catalog_digest) = 64
        AND manifest_catalog_digest NOT GLOB '*[^0-9a-f]*'
    ),
    manifest_catalog_binding_receipt_digest TEXT NOT NULL CHECK (
        length(manifest_catalog_binding_receipt_digest) = 64
        AND manifest_catalog_binding_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    keyring_bundle_revision             INTEGER NOT NULL CHECK (
        keyring_bundle_revision > 0 AND keyring_bundle_revision <= 9007199254740991
    ),
    publisher_keyring_revision          INTEGER NOT NULL CHECK (
        publisher_keyring_revision > 0 AND publisher_keyring_revision <= 9007199254740991
    ),
    publisher_keyring_digest            TEXT NOT NULL CHECK (
        length(publisher_keyring_digest) = 64
        AND publisher_keyring_digest NOT GLOB '*[^0-9a-f]*'
    ),
    control_keyring_revision            INTEGER NOT NULL CHECK (
        control_keyring_revision > 0 AND control_keyring_revision <= 9007199254740991
    ),
    control_keyring_digest              TEXT NOT NULL CHECK (
        length(control_keyring_digest) = 64
        AND control_keyring_digest NOT GLOB '*[^0-9a-f]*'
    ),
    plugin_version                      TEXT NOT NULL CHECK (
        length(plugin_version) > 0 AND length(plugin_version) <= 256
    ),
    publisher_id                        TEXT NOT NULL CHECK (
        length(publisher_id) > 0 AND length(publisher_id) <= 256
    ),
    manifest_digest                     TEXT NOT NULL CHECK (
        length(manifest_digest) = 64 AND manifest_digest NOT GLOB '*[^0-9a-f]*'
    ),
    signed_manifest_envelope_digest     TEXT NOT NULL CHECK (
        length(signed_manifest_envelope_digest) = 64
        AND signed_manifest_envelope_digest NOT GLOB '*[^0-9a-f]*'
    ),
    target_id                           TEXT NOT NULL CHECK (
        length(target_id) > 0 AND length(target_id) <= 256 AND target_id = trim(target_id)
    ),
    target_json                         TEXT NOT NULL CHECK (
        length(CAST(target_json AS BLOB)) > 0
        AND length(CAST(target_json AS BLOB)) <= 131072
        AND json_valid(target_json) AND json_type(target_json) = 'object'
    ),
    task_kinds_json                     TEXT NOT NULL CHECK (
        length(CAST(task_kinds_json AS BLOB)) > 0
        AND length(CAST(task_kinds_json AS BLOB)) <= 65536
        AND json_valid(task_kinds_json) AND json_type(task_kinds_json) = 'array'
        AND json_array_length(task_kinds_json) > 0
    ),
    host_api_protocol_id                TEXT NOT NULL CHECK (
        length(host_api_protocol_id) > 0 AND length(host_api_protocol_id) <= 256
    ),
    host_api_revision                   INTEGER NOT NULL CHECK (
        host_api_revision > 0 AND host_api_revision <= 4294967295
    ),
    entrypoint_kind                     TEXT NOT NULL CHECK (entrypoint_kind = 'sidecar'),
    entrypoint_relative_path            TEXT NOT NULL CHECK (
        length(entrypoint_relative_path) > 0 AND length(entrypoint_relative_path) <= 4096
    ),
    entrypoint_arguments_json           TEXT NOT NULL CHECK (
        length(CAST(entrypoint_arguments_json AS BLOB)) <= 65536
        AND json_valid(entrypoint_arguments_json)
        AND json_type(entrypoint_arguments_json) = 'array'
        AND json_array_length(entrypoint_arguments_json) <= 64
    ),
    entrypoint_arguments_digest         TEXT NOT NULL CHECK (
        length(entrypoint_arguments_digest) = 64
        AND entrypoint_arguments_digest NOT GLOB '*[^0-9a-f]*'
    ),
    health_check_json                   TEXT NOT NULL CHECK (
        length(CAST(health_check_json AS BLOB)) > 0
        AND length(CAST(health_check_json AS BLOB)) <= 65536
        AND json_valid(health_check_json) AND json_type(health_check_json) = 'object'
    ),
    runner_relative_path                TEXT NOT NULL CHECK (
        runner_relative_path = entrypoint_relative_path
    ),
    runner_file_digest                  TEXT NOT NULL CHECK (
        length(runner_file_digest) = 64 AND runner_file_digest NOT GLOB '*[^0-9a-f]*'
    ),
    runner_file_size_bytes              INTEGER NOT NULL CHECK (
        runner_file_size_bytes > 0 AND runner_file_size_bytes <= 9007199254740991
    ),
    runner_file_executable              INTEGER NOT NULL CHECK (runner_file_executable = 1),
    grant_ref                           TEXT NOT NULL CHECK (
        length(grant_ref) > 0 AND length(grant_ref) <= 256 AND grant_ref = trim(grant_ref)
    ),
    permission_grant_digest             TEXT NOT NULL CHECK (
        length(permission_grant_digest) = 64
        AND permission_grant_digest NOT GLOB '*[^0-9a-f]*'
    ),
    granted_permissions_json            TEXT NOT NULL CHECK (
        length(CAST(granted_permissions_json AS BLOB)) > 0
        AND length(CAST(granted_permissions_json AS BLOB)) <= 131072
        AND json_valid(granted_permissions_json)
        AND json_type(granted_permissions_json) = 'object'
    ),
    authorized_max_cpu_millicores       INTEGER NOT NULL CHECK (
        authorized_max_cpu_millicores > 0
        AND authorized_max_cpu_millicores <= 9007199254740991
    ),
    authorized_max_memory_bytes         INTEGER NOT NULL CHECK (
        authorized_max_memory_bytes > 0
        AND authorized_max_memory_bytes <= 9007199254740991
    ),
    authorized_max_vram_bytes           INTEGER NOT NULL CHECK (
        authorized_max_vram_bytes >= 0
        AND authorized_max_vram_bytes <= 9007199254740991
    ),
    authorized_max_disk_bytes           INTEGER NOT NULL CHECK (
        authorized_max_disk_bytes > 0
        AND authorized_max_disk_bytes <= 9007199254740991
    ),
    authorized_max_processes            INTEGER NOT NULL CHECK (
        authorized_max_processes > 0 AND authorized_max_processes <= 9007199254740991
    ),
    authorized_max_sidecar_uptime_seconds INTEGER NOT NULL CHECK (
        authorized_max_sidecar_uptime_seconds > 0
        AND authorized_max_sidecar_uptime_seconds <= 9007199254740991
    ),
    install_generation                  INTEGER NOT NULL CHECK (
        install_generation > 0 AND install_generation <= 9007199254740991
    ),
    activation_generation               INTEGER NOT NULL CHECK (
        activation_generation > 0 AND activation_generation <= 9007199254740991
    ),
    runtime_generation                  INTEGER NOT NULL CHECK (
        runtime_generation >= 0 AND runtime_generation <= 9007199254740991
    ),
    work_admission_generation_before    INTEGER NOT NULL CHECK (
        work_admission_generation_before >= 0
        AND work_admission_generation_before < 9007199254740991
    ),
    work_admission_generation_after     INTEGER NOT NULL CHECK (
        work_admission_generation_after = work_admission_generation_before + 1
    ),
    previous_work_admission_id          TEXT,
    previous_work_admission_receipt_digest TEXT,
    desired_presence                    TEXT NOT NULL CHECK (desired_presence = 'present'),
    desired_activation                  TEXT NOT NULL CHECK (desired_activation = 'enabled'),
    slot_phase                          TEXT NOT NULL CHECK (slot_phase = 'installed'),
    admission                           TEXT NOT NULL CHECK (admission = 'allowed'),
    runtime_phase                       TEXT NOT NULL CHECK (runtime_phase = 'stopped'),
    candidate_slot_present              INTEGER NOT NULL CHECK (candidate_slot_present = 0),
    runtime_slot_present                INTEGER NOT NULL CHECK (runtime_slot_present = 0),
    runtime_runner_digest_present       INTEGER NOT NULL CHECK (
        runtime_runner_digest_present = 0
    ),
    health_present                      INTEGER NOT NULL CHECK (health_present = 0),
    active_attempts                     INTEGER NOT NULL CHECK (active_attempts = 0),
    authority_state_revision_before     INTEGER NOT NULL CHECK (
        authority_state_revision_before > 0
        AND authority_state_revision_before < 9007199254740991
    ),
    authority_state_revision_after      INTEGER NOT NULL CHECK (
        authority_state_revision_after = authority_state_revision_before + 1
    ),
    inventory_revision_before           INTEGER NOT NULL CHECK (
        inventory_revision_before > 0 AND inventory_revision_before <= 9007199254740991
    ),
    inventory_revision_after            INTEGER NOT NULL CHECK (
        inventory_revision_after = inventory_revision_before
    ),
    inventory_digest_before             TEXT NOT NULL CHECK (
        length(inventory_digest_before) = 64
        AND inventory_digest_before NOT GLOB '*[^0-9a-f]*'
    ),
    inventory_digest_after              TEXT NOT NULL CHECK (
        inventory_digest_after = inventory_digest_before
    ),
    authority_epoch_before              INTEGER NOT NULL CHECK (
        authority_epoch_before > 0 AND authority_epoch_before < 9007199254740991
    ),
    authority_epoch_after               INTEGER NOT NULL CHECK (
        authority_epoch_after = authority_epoch_before + 1
    ),
    process_owner_epoch                 INTEGER NOT NULL CHECK (
        process_owner_epoch > 0 AND process_owner_epoch <= 9007199254740991
    ),
    trusted_time_before_ms              INTEGER NOT NULL CHECK (
        trusted_time_before_ms > 0 AND trusted_time_before_ms < 9007199254740991
    ),
    authority_updated_at_ms_before      INTEGER NOT NULL CHECK (
        authority_updated_at_ms_before > 0
        AND authority_updated_at_ms_before < 9007199254740991
    ),
    admitted_at_ms                      INTEGER NOT NULL CHECK (
        admitted_at_ms <= 9007199254740991
        AND admitted_at_ms > trusted_time_before_ms
        AND admitted_at_ms > authority_updated_at_ms_before
    ),
    source_json                         TEXT NOT NULL CHECK (
        length(CAST(source_json AS BLOB)) > 0
        AND length(CAST(source_json AS BLOB)) <= 1048576
        AND json_valid(source_json) AND json_type(source_json) = 'object'
    ),
    receipt_json                        TEXT NOT NULL CHECK (
        length(CAST(receipt_json AS BLOB)) > 0
        AND length(CAST(receipt_json AS BLOB)) <= 1048576
        AND json_valid(receipt_json) AND json_type(receipt_json) = 'object'
    ),
    receipt_digest                      TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64 AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    UNIQUE (
        work_admission_id, plugin_id, work_admission_generation_after, receipt_digest
    ),
    UNIQUE (
        installation_id_digest, plugin_id, work_admission_generation_after
    ),
    UNIQUE (work_admission_id, receipt_digest),
    FOREIGN KEY (install_receipt_id)
        REFERENCES candidate_install_receipts(install_id) ON DELETE RESTRICT,
    FOREIGN KEY (promotion_receipt_id)
        REFERENCES candidate_promotion_receipts(promotion_id) ON DELETE RESTRICT,
    FOREIGN KEY (plan_id, plan_digest)
        REFERENCES plan_applications(plan_id, plan_digest) ON DELETE RESTRICT,
    FOREIGN KEY (policy_revision)
        REFERENCES sharing_policy_binding_receipts(policy_revision) ON DELETE RESTRICT,
    FOREIGN KEY (policy_revision)
        REFERENCES sharing_policy_binding_revocation_receipts(policy_revision) ON DELETE RESTRICT,
    FOREIGN KEY (manifest_catalog_revision)
        REFERENCES manifest_catalog_binding_receipts(catalog_revision) ON DELETE RESTRICT,
    FOREIGN KEY (previous_work_admission_id, previous_work_admission_receipt_digest)
        REFERENCES compute_plugin_work_admission_receipts(work_admission_id, receipt_digest)
        ON DELETE RESTRICT,
    CHECK (
        (work_admission_generation_before = 0
         AND work_admission_generation_after = 1
         AND previous_work_admission_id IS NULL
         AND previous_work_admission_receipt_digest IS NULL)
        OR
        (work_admission_generation_before > 0
         AND previous_work_admission_id IS NOT NULL
         AND length(previous_work_admission_id) = 68
         AND previous_work_admission_id GLOB 'cpw_[0-9a-f]*'
         AND previous_work_admission_id NOT GLOB 'cpw_*[^0-9a-f]*'
         AND previous_work_admission_receipt_digest IS NOT NULL
         AND length(previous_work_admission_receipt_digest) = 64
         AND previous_work_admission_receipt_digest NOT GLOB '*[^0-9a-f]*')
    )
) WITHOUT ROWID;

CREATE TABLE compute_plugin_work_admission_heads (
    installation_id_digest             TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    plugin_id                           TEXT PRIMARY KEY CHECK (
        length(plugin_id) > 0 AND length(plugin_id) <= 256 AND plugin_id = trim(plugin_id)
    ),
    work_admission_generation          INTEGER NOT NULL CHECK (
        work_admission_generation > 0
        AND work_admission_generation <= 9007199254740991
    ),
    work_admission_id                  TEXT NOT NULL UNIQUE CHECK (
        length(work_admission_id) = 68
        AND work_admission_id GLOB 'cpw_[0-9a-f]*'
        AND work_admission_id NOT GLOB 'cpw_*[^0-9a-f]*'
    ),
    receipt_digest                     TEXT NOT NULL UNIQUE CHECK (
        length(receipt_digest) = 64 AND receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    previous_work_admission_id         TEXT,
    previous_work_admission_receipt_digest TEXT,
    updated_at_ms                      INTEGER NOT NULL CHECK (
        updated_at_ms > 0 AND updated_at_ms <= 9007199254740991
    ),
    FOREIGN KEY (
        work_admission_id, plugin_id, work_admission_generation, receipt_digest
    ) REFERENCES compute_plugin_work_admission_receipts (
        work_admission_id, plugin_id, work_admission_generation_after, receipt_digest
    ) ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED,
    CHECK (
        (work_admission_generation = 1
         AND previous_work_admission_id IS NULL
         AND previous_work_admission_receipt_digest IS NULL)
        OR
        (work_admission_generation > 1
         AND previous_work_admission_id IS NOT NULL
         AND length(previous_work_admission_id) = 68
         AND previous_work_admission_id GLOB 'cpw_[0-9a-f]*'
         AND previous_work_admission_id NOT GLOB 'cpw_*[^0-9a-f]*'
         AND previous_work_admission_receipt_digest IS NOT NULL
         AND length(previous_work_admission_receipt_digest) = 64
         AND previous_work_admission_receipt_digest NOT GLOB '*[^0-9a-f]*')
    )
) WITHOUT ROWID;
"#;
