use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(connection: &Connection) -> Result<()> {
    install_version_guards(connection)?;
    install_head_guards(connection)?;
    Ok(())
}

fn install_version_guards(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS v283_interactive_authority_version_exact_projection;
        CREATE TRIGGER v283_interactive_authority_version_exact_projection
        BEFORE INSERT ON compute_interactive_desktop_authority_versions
        WHEN json_extract(NEW.authority_record_json,'$.schema')
                IS NOT NEW.authority_record_schema
          OR json_extract(NEW.authority_record_json,'$.record_digest')
                IS NOT NEW.authority_record_digest
          OR (SELECT COUNT(*) FROM json_each(NEW.authority_record_json))!=10
          OR EXISTS (
                SELECT 1 FROM json_each(NEW.authority_record_json) field
                 WHERE field.key NOT IN (
                    'schema','record_digest','request','profile','reservation','session',
                    'host_lease','viewer_grant','media_epoch','control_epoch'
                 )
          )
          OR json_type(NEW.authority_record_json,'$.request') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.profile') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.reservation') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.session') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.host_lease') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.viewer_grant') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.media_epoch') IS NOT 'object'
          OR json_type(NEW.authority_record_json,'$.control_epoch') IS NOT 'object'
          OR json_extract(NEW.authority_record_json,'$.session.session_id')
                IS NOT NEW.session_id
          OR json_extract(NEW.authority_record_json,'$.session.session_root_digest')
                IS NOT NEW.session_root_digest
          OR json_extract(NEW.authority_record_json,'$.session.session_revision')
                IS NOT NEW.session_revision
          OR json_extract(NEW.authority_record_json,'$.session.session_digest')
                IS NOT NEW.session_digest
          OR json_extract(NEW.authority_record_json,'$.session.state')
                IS NOT NEW.session_state
          OR json_extract(
                NEW.authority_record_json,
                '$.session.session_reservation.session_reservation_id'
             ) IS NOT NEW.session_reservation_id
          OR json_extract(
                NEW.authority_record_json,
                '$.session.session_reservation.session_reservation_revision'
             ) IS NOT NEW.session_reservation_revision
          OR json_extract(
                NEW.authority_record_json,
                '$.session.session_reservation.session_reservation_digest'
             ) IS NOT NEW.session_reservation_digest
          OR json_extract(NEW.authority_record_json,'$.session.binding.binding_digest')
                IS NOT NEW.binding_digest
          OR json_extract(NEW.authority_record_json,'$.session.binding.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(
                NEW.authority_record_json,'$.session.binding.provider_policy_revision'
             ) IS NOT NEW.provider_policy_revision
          OR json_extract(NEW.authority_record_json,'$.session.binding.provider_digest')
                IS NOT NEW.provider_digest
          OR json_extract(
                NEW.authority_record_json,'$.session.binding.provider_owner_account_id'
             ) IS NOT NEW.provider_owner_account_id
          OR json_extract(NEW.authority_record_json,'$.session.binding.consumer_account_id')
                IS NOT NEW.consumer_account_id
          OR json_extract(NEW.authority_record_json,'$.reservation.session_id')
                IS NOT NEW.session_id
          OR json_extract(
                NEW.authority_record_json,
                '$.reservation.session_reservation.session_reservation_id'
             ) IS NOT NEW.session_reservation_id
          OR json_extract(
                NEW.authority_record_json,
                '$.reservation.session_reservation.session_reservation_revision'
             ) IS NOT NEW.session_reservation_revision
          OR json_extract(
                NEW.authority_record_json,
                '$.reservation.session_reservation.session_reservation_digest'
             ) IS NOT NEW.session_reservation_digest
          OR json_extract(NEW.authority_record_json,'$.reservation.binding.binding_digest')
                IS NOT NEW.binding_digest
          OR json_extract(NEW.authority_record_json,'$.reservation.binding.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(
                NEW.authority_record_json,'$.reservation.binding.provider_policy_revision'
             ) IS NOT NEW.provider_policy_revision
          OR json_extract(NEW.authority_record_json,'$.reservation.binding.provider_digest')
                IS NOT NEW.provider_digest
          OR json_extract(
                NEW.authority_record_json,'$.reservation.binding.provider_owner_account_id'
             ) IS NOT NEW.provider_owner_account_id
          OR json_extract(
                NEW.authority_record_json,'$.reservation.binding.consumer_account_id'
             ) IS NOT NEW.consumer_account_id
          OR json_extract(NEW.authority_record_json,'$.host_lease.host_lease_id')
                IS NOT NEW.host_lease_id
          OR json_extract(NEW.authority_record_json,'$.host_lease.host_lease_digest')
                IS NOT NEW.host_lease_digest
          OR json_extract(NEW.authority_record_json,'$.host_lease.session_id')
                IS NOT NEW.session_id
          OR json_extract(
                NEW.authority_record_json,'$.host_lease.session_reservation_digest'
             ) IS NOT NEW.session_reservation_digest
          OR json_extract(NEW.authority_record_json,'$.host_lease.binding_digest')
                IS NOT NEW.binding_digest
          OR json_extract(NEW.authority_record_json,'$.host_lease.provider_id')
                IS NOT NEW.provider_id
          OR json_extract(NEW.authority_record_json,'$.host_lease.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(
                NEW.authority_record_json,'$.host_lease.selected_surface.selection_digest'
             ) IS NOT NEW.selected_surface_digest
          OR json_extract(NEW.authority_record_json,'$.viewer_grant.viewer_grant_id')
                IS NOT NEW.viewer_grant_id
          OR json_extract(NEW.authority_record_json,'$.viewer_grant.viewer_grant_digest')
                IS NOT NEW.viewer_grant_digest
          OR json_extract(NEW.authority_record_json,'$.viewer_grant.grant_generation')
                IS NOT NEW.viewer_grant_generation
          OR json_extract(NEW.authority_record_json,'$.viewer_grant.session_id')
                IS NOT NEW.session_id
          OR json_extract(
                NEW.authority_record_json,'$.viewer_grant.session_reservation_digest'
             ) IS NOT NEW.session_reservation_digest
          OR json_extract(NEW.authority_record_json,'$.viewer_grant.binding_digest')
                IS NOT NEW.binding_digest
          OR json_extract(NEW.authority_record_json,'$.viewer_grant.consumer_account_id')
                IS NOT NEW.consumer_account_id
          OR json_extract(
                NEW.authority_record_json,'$.viewer_grant.viewer_transport_identity_digest'
             ) IS NOT NEW.viewer_transport_identity_digest
          OR json_extract(NEW.authority_record_json,'$.media_epoch.media_epoch_id')
                IS NOT NEW.media_epoch_id
          OR json_extract(NEW.authority_record_json,'$.media_epoch.media_epoch_digest')
                IS NOT NEW.media_epoch_digest
          OR json_extract(NEW.authority_record_json,'$.media_epoch.epoch_sequence')
                IS NOT NEW.media_epoch_sequence
          OR json_extract(NEW.authority_record_json,'$.media_epoch.session_id')
                IS NOT NEW.session_id
          OR json_extract(
                NEW.authority_record_json,'$.media_epoch.session_reservation_digest'
             ) IS NOT NEW.session_reservation_digest
          OR json_extract(NEW.authority_record_json,'$.media_epoch.binding_digest')
                IS NOT NEW.binding_digest
          OR json_extract(NEW.authority_record_json,'$.media_epoch.host_lease_id')
                IS NOT NEW.host_lease_id
          OR json_extract(NEW.authority_record_json,'$.media_epoch.viewer_grant_id')
                IS NOT NEW.viewer_grant_id
          OR json_extract(
                NEW.authority_record_json,'$.media_epoch.viewer_grant_generation'
             ) IS NOT NEW.viewer_grant_generation
          OR json_extract(
                NEW.authority_record_json,'$.media_epoch.viewer_transport_identity_digest'
             ) IS NOT NEW.viewer_transport_identity_digest
          OR json_extract(NEW.authority_record_json,'$.media_epoch.selected_surface_digest')
                IS NOT NEW.selected_surface_digest
          OR json_extract(NEW.authority_record_json,'$.media_epoch.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.authority_record_json,'$.control_epoch.control_epoch_id')
                IS NOT NEW.control_epoch_id
          OR json_extract(NEW.authority_record_json,'$.control_epoch.control_epoch_digest')
                IS NOT NEW.control_epoch_digest
          OR json_extract(NEW.authority_record_json,'$.control_epoch.epoch_sequence')
                IS NOT NEW.control_epoch_sequence
          OR json_extract(NEW.authority_record_json,'$.control_epoch.session_id')
                IS NOT NEW.session_id
          OR json_extract(
                NEW.authority_record_json,'$.control_epoch.session_reservation_digest'
             ) IS NOT NEW.session_reservation_digest
          OR json_extract(NEW.authority_record_json,'$.control_epoch.binding_digest')
                IS NOT NEW.binding_digest
          OR json_extract(NEW.authority_record_json,'$.control_epoch.host_lease_id')
                IS NOT NEW.host_lease_id
          OR json_extract(NEW.authority_record_json,'$.control_epoch.viewer_grant_id')
                IS NOT NEW.viewer_grant_id
          OR json_extract(
                NEW.authority_record_json,'$.control_epoch.viewer_grant_generation'
             ) IS NOT NEW.viewer_grant_generation
          OR json_extract(NEW.authority_record_json,'$.control_epoch.media_epoch_id')
                IS NOT NEW.media_epoch_id
          OR json_extract(NEW.authority_record_json,'$.control_epoch.media_epoch_digest')
                IS NOT NEW.media_epoch_digest
          OR json_extract(NEW.authority_record_json,'$.control_epoch.media_epoch_sequence')
                IS NOT NEW.media_epoch_sequence
          OR json_extract(
                NEW.authority_record_json,'$.control_epoch.viewer_transport_identity_digest'
             ) IS NOT NEW.viewer_transport_identity_digest
          OR json_extract(NEW.authority_record_json,'$.control_epoch.selected_surface_digest')
                IS NOT NEW.selected_surface_digest
          OR json_extract(NEW.authority_record_json,'$.control_epoch.fencing_generation')
                IS NOT NEW.fencing_generation
          OR json_extract(NEW.authority_record_json,'$.session.authority_head.host_lease_id')
                IS NOT NEW.host_lease_id
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.host_lease_digest'
             ) IS NOT NEW.host_lease_digest
          OR json_extract(NEW.authority_record_json,'$.session.authority_head.viewer_grant_id')
                IS NOT NEW.viewer_grant_id
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.viewer_grant_digest'
             ) IS NOT NEW.viewer_grant_digest
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.viewer_grant_generation'
             ) IS NOT NEW.viewer_grant_generation
          OR json_extract(NEW.authority_record_json,'$.session.authority_head.media_epoch_id')
                IS NOT NEW.media_epoch_id
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.media_epoch_digest'
             ) IS NOT NEW.media_epoch_digest
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.media_epoch_sequence'
             ) IS NOT NEW.media_epoch_sequence
          OR json_extract(NEW.authority_record_json,'$.session.authority_head.control_epoch_id')
                IS NOT NEW.control_epoch_id
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.control_epoch_digest'
             ) IS NOT NEW.control_epoch_digest
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.control_epoch_sequence'
             ) IS NOT NEW.control_epoch_sequence
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.selected_surface_digest'
             ) IS NOT NEW.selected_surface_digest
          OR json_extract(
                NEW.authority_record_json,
                '$.session.authority_head.viewer_transport_identity_digest'
             ) IS NOT NEW.viewer_transport_identity_digest
          OR json_extract(
                NEW.authority_record_json,'$.session.authority_head.fencing_generation'
             ) IS NOT NEW.fencing_generation
        BEGIN
            SELECT RAISE(ABORT,'V283 authority record projection is not exact');
        END;

        DROP TRIGGER IF EXISTS v283_interactive_authority_version_current_source;
        CREATE TRIGGER v283_interactive_authority_version_current_source
        BEFORE INSERT ON compute_interactive_desktop_authority_versions
        WHEN NEW.session_state='active' AND NOT EXISTS (
            SELECT 1
              FROM compute_providers provider
              JOIN compute_provider_versions version
                ON version.provider_id=provider.provider_id
               AND version.policy_revision=provider.current_policy_revision
               AND version.provider_digest=provider.current_provider_digest
             WHERE provider.provider_id=NEW.provider_id
               AND provider.current_policy_revision=NEW.provider_policy_revision
               AND provider.current_provider_digest=NEW.provider_digest
               AND provider.owner_account_id=NEW.provider_owner_account_id
        )
        BEGIN
            SELECT RAISE(ABORT,'V283 authority record lacks exact current Provider');
        END;

        DROP TRIGGER IF EXISTS v283_interactive_authority_version_linear_insert;
        CREATE TRIGGER v283_interactive_authority_version_linear_insert
        BEFORE INSERT ON compute_interactive_desktop_authority_versions
        WHEN (
            NEW.session_revision=1
            AND (
                EXISTS (
                    SELECT 1 FROM compute_interactive_desktop_authority_versions old
                     WHERE old.session_id=NEW.session_id
                )
                OR EXISTS (
                    SELECT 1 FROM compute_interactive_desktop_authority_heads head
                     WHERE head.session_id=NEW.session_id
                )
            )
        ) OR (
            NEW.session_revision>1
            AND NOT EXISTS (
                SELECT 1 FROM compute_interactive_desktop_authority_heads head
                 WHERE head.session_id=NEW.session_id
                   AND head.session_root_digest=NEW.session_root_digest
                   AND head.current_session_revision=NEW.session_revision-1
                   AND head.is_terminal=0
            )
        )
        BEGIN
            SELECT RAISE(ABORT,'V283 authority version is not the next live Session revision');
        END;

        DROP TRIGGER IF EXISTS v283_interactive_authority_version_no_update;
        CREATE TRIGGER v283_interactive_authority_version_no_update
        BEFORE UPDATE ON compute_interactive_desktop_authority_versions
        BEGIN
            SELECT RAISE(ABORT,'V283 authority versions are immutable');
        END;

        DROP TRIGGER IF EXISTS v283_interactive_authority_version_no_delete;
        CREATE TRIGGER v283_interactive_authority_version_no_delete
        BEFORE DELETE ON compute_interactive_desktop_authority_versions
        BEGIN
            SELECT RAISE(ABORT,'V283 authority versions cannot be deleted');
        END;
        "#,
    )?;
    Ok(())
}

