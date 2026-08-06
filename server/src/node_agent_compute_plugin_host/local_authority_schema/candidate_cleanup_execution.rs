/// Durable expected-object topology for failed-candidate cleanup.
///
/// The execution plan is committed before the first deletion intent. Every expected object is
/// immutable, handle identity is part of its binding, and parent ordinals force children to be
/// consumed before their directories. The final object is the candidate root and binds the exact
/// non-deleted parent anchor identity. None of these rows grants path-only deletion authority.
pub(super) const CANDIDATE_CLEANUP_EXECUTION_SCHEMA_V3: &str = r#"
CREATE TABLE candidate_cleanup_execution_plans (
    cleanup_id                      TEXT PRIMARY KEY,
    candidate_token                TEXT NOT NULL UNIQUE,
    authorization_receipt_digest   TEXT NOT NULL UNIQUE CHECK (
        length(authorization_receipt_digest) = 64
        AND authorization_receipt_digest NOT GLOB '*[^0-9a-f]*'
    ),
    installation_id_digest         TEXT NOT NULL CHECK (
        length(installation_id_digest) = 64
        AND installation_id_digest NOT GLOB '*[^0-9a-f]*'
    ),
    root_identity_digest           TEXT NOT NULL CHECK (
        length(root_identity_digest) = 64
        AND root_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    candidate_parent_anchor_relative_path TEXT NOT NULL CHECK (
        candidate_parent_anchor_relative_path = 'compute-plugin/candidates'
    ),
    candidate_parent_anchor_identity_digest TEXT NOT NULL CHECK (
        length(candidate_parent_anchor_identity_digest) = 64
        AND candidate_parent_anchor_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    object_count                   INTEGER NOT NULL CHECK (
        object_count > 0 AND object_count <= 32768
    ),
    file_count                     INTEGER NOT NULL CHECK (file_count > 0),
    directory_count                INTEGER NOT NULL CHECK (directory_count > 0),
    expected_file_bytes            INTEGER NOT NULL CHECK (expected_file_bytes >= 0),
    process_owner_epoch            INTEGER NOT NULL CHECK (process_owner_epoch > 0),
    planned_at_ms                  INTEGER NOT NULL CHECK (planned_at_ms >= 0),
    plan_json                      TEXT NOT NULL CHECK (
        length(plan_json) > 0 AND length(plan_json) <= 16777216
    ),
    plan_digest                    TEXT NOT NULL UNIQUE CHECK (
        length(plan_digest) = 64
        AND plan_digest NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (object_count = file_count + directory_count),
    UNIQUE (cleanup_id, candidate_token, plan_digest),
    FOREIGN KEY (cleanup_id, candidate_token, authorization_receipt_digest)
        REFERENCES candidate_cleanup_authorizations(
            cleanup_id, candidate_token, receipt_digest
        ) ON DELETE RESTRICT
);

CREATE TABLE candidate_cleanup_expected_objects (
    cleanup_id                  TEXT NOT NULL,
    step_ordinal               INTEGER NOT NULL CHECK (
        step_ordinal >= 0 AND step_ordinal < 32768
    ),
    parent_step_ordinal        INTEGER,
    topology_depth             INTEGER NOT NULL CHECK (
        topology_depth >= 0 AND topology_depth < 32768
    ),
    object_kind               TEXT NOT NULL CHECK (object_kind IN ('file', 'directory')),
    relative_name             TEXT NOT NULL CHECK (
        length(relative_name) > 0 AND length(relative_name) <= 255
        AND relative_name <> '.'
        AND relative_name <> '..'
        AND instr(relative_name, '/') = 0
        AND instr(relative_name, '\') = 0
    ),
    relative_path             TEXT NOT NULL CHECK (
        length(relative_path) > 0 AND length(relative_path) <= 4096
        AND relative_path NOT LIKE '/%'
        AND relative_path NOT LIKE '%/'
        AND instr(relative_path, '\') = 0
        AND instr(relative_path, '//') = 0
        AND relative_path <> '.'
        AND relative_path <> '..'
        AND relative_path NOT LIKE './%'
        AND relative_path NOT LIKE '../%'
        AND relative_path NOT LIKE '%/./%'
        AND relative_path NOT LIKE '%/../%'
        AND relative_path NOT LIKE '%/.'
        AND relative_path NOT LIKE '%/..'
    ),
    relative_path_digest      TEXT NOT NULL CHECK (
        length(relative_path_digest) = 64
        AND relative_path_digest NOT GLOB '*[^0-9a-f]*'
    ),
    expected_identity_digest  TEXT NOT NULL CHECK (
        length(expected_identity_digest) = 64
        AND expected_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    expected_parent_identity_digest TEXT NOT NULL CHECK (
        length(expected_parent_identity_digest) = 64
        AND expected_parent_identity_digest NOT GLOB '*[^0-9a-f]*'
    ),
    expected_content_digest   TEXT CHECK (
        expected_content_digest IS NULL OR (
            length(expected_content_digest) = 64
            AND expected_content_digest NOT GLOB '*[^0-9a-f]*'
        )
    ),
    expected_size_bytes       INTEGER CHECK (
        expected_size_bytes IS NULL OR expected_size_bytes >= 0
    ),
    object_json               TEXT NOT NULL CHECK (
        length(object_json) > 0 AND length(object_json) <= 131072
    ),
    object_digest             TEXT NOT NULL CHECK (
        length(object_digest) = 64
        AND object_digest NOT GLOB '*[^0-9a-f]*'
    ),
    CHECK (
        (object_kind = 'file'
            AND expected_content_digest IS NOT NULL
            AND expected_size_bytes IS NOT NULL)
        OR
        (object_kind = 'directory'
            AND expected_content_digest IS NULL
            AND expected_size_bytes IS NULL)
    ),
    PRIMARY KEY (cleanup_id, step_ordinal),
    UNIQUE (cleanup_id, relative_path),
    UNIQUE (cleanup_id, parent_step_ordinal, relative_name),
    UNIQUE (cleanup_id, expected_identity_digest),
    UNIQUE (cleanup_id, object_digest),
    UNIQUE (cleanup_id, step_ordinal, object_digest),
    FOREIGN KEY (cleanup_id)
        REFERENCES candidate_cleanup_execution_plans(cleanup_id) ON DELETE RESTRICT,
    FOREIGN KEY (cleanup_id, parent_step_ordinal)
        REFERENCES candidate_cleanup_expected_objects(cleanup_id, step_ordinal)
        ON DELETE RESTRICT DEFERRABLE INITIALLY DEFERRED
);

CREATE TABLE candidate_cleanup_execution_plan_seals (
    cleanup_id       TEXT PRIMARY KEY,
    candidate_token TEXT NOT NULL UNIQUE,
    plan_digest     TEXT NOT NULL UNIQUE,
    object_count    INTEGER NOT NULL CHECK (object_count > 0),
    sealed_at_ms    INTEGER NOT NULL CHECK (sealed_at_ms >= 0),
    UNIQUE (cleanup_id, plan_digest),
    FOREIGN KEY (cleanup_id, candidate_token, plan_digest)
        REFERENCES candidate_cleanup_execution_plans(
            cleanup_id, candidate_token, plan_digest
        ) ON DELETE RESTRICT
);

CREATE TRIGGER candidate_cleanup_execution_plan_insert_fenced
BEFORE INSERT ON candidate_cleanup_execution_plans
WHEN NOT EXISTS (
    SELECT 1
    FROM authority_meta AS meta
    JOIN candidate_cleanup_authorizations AS authorization
      ON authorization.cleanup_id = NEW.cleanup_id
     AND authorization.candidate_token = NEW.candidate_token
     AND authorization.receipt_digest = NEW.authorization_receipt_digest
    JOIN candidate_staging_receipts AS staging
      ON staging.staging_id = authorization.staging_id
     AND staging.candidate_token = authorization.candidate_token
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = authorization.candidate_token
    WHERE meta.singleton = 1
      AND meta.clock_status = 'trusted'
      AND meta.trusted_time_high_water_ms = NEW.planned_at_ms
      AND meta.updated_at_ms = NEW.planned_at_ms
      AND meta.process_owner_epoch = NEW.process_owner_epoch
      AND meta.installation_id_digest = NEW.installation_id_digest
      AND staging.root_identity_digest = NEW.root_identity_digest
      AND candidate.state = 'cleanup_pending'
      AND authorization.process_owner_epoch <= NEW.process_owner_epoch
      AND authorization.authorized_at_ms < NEW.planned_at_ms
      AND NOT EXISTS (
          SELECT 1 FROM candidate_cleanup_completions AS completion
          WHERE completion.cleanup_id = NEW.cleanup_id
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup execution plan lost its pending authority fence');
END;

CREATE TRIGGER candidate_cleanup_execution_plan_update_forbidden
BEFORE UPDATE ON candidate_cleanup_execution_plans
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup execution plan is immutable');
END;

CREATE TRIGGER candidate_cleanup_execution_plan_delete_forbidden
BEFORE DELETE ON candidate_cleanup_execution_plans
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup execution plan is immutable');
END;

CREATE TRIGGER candidate_cleanup_expected_object_insert_fenced
BEFORE INSERT ON candidate_cleanup_expected_objects
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_cleanup_execution_plans AS plan
    JOIN authority_meta AS meta ON meta.singleton = 1
    JOIN candidate_cleanup_authorizations AS authorization
      ON authorization.cleanup_id = plan.cleanup_id
     AND authorization.candidate_token = plan.candidate_token
     AND authorization.receipt_digest = plan.authorization_receipt_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = plan.candidate_token
    WHERE plan.cleanup_id = NEW.cleanup_id
      AND NEW.step_ordinal < plan.object_count
      AND meta.clock_status = 'trusted'
      AND meta.installation_id_digest = plan.installation_id_digest
      AND meta.process_owner_epoch = plan.process_owner_epoch
      AND meta.trusted_time_high_water_ms = plan.planned_at_ms
      AND meta.updated_at_ms = plan.planned_at_ms
      AND candidate.state = 'cleanup_pending'
      AND NOT EXISTS (
          SELECT 1 FROM candidate_cleanup_execution_plan_seals AS seal
          WHERE seal.cleanup_id = NEW.cleanup_id
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_cleanup_completions AS completion
          WHERE completion.cleanup_id = NEW.cleanup_id
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup expected object lost its unsealed plan fence');
END;

CREATE TRIGGER candidate_cleanup_expected_object_update_forbidden
BEFORE UPDATE ON candidate_cleanup_expected_objects
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup expected object is immutable');
END;

CREATE TRIGGER candidate_cleanup_expected_object_delete_forbidden
BEFORE DELETE ON candidate_cleanup_expected_objects
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup expected object is immutable');
END;

CREATE TRIGGER candidate_cleanup_execution_plan_seal_insert_fenced
BEFORE INSERT ON candidate_cleanup_execution_plan_seals
WHEN NOT EXISTS (
    SELECT 1
    FROM candidate_cleanup_execution_plans AS plan
    JOIN authority_meta AS meta ON meta.singleton = 1
    JOIN candidate_cleanup_authorizations AS authorization
      ON authorization.cleanup_id = plan.cleanup_id
     AND authorization.candidate_token = plan.candidate_token
     AND authorization.receipt_digest = plan.authorization_receipt_digest
    JOIN candidate_owners AS candidate
      ON candidate.candidate_token = plan.candidate_token
    WHERE plan.cleanup_id = NEW.cleanup_id
      AND plan.candidate_token = NEW.candidate_token
      AND plan.plan_digest = NEW.plan_digest
      AND plan.object_count = NEW.object_count
      AND plan.planned_at_ms = NEW.sealed_at_ms
      AND meta.clock_status = 'trusted'
      AND meta.installation_id_digest = plan.installation_id_digest
      AND meta.process_owner_epoch = plan.process_owner_epoch
      AND meta.trusted_time_high_water_ms = plan.planned_at_ms
      AND meta.updated_at_ms = plan.planned_at_ms
      AND candidate.state = 'cleanup_pending'
      AND (SELECT COUNT(*) FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id) = plan.object_count
      AND (SELECT MIN(step_ordinal) FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id) = 0
      AND (SELECT MAX(step_ordinal) FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id) = plan.object_count - 1
      AND (SELECT COUNT(*) FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id AND object.object_kind = 'file')
          = plan.file_count
      AND (SELECT COUNT(*) FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id AND object.object_kind = 'directory')
          = plan.directory_count
      AND (SELECT COALESCE(SUM(expected_size_bytes), 0)
           FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id AND object.object_kind = 'file')
          = plan.expected_file_bytes
      AND (SELECT COUNT(*) FROM candidate_cleanup_expected_objects AS object
           WHERE object.cleanup_id = NEW.cleanup_id
             AND object.parent_step_ordinal IS NULL) = 1
      AND EXISTS (
          SELECT 1 FROM candidate_cleanup_expected_objects AS object
          WHERE object.cleanup_id = NEW.cleanup_id
            AND object.step_ordinal = plan.object_count - 1
            AND object.parent_step_ordinal IS NULL
            AND object.topology_depth = 0
            AND object.object_kind = 'directory'
            AND object.relative_name = authorization.candidate_token_digest
            AND object.relative_path = plan.candidate_parent_anchor_relative_path
                || '/' || authorization.candidate_token_digest
            AND object.expected_parent_identity_digest
                = plan.candidate_parent_anchor_identity_digest
            AND object.expected_identity_digest
                <> plan.candidate_parent_anchor_identity_digest
      )
      AND NOT EXISTS (
          SELECT 1
          FROM candidate_cleanup_expected_objects AS child
          LEFT JOIN candidate_cleanup_expected_objects AS parent
            ON parent.cleanup_id = child.cleanup_id
           AND parent.step_ordinal = child.parent_step_ordinal
          WHERE child.cleanup_id = NEW.cleanup_id
            AND child.parent_step_ordinal IS NOT NULL
            AND (
                parent.step_ordinal IS NULL
                OR parent.object_kind <> 'directory'
                OR parent.step_ordinal <= child.step_ordinal
                OR parent.topology_depth + 1 <> child.topology_depth
                OR child.expected_parent_identity_digest
                    <> parent.expected_identity_digest
                OR child.relative_path
                    <> parent.relative_path || '/' || child.relative_name
            )
      )
      AND NOT EXISTS (
          SELECT 1 FROM candidate_cleanup_completions AS completion
          WHERE completion.cleanup_id = NEW.cleanup_id
      )
)
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup execution plan cannot seal incomplete topology');
END;

CREATE TRIGGER candidate_cleanup_execution_plan_seal_update_forbidden
BEFORE UPDATE ON candidate_cleanup_execution_plan_seals
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup execution plan seal is immutable');
END;

CREATE TRIGGER candidate_cleanup_execution_plan_seal_delete_forbidden
BEFORE DELETE ON candidate_cleanup_execution_plan_seals
BEGIN
    SELECT RAISE(ABORT, 'candidate cleanup execution plan seal is immutable');
END;

"#;
