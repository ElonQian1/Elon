use anyhow::Result;
use rusqlite::Connection;

pub(super) fn install(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_root_projection
        BEFORE INSERT ON compute_capacity_instruments
        WHEN json_extract(NEW.instrument_json,'$.schema') IS NOT NEW.instrument_schema
          OR json_extract(NEW.instrument_json,'$.instrument_id') IS NOT NEW.instrument_id
          OR json_extract(NEW.instrument_json,'$.instrument_revision')
                IS NOT NEW.instrument_revision
          OR json_extract(NEW.instrument_json,'$.instrument_digest')
                IS NOT NEW.instrument_digest
          OR json_extract(NEW.instrument_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.instrument_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.instrument_json,'$.sku_id') IS NOT NEW.sku_id
          OR json_extract(NEW.instrument_json,'$.sku_digest') IS NOT NEW.sku_digest
          OR json_extract(NEW.instrument_json,'$.delivery_window.binding.window_id')
                IS NOT NEW.delivery_window_id
          OR json_extract(NEW.instrument_json,'$.delivery_window.binding.window_digest')
                IS NOT NEW.delivery_window_digest
          OR json_extract(NEW.instrument_json,'$.delivery_window.starts_at_utc')
                IS NOT NEW.delivery_window_starts_at
          OR json_extract(NEW.instrument_json,'$.delivery_window.ends_at_utc')
                IS NOT NEW.delivery_window_ends_at
          OR json_extract(NEW.instrument_json,'$.contract_units')
                IS NOT json(NEW.contract_units_json)
          OR json_extract(NEW.instrument_json,'$.availability_sla_tier')
                IS NOT NEW.availability_sla_tier
          OR json_extract(NEW.instrument_json,'$.region_or_data_zone')
                IS NOT NEW.region_or_data_zone
          OR json_extract(NEW.instrument_json,'$.verification_tier')
                IS NOT NEW.verification_tier
          OR json_extract(NEW.instrument_json,'$.settlement_currency')
                IS NOT NEW.settlement_currency
          OR json_extract(NEW.instrument_json,'$.settlement_unit') IS NOT NEW.settlement_unit
          OR json_extract(NEW.instrument_json,'$.registered_by_admin_user_id')
                IS NOT NEW.registered_by_admin_user_id
          OR json_extract(NEW.instrument_json,'$.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.instrument_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.instrument_json,'$.idempotency_key') IS NOT NEW.idempotency_key
          OR json_extract(NEW.instrument_json,'$.registered_at') IS NOT NEW.registered_at
          OR json_extract(NEW.instrument_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR (SELECT COUNT(*) FROM json_each(NEW.instrument_json))<>21
          OR EXISTS (SELECT 1 FROM json_each(NEW.instrument_json) WHERE key NOT IN (
                'schema','instrument_id','instrument_revision','instrument_digest',
                'canonicalization','digest_algorithm','sku_id','sku_digest','delivery_window',
                'contract_units','availability_sla_tier','region_or_data_zone','verification_tier',
                'settlement_currency','settlement_unit','registered_by_admin_user_id',
                'confirmation','idempotency_scope','idempotency_key','registered_at','recorded_at'))
          OR EXISTS (SELECT 1 FROM json_each(NEW.contract_units_json) unit
               WHERE json_type(unit.value)<>'object'
                  OR (SELECT COUNT(*) FROM json_each(unit.value))<>3
                  OR EXISTS (SELECT 1 FROM json_each(unit.value)
                       WHERE key NOT IN ('meter','unit_size','quantity_units'))
                  OR json_type(unit.value,'$.meter')<>'text'
                  OR json_type(unit.value,'$.unit_size')<>'integer'
                  OR json_type(unit.value,'$.quantity_units')<>'integer'
                  OR length(trim(json_extract(unit.value,'$.meter'))) NOT BETWEEN 1 AND 160
                  OR json_extract(unit.value,'$.meter')<>
                        trim(json_extract(unit.value,'$.meter'))
                  OR json_extract(unit.value,'$.unit_size') NOT BETWEEN
                        1 AND 9007199254740991
                  OR json_extract(unit.value,'$.quantity_units') NOT BETWEEN
                        1 AND 9007199254740991
                  OR json_extract(unit.value,'$.quantity_units')%
                        json_extract(unit.value,'$.unit_size')<>0)
          OR EXISTS (SELECT 1
                FROM json_each(NEW.contract_units_json) unit
                JOIN json_each(NEW.contract_units_json) prior
                  ON CAST(prior.key AS INTEGER)<CAST(unit.key AS INTEGER)
               WHERE json_extract(prior.value,'$.meter')>=
                     json_extract(unit.value,'$.meter'))
          OR (SELECT COUNT(*) FROM json_each(NEW.contract_units_json))<>
             (SELECT COUNT(DISTINCT json_extract(value,'$.meter'))
                FROM json_each(NEW.contract_units_json))
          OR json_type(NEW.instrument_json,'$.delivery_window')<>'object'
          OR (SELECT COUNT(*) FROM json_each(
                NEW.instrument_json,'$.delivery_window'))<>3
          OR EXISTS (SELECT 1 FROM json_each(
                NEW.instrument_json,'$.delivery_window')
               WHERE key NOT IN ('binding','starts_at_utc','ends_at_utc'))
          OR json_type(NEW.instrument_json,'$.delivery_window.binding')<>'object'
          OR (SELECT COUNT(*) FROM json_each(
                NEW.instrument_json,'$.delivery_window.binding'))<>2
          OR EXISTS (SELECT 1 FROM json_each(
                NEW.instrument_json,'$.delivery_window.binding')
               WHERE key NOT IN ('window_id','window_digest'))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument root projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_activation_projection
        BEFORE INSERT ON compute_capacity_instrument_activations
        WHEN json_extract(NEW.activation_receipt_json,'$.schema') IS NOT NEW.activation_schema
          OR json_extract(NEW.activation_receipt_json,'$.activation_receipt_id')
                IS NOT NEW.activation_receipt_id
          OR json_extract(NEW.activation_receipt_json,'$.activation_receipt_digest')
                IS NOT NEW.activation_receipt_digest
          OR json_extract(NEW.activation_receipt_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.activation_receipt_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.activation_receipt_json,'$.instrument_id') IS NOT NEW.instrument_id
          OR json_extract(NEW.activation_receipt_json,'$.instrument_revision')
                IS NOT NEW.instrument_revision
          OR json_extract(NEW.activation_receipt_json,'$.instrument_digest')
                IS NOT NEW.instrument_digest
          OR json_extract(NEW.activation_receipt_json,'$.activated_by_admin_user_id')
                IS NOT NEW.activated_by_admin_user_id
          OR json_extract(NEW.activation_receipt_json,'$.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.activation_receipt_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.activation_receipt_json,'$.idempotency_key')
                IS NOT NEW.idempotency_key
          OR json_extract(NEW.activation_receipt_json,'$.activated_at') IS NOT NEW.activated_at
          OR json_extract(NEW.activation_receipt_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR (SELECT COUNT(*) FROM json_each(NEW.activation_receipt_json))<>14
          OR EXISTS (SELECT 1 FROM json_each(NEW.activation_receipt_json) WHERE key NOT IN (
                'schema','activation_receipt_id','activation_receipt_digest','canonicalization',
                'digest_algorithm','instrument_id','instrument_revision','instrument_digest',
                'activated_by_admin_user_id','confirmation','idempotency_scope','idempotency_key',
                'activated_at','recorded_at'))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument activation projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_retirement_projection
        BEFORE INSERT ON compute_capacity_instrument_retirements
        WHEN json_extract(NEW.retirement_receipt_json,'$.schema') IS NOT NEW.retirement_schema
          OR json_extract(NEW.retirement_receipt_json,'$.retirement_receipt_id')
                IS NOT NEW.retirement_receipt_id
          OR json_extract(NEW.retirement_receipt_json,'$.retirement_receipt_digest')
                IS NOT NEW.retirement_receipt_digest
          OR json_extract(NEW.retirement_receipt_json,'$.canonicalization')
                IS NOT NEW.canonicalization
          OR json_extract(NEW.retirement_receipt_json,'$.digest_algorithm')
                IS NOT NEW.digest_algorithm
          OR json_extract(NEW.retirement_receipt_json,'$.instrument_id') IS NOT NEW.instrument_id
          OR json_extract(NEW.retirement_receipt_json,'$.instrument_revision')
                IS NOT NEW.instrument_revision
          OR json_extract(NEW.retirement_receipt_json,'$.instrument_digest')
                IS NOT NEW.instrument_digest
          OR json_extract(NEW.retirement_receipt_json,'$.retired_by_admin_user_id')
                IS NOT NEW.retired_by_admin_user_id
          OR json_extract(NEW.retirement_receipt_json,'$.reason') IS NOT NEW.reason
          OR json_extract(NEW.retirement_receipt_json,'$.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.retirement_receipt_json,'$.idempotency_scope')
                IS NOT NEW.idempotency_scope
          OR json_extract(NEW.retirement_receipt_json,'$.idempotency_key')
                IS NOT NEW.idempotency_key
          OR json_extract(NEW.retirement_receipt_json,'$.retired_at') IS NOT NEW.retired_at
          OR json_extract(NEW.retirement_receipt_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR (SELECT COUNT(*) FROM json_each(NEW.retirement_receipt_json))<>15
          OR EXISTS (SELECT 1 FROM json_each(NEW.retirement_receipt_json) WHERE key NOT IN (
                'schema','retirement_receipt_id','retirement_receipt_digest','canonicalization',
                'digest_algorithm','instrument_id','instrument_revision','instrument_digest',
                'retired_by_admin_user_id','reason','confirmation','idempotency_scope',
                'idempotency_key','retired_at','recorded_at'))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument retirement projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_adoption_projection
        BEFORE INSERT ON compute_capacity_instrument_offer_adoptions
        WHEN json_extract(NEW.adoption_receipt_json,'$.schema') IS NOT NEW.adoption_schema
          OR json_extract(NEW.adoption_receipt_json,'$.adoption_receipt_id')
                IS NOT NEW.adoption_receipt_id
          OR json_extract(NEW.adoption_receipt_json,'$.adoption_receipt_digest')
                IS NOT NEW.adoption_receipt_digest
          OR json_extract(NEW.adoption_receipt_json,'$.canonicalization') IS NOT NEW.canonicalization
          OR json_extract(NEW.adoption_receipt_json,'$.digest_algorithm') IS NOT NEW.digest_algorithm
          OR json_extract(NEW.adoption_receipt_json,'$.instrument_id') IS NOT NEW.instrument_id
          OR json_extract(NEW.adoption_receipt_json,'$.instrument_revision')
                IS NOT NEW.instrument_revision
          OR json_extract(NEW.adoption_receipt_json,'$.instrument_digest') IS NOT NEW.instrument_digest
          OR json_extract(NEW.adoption_receipt_json,'$.offer_id') IS NOT NEW.offer_id
          OR json_extract(NEW.adoption_receipt_json,'$.offer_version') IS NOT NEW.offer_version
          OR json_extract(NEW.adoption_receipt_json,'$.offer_digest') IS NOT NEW.offer_digest
          OR json_extract(NEW.adoption_receipt_json,'$.publication_id') IS NOT NEW.publication_id
          OR json_extract(NEW.adoption_receipt_json,'$.publication_digest') IS NOT NEW.publication_digest
          OR json_extract(NEW.adoption_receipt_json,'$.adopted_by_admin_user_id')
                IS NOT NEW.adopted_by_admin_user_id
          OR json_extract(NEW.adoption_receipt_json,'$.confirmation') IS NOT NEW.confirmation
          OR json_extract(NEW.adoption_receipt_json,'$.idempotency_scope') IS NOT NEW.idempotency_scope
          OR json_extract(NEW.adoption_receipt_json,'$.idempotency_key') IS NOT NEW.idempotency_key
          OR json_extract(NEW.adoption_receipt_json,'$.adopted_at') IS NOT NEW.adopted_at
          OR json_extract(NEW.adoption_receipt_json,'$.recorded_at') IS NOT NEW.recorded_at
          OR (SELECT COUNT(*) FROM json_each(NEW.adoption_receipt_json))<>19
          OR EXISTS (SELECT 1 FROM json_each(NEW.adoption_receipt_json) WHERE key NOT IN (
                'schema','adoption_receipt_id','adoption_receipt_digest','canonicalization',
                'digest_algorithm','instrument_id','instrument_revision','instrument_digest',
                'offer_id','offer_version','offer_digest','publication_id','publication_digest',
                'adopted_by_admin_user_id','confirmation','idempotency_scope','idempotency_key',
                'adopted_at','recorded_at'))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument adoption projection mismatch');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_activation_source
        BEFORE INSERT ON compute_capacity_instrument_activations
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_capacity_instruments root
             WHERE root.instrument_id=NEW.instrument_id
               AND root.instrument_revision=NEW.instrument_revision
               AND root.instrument_digest=NEW.instrument_digest
               AND root.registered_by_admin_user_id<>NEW.activated_by_admin_user_id
               AND julianday(root.registered_at)<=julianday(NEW.activated_at))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument activation lacks four-eyes root');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_retirement_source
        BEFORE INSERT ON compute_capacity_instrument_retirements
        WHEN NOT EXISTS (
            SELECT 1 FROM compute_capacity_instruments root
              JOIN compute_capacity_instrument_activations active
                ON active.instrument_id=root.instrument_id
               AND active.instrument_revision=root.instrument_revision
               AND active.instrument_digest=root.instrument_digest
             WHERE root.instrument_id=NEW.instrument_id
               AND root.instrument_revision=NEW.instrument_revision
               AND root.instrument_digest=NEW.instrument_digest
               AND root.registered_by_admin_user_id<>NEW.retired_by_admin_user_id
               AND julianday(active.activated_at)<=julianday(NEW.retired_at))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument retirement lacks active four-eyes root');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_adoption_source
        BEFORE INSERT ON compute_capacity_instrument_offer_adoptions
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_capacity_instruments root
              JOIN compute_capacity_instrument_activations active
                ON active.instrument_id=root.instrument_id
               AND active.instrument_revision=root.instrument_revision
               AND active.instrument_digest=root.instrument_digest
              JOIN compute_offer_versions offer
                ON offer.offer_id=NEW.offer_id AND offer.offer_version=NEW.offer_version
              JOIN compute_offers current_offer
                ON current_offer.offer_id=offer.offer_id
               AND current_offer.current_offer_version=offer.offer_version
               AND current_offer.current_offer_digest=offer.offer_digest
              JOIN compute_offer_publications publication
                ON publication.publication_id=NEW.publication_id
               AND publication.publication_digest=NEW.publication_digest
             WHERE root.instrument_id=NEW.instrument_id
               AND root.instrument_revision=NEW.instrument_revision
               AND root.instrument_digest=NEW.instrument_digest
               AND root.registered_by_admin_user_id<>NEW.adopted_by_admin_user_id
               AND NOT EXISTS (SELECT 1 FROM compute_capacity_instrument_retirements retired
                    WHERE retired.instrument_id=root.instrument_id)
               AND offer.offer_digest=NEW.offer_digest AND offer.status='active'
               AND current_offer.status='active'
               AND current_offer.sku_id=root.sku_id
               AND current_offer.sku_digest=root.sku_digest
               AND json_extract(offer.offer_json,'$.schema')='compute_federation.offer.v1'
               AND json_extract(offer.offer_json,'$.offer_id')=NEW.offer_id
               AND json_extract(offer.offer_json,'$.offer_version')=NEW.offer_version
               AND json_extract(offer.offer_json,'$.offer_digest')=NEW.offer_digest
               AND json_extract(offer.offer_json,'$.status')='active'
               AND json_extract(offer.offer_json,'$.sku.sku_id')=root.sku_id
               AND json_extract(offer.offer_json,'$.sku.sku_digest')=root.sku_digest
               AND json_extract(offer.offer_json,'$.price_terms.pricing_mode')='capacity_future'
               AND json_extract(offer.offer_json,'$.price_terms.instrument_id')=root.instrument_id
               AND offer.sku_id=root.sku_id AND offer.sku_digest=root.sku_digest
               AND json_extract(offer.offer_json,'$.sku.sla_tier')=root.availability_sla_tier
               AND json_extract(offer.offer_json,'$.sku.region_or_data_zone')=
                    root.region_or_data_zone
               AND json_extract(offer.offer_json,'$.sku.verification_tier')=
                    root.verification_tier
               AND json_extract(offer.offer_json,'$.price_terms.currency')=
                    root.settlement_currency
               AND publication.offer_id=NEW.offer_id
               AND publication.active_offer_version=NEW.offer_version
               AND publication.active_offer_digest=NEW.offer_digest
               AND publication.provider_id=offer.provider_id
               AND publication.pool_id=offer.capacity_pool_id
               AND publication.provider_policy_revision=offer.provider_policy_revision
               AND publication.provider_digest=offer.provider_digest
               AND current_offer.capacity_pool_id=offer.capacity_pool_id
               AND current_offer.current_provider_policy_revision=offer.provider_policy_revision
               AND current_offer.current_provider_digest=offer.provider_digest
               AND julianday(active.activated_at)<=julianday(NEW.adopted_at)
               AND julianday(publication.published_at)<=julianday(NEW.adopted_at)
               AND EXISTS (SELECT 1 FROM json_each(offer.offer_json,'$.delivery_windows') window
                    WHERE json_extract(window.value,'$.binding.window_id')=
                            root.delivery_window_id
                      AND json_extract(window.value,'$.binding.window_digest')=
                            root.delivery_window_digest
                      AND json_extract(window.value,'$.starts_at_utc')=
                            root.delivery_window_starts_at
                      AND json_extract(window.value,'$.ends_at_utc')=
                            root.delivery_window_ends_at)
               AND (SELECT COUNT(*) FROM json_each(root.contract_units_json))=
                    (SELECT COUNT(*) FROM json_each(offer.offer_json,'$.price_terms.components'))
               AND NOT EXISTS (SELECT 1 FROM json_each(root.contract_units_json) unit
                    WHERE NOT EXISTS (
                        SELECT 1 FROM json_each(offer.offer_json,'$.price_terms.components') component
                         WHERE json_extract(component.value,'$.meter')=
                                json_extract(unit.value,'$.meter')
                           AND json_extract(component.value,'$.unit_size')=
                                json_extract(unit.value,'$.unit_size'))
                       OR NOT EXISTS (
                        SELECT 1 FROM json_each(offer.offer_json,'$.capacity') capacity
                         WHERE json_extract(capacity.value,'$.bucket.delivery_window.window_id')=
                                root.delivery_window_id
                           AND json_extract(capacity.value,'$.bucket.delivery_window.window_digest')=
                                root.delivery_window_digest
                           AND json_extract(capacity.value,'$.bucket.meter')=
                                json_extract(unit.value,'$.meter')
                           AND json_extract(capacity.value,'$.bucket.quantum_units')=
                                json_extract(unit.value,'$.unit_size')
                           AND json_extract(unit.value,'$.quantity_units')<=
                                json_extract(capacity.value,'$.reservable_units'))))
        BEGIN
            SELECT RAISE(ABORT, 'capacity instrument adoption lacks exact current authority');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_publication_source
        BEFORE INSERT ON compute_offer_publications
        WHEN NOT EXISTS (
            SELECT 1
              FROM compute_offer_versions source
              JOIN compute_offer_versions active
                ON active.offer_id=source.offer_id
               AND active.offer_version=source.offer_version+1
              JOIN compute_offers current_offer
                ON current_offer.offer_id=active.offer_id
               AND current_offer.current_offer_version=active.offer_version
               AND current_offer.current_offer_digest=active.offer_digest
             WHERE source.offer_id=NEW.offer_id
               AND source.offer_version=NEW.source_offer_version
               AND source.offer_digest=NEW.source_offer_digest
               AND source.status='draft'
               AND active.offer_version=NEW.active_offer_version
               AND active.offer_digest=NEW.active_offer_digest
               AND active.status='active'
               AND active.provider_id=NEW.provider_id
               AND active.capacity_pool_id=NEW.pool_id
               AND active.provider_policy_revision=NEW.provider_policy_revision
               AND active.provider_digest=NEW.provider_digest
               AND current_offer.status='active'
               AND current_offer.provider_id=NEW.provider_id
               AND current_offer.capacity_pool_id=NEW.pool_id
               AND current_offer.current_provider_policy_revision=
                    NEW.provider_policy_revision
               AND current_offer.current_provider_digest=NEW.provider_digest
               AND NEW.published_at=NEW.created_at)
        BEGIN
            SELECT RAISE(ABORT, 'Offer publication lacks exact draft to active source');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_publication_no_replace
        BEFORE INSERT ON compute_offer_publications
        WHEN EXISTS (
            SELECT 1 FROM compute_offer_publications old
             WHERE old.publication_id=NEW.publication_id
                OR old.offer_id=NEW.offer_id
                OR old.publication_digest=NEW.publication_digest
                OR (old.idempotency_scope=NEW.idempotency_scope
                    AND old.idempotency_key=NEW.idempotency_key))
        BEGIN
            SELECT RAISE(ABORT, 'Offer publication cannot replace history');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_root_no_replace
        BEFORE INSERT ON compute_capacity_instruments
        WHEN EXISTS (SELECT 1 FROM compute_capacity_instruments old
              WHERE old.instrument_id=NEW.instrument_id
                 OR old.instrument_digest=NEW.instrument_digest
                 OR (old.idempotency_scope=NEW.idempotency_scope
                     AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'capacity instrument root cannot replace history'); END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_activation_no_replace
        BEFORE INSERT ON compute_capacity_instrument_activations
        WHEN EXISTS (SELECT 1 FROM compute_capacity_instrument_activations old
              WHERE old.activation_receipt_id=NEW.activation_receipt_id
                 OR old.activation_receipt_digest=NEW.activation_receipt_digest
                 OR old.instrument_id=NEW.instrument_id
                 OR (old.idempotency_scope=NEW.idempotency_scope
                     AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'capacity instrument activation cannot replace history'); END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_retirement_no_replace
        BEFORE INSERT ON compute_capacity_instrument_retirements
        WHEN EXISTS (SELECT 1 FROM compute_capacity_instrument_retirements old
              WHERE old.retirement_receipt_id=NEW.retirement_receipt_id
                 OR old.retirement_receipt_digest=NEW.retirement_receipt_digest
                 OR old.instrument_id=NEW.instrument_id
                 OR (old.idempotency_scope=NEW.idempotency_scope
                     AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'capacity instrument retirement cannot replace history'); END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_adoption_no_replace
        BEFORE INSERT ON compute_capacity_instrument_offer_adoptions
        WHEN EXISTS (SELECT 1 FROM compute_capacity_instrument_offer_adoptions old
              WHERE old.adoption_receipt_id=NEW.adoption_receipt_id
                 OR old.adoption_receipt_digest=NEW.adoption_receipt_digest
                 OR old.publication_id=NEW.publication_id
                 OR (old.offer_id=NEW.offer_id AND old.offer_version=NEW.offer_version)
                 OR (old.idempotency_scope=NEW.idempotency_scope
                     AND old.idempotency_key=NEW.idempotency_key))
        BEGIN SELECT RAISE(ABORT, 'capacity instrument adoption cannot replace history'); END;

        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_root_no_update
        BEFORE UPDATE ON compute_capacity_instruments
        BEGIN SELECT RAISE(ABORT, 'capacity instrument roots are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_root_no_delete
        BEFORE DELETE ON compute_capacity_instruments
        BEGIN SELECT RAISE(ABORT, 'capacity instrument roots are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_activation_no_update
        BEFORE UPDATE ON compute_capacity_instrument_activations
        BEGIN SELECT RAISE(ABORT, 'capacity instrument activations are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_activation_no_delete
        BEFORE DELETE ON compute_capacity_instrument_activations
        BEGIN SELECT RAISE(ABORT, 'capacity instrument activations are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_retirement_no_update
        BEFORE UPDATE ON compute_capacity_instrument_retirements
        BEGIN SELECT RAISE(ABORT, 'capacity instrument retirements are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_retirement_no_delete
        BEFORE DELETE ON compute_capacity_instrument_retirements
        BEGIN SELECT RAISE(ABORT, 'capacity instrument retirements are append-only'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_adoption_no_update
        BEFORE UPDATE ON compute_capacity_instrument_offer_adoptions
        BEGIN SELECT RAISE(ABORT, 'capacity instrument adoptions are immutable'); END;
        CREATE TRIGGER IF NOT EXISTS trg_capacity_instrument_adoption_no_delete
        BEFORE DELETE ON compute_capacity_instrument_offer_adoptions
        BEGIN SELECT RAISE(ABORT, 'capacity instrument adoptions are append-only'); END;
        "#,
    )?;
    Ok(())
}