fn install_head_guards(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        DROP TRIGGER IF EXISTS v283_interactive_authority_head_initial_exact;
        CREATE TRIGGER v283_interactive_authority_head_initial_exact
        BEFORE INSERT ON compute_interactive_desktop_authority_heads
        WHEN NEW.current_session_revision!=1
          OR NEW.created_at_ms!=NEW.updated_at_ms
          OR NOT EXISTS (
                SELECT 1 FROM compute_interactive_desktop_authority_versions version
                 WHERE version.session_id=NEW.session_id
                   AND version.session_root_digest=NEW.session_root_digest
                   AND version.session_revision=1
                   AND version.session_digest=NEW.current_session_digest
                   AND version.authority_record_digest=
                        NEW.current_authority_record_digest
                   AND version.session_state=NEW.session_state
                   AND version.is_terminal=NEW.is_terminal
                   AND version.recorded_at_ms=NEW.created_at_ms
          )
        BEGIN
            SELECT RAISE(ABORT,'V283 initial authority head lacks exact revision one');
        END;

        DROP TRIGGER IF EXISTS v283_interactive_authority_head_linear_update;
        CREATE TRIGGER v283_interactive_authority_head_linear_update
        BEFORE UPDATE ON compute_interactive_desktop_authority_heads
        WHEN OLD.is_terminal=1
          OR NEW.session_id IS NOT OLD.session_id
          OR NEW.session_root_digest IS NOT OLD.session_root_digest
          OR NEW.created_at_ms IS NOT OLD.created_at_ms
          OR NEW.current_session_revision!=OLD.current_session_revision+1
          OR NEW.current_session_digest=OLD.current_session_digest
          OR NEW.current_authority_record_digest=OLD.current_authority_record_digest
          OR NEW.updated_at_ms<=OLD.updated_at_ms
          OR NOT EXISTS (
                SELECT 1 FROM compute_interactive_desktop_authority_versions version
                 WHERE version.session_id=NEW.session_id
                   AND version.session_root_digest=OLD.session_root_digest
                   AND version.session_revision=NEW.current_session_revision
                   AND version.session_digest=NEW.current_session_digest
                   AND version.authority_record_digest=
                        NEW.current_authority_record_digest
                   AND version.session_state=NEW.session_state
                   AND version.is_terminal=NEW.is_terminal
                   AND version.recorded_at_ms=NEW.updated_at_ms
          )
        BEGIN
            SELECT RAISE(ABORT,'V283 authority head transition is not exact and linear');
        END;

        DROP TRIGGER IF EXISTS v283_interactive_authority_head_no_delete;
        CREATE TRIGGER v283_interactive_authority_head_no_delete
        BEFORE DELETE ON compute_interactive_desktop_authority_heads
        BEGIN
            SELECT RAISE(ABORT,'V283 authority heads cannot be deleted');
        END;
        "#,
    )?;
    Ok(())
}
