use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS external_pool_provider_activation_delegation_lineage
        BEFORE INSERT ON compute_external_pool_provider_activation_delegations
        WHEN NOT (
          (NEW.sequence=1
           AND NEW.predecessor_delegation_id IS NULL
           AND NEW.predecessor_delegation_digest IS NULL
           AND NOT EXISTS (
                 SELECT 1 FROM compute_external_pool_provider_activation_delegations existing
                  WHERE existing.provider_binding_id=NEW.provider_binding_id
           ))
          OR
          (NEW.sequence>1
           AND EXISTS (
                 SELECT 1 FROM compute_external_pool_provider_activation_delegations predecessor
                  WHERE predecessor.delegation_id=NEW.predecessor_delegation_id
                    AND predecessor.delegation_digest=NEW.predecessor_delegation_digest
                    AND predecessor.provider_binding_id=NEW.provider_binding_id
                    AND predecessor.sequence=NEW.sequence-1
                    AND NEW.issued_at>=predecessor.issued_at
                    AND EXISTS (
                          SELECT 1 FROM compute_external_pool_provider_activation_candidates companion
                           WHERE companion.delegation_id=predecessor.delegation_id
                             AND companion.delegation_digest=predecessor.delegation_digest
                             AND companion.provider_binding_id=predecessor.provider_binding_id
                             AND companion.sequence=predecessor.sequence
                    )
                    AND NOT EXISTS (
                          SELECT 1 FROM compute_external_pool_provider_activation_delegations later
                           WHERE later.provider_binding_id=predecessor.provider_binding_id
                             AND later.sequence>predecessor.sequence
                    )
                    AND NOT EXISTS (
                          SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                           WHERE revoked.delegation_id=predecessor.delegation_id
                             AND revoked.delegation_digest=predecessor.delegation_digest
                    )
           ))
        )
        BEGIN SELECT RAISE(ABORT,'V254 delegation requires exact unrevoked linear predecessor'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_provider_activation_candidate_lineage
        BEFORE INSERT ON compute_external_pool_provider_activation_candidates
        WHEN NOT EXISTS (
          SELECT 1 FROM compute_external_pool_provider_activation_delegations delegation
           WHERE delegation.delegation_id=NEW.delegation_id
             AND delegation.delegation_digest=NEW.delegation_digest
             AND delegation.provider_binding_id=NEW.provider_binding_id
             AND delegation.sequence=NEW.sequence
             AND NEW.checked_at>=delegation.issued_at
             AND NOT EXISTS (
                   SELECT 1 FROM compute_external_pool_provider_activation_delegations later
                    WHERE later.provider_binding_id=delegation.provider_binding_id
                      AND later.sequence>delegation.sequence
             )
             AND NOT EXISTS (
                   SELECT 1 FROM compute_external_pool_provider_activation_delegation_revocations revoked
                    WHERE revoked.delegation_id=delegation.delegation_id
                      AND revoked.delegation_digest=delegation.delegation_digest
             )
             AND (
               (NEW.sequence=1
                AND NEW.predecessor_candidate_id IS NULL
                AND NEW.predecessor_candidate_digest IS NULL
                AND delegation.predecessor_delegation_id IS NULL
                AND delegation.predecessor_delegation_digest IS NULL
                AND NOT EXISTS (
                      SELECT 1 FROM compute_external_pool_provider_activation_candidates existing
                       WHERE existing.provider_binding_id=NEW.provider_binding_id
                ))
               OR
               (NEW.sequence>1
                AND EXISTS (
                      SELECT 1
                        FROM compute_external_pool_provider_activation_candidates predecessor
                        JOIN compute_external_pool_provider_activation_delegations predecessor_delegation
                          ON predecessor_delegation.delegation_id=predecessor.delegation_id
                         AND predecessor_delegation.delegation_digest=predecessor.delegation_digest
                       WHERE predecessor.candidate_id=NEW.predecessor_candidate_id
                         AND predecessor.candidate_digest=NEW.predecessor_candidate_digest
                         AND predecessor.provider_binding_id=NEW.provider_binding_id
                         AND predecessor.sequence=NEW.sequence-1
                         AND NEW.checked_at>=predecessor.checked_at
                         AND predecessor_delegation.delegation_id=delegation.predecessor_delegation_id
                         AND predecessor_delegation.delegation_digest=delegation.predecessor_delegation_digest
                ))
             )
        )
        BEGIN SELECT RAISE(ABORT,'V254 candidate must mirror its exact delegation lineage'); END;

        CREATE TRIGGER IF NOT EXISTS external_pool_provider_activation_revocation_lineage
        BEFORE INSERT ON compute_external_pool_provider_activation_delegation_revocations
        WHEN NOT EXISTS (
          SELECT 1
            FROM compute_external_pool_provider_activation_delegations delegation
            JOIN compute_external_pool_provider_activation_candidates candidate
              ON candidate.delegation_id=delegation.delegation_id
             AND candidate.delegation_digest=delegation.delegation_digest
           WHERE delegation.delegation_id=NEW.delegation_id
             AND delegation.delegation_digest=NEW.delegation_digest
             AND candidate.candidate_id=NEW.candidate_id
             AND candidate.candidate_digest=NEW.candidate_digest
             AND delegation.provider_binding_id=NEW.provider_binding_id
             AND candidate.provider_binding_id=NEW.provider_binding_id
             AND NEW.revoked_at>=delegation.issued_at
             AND NEW.revoked_at>=candidate.checked_at
             AND NOT EXISTS (
                   SELECT 1 FROM compute_external_pool_provider_activation_delegations later
                    WHERE later.provider_binding_id=delegation.provider_binding_id
                      AND later.sequence>delegation.sequence
             )
        )
        BEGIN SELECT RAISE(ABORT,'V254 revocation requires the current exact delegation'); END;
        "#,
    )?;
    Ok(())
}
