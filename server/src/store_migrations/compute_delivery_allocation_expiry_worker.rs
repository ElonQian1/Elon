use anyhow::Result;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub(crate) fn migration_v234(conn: &Connection) -> Result<()> {
    let transaction = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS compute_delivery_allocation_expiry_worker_checkpoint (
            checkpoint_key TEXT PRIMARY KEY NOT NULL CHECK(
                checkpoint_key='delivery_allocation_reservation_expiry_v1'),
            sweep_id TEXT NOT NULL CHECK(length(trim(sweep_id)) BETWEEN 1 AND 200),
            sweep_cutoff TEXT NOT NULL CHECK(
                length(trim(sweep_cutoff))>0
                AND julianday(sweep_cutoff) IS NOT NULL
                AND (sweep_cutoff GLOB '*Z' OR sweep_cutoff GLOB '*+00:00')),
            last_expires_at TEXT CHECK(
                last_expires_at IS NULL OR (
                    length(trim(last_expires_at))>0
                    AND julianday(last_expires_at) IS NOT NULL
                    AND (last_expires_at GLOB '*Z'
                        OR last_expires_at GLOB '*+00:00'))),
            last_reservation_id TEXT CHECK(
                last_reservation_id IS NULL
                OR length(trim(last_reservation_id)) BETWEEN 1 AND 200),
            revision INTEGER NOT NULL CHECK(
                revision BETWEEN 1 AND 9007199254740991),
            updated_at TEXT NOT NULL CHECK(
                length(trim(updated_at))>0
                AND julianday(updated_at) IS NOT NULL
                AND (updated_at GLOB '*Z' OR updated_at GLOB '*+00:00')),
            CHECK((last_expires_at IS NULL AND last_reservation_id IS NULL)
                OR (last_expires_at IS NOT NULL AND last_reservation_id IS NOT NULL)),
            CHECK(last_expires_at IS NULL
                OR julianday(last_expires_at)<=julianday(sweep_cutoff)),
            CHECK(julianday(updated_at)>=julianday(sweep_cutoff))
        );

        CREATE INDEX IF NOT EXISTS idx_compute_reservations_delivery_expiry_due
            ON compute_reservations(
                status, julianday(expires_at), expires_at, reservation_id)
            WHERE status='active';

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_expiry_checkpoint_immutable_sweep
        BEFORE UPDATE ON compute_delivery_allocation_expiry_worker_checkpoint
        WHEN NEW.checkpoint_key<>OLD.checkpoint_key
          OR NEW.sweep_id<>OLD.sweep_id
          OR NEW.sweep_cutoff<>OLD.sweep_cutoff
          OR NEW.revision<>OLD.revision+1
          OR julianday(NEW.updated_at)<julianday(OLD.updated_at)
        BEGIN
            SELECT RAISE(ABORT,
                'delivery allocation expiry checkpoint sweep is immutable');
        END;

        CREATE TRIGGER IF NOT EXISTS trg_delivery_allocation_expiry_checkpoint_forward_only
        BEFORE UPDATE ON compute_delivery_allocation_expiry_worker_checkpoint
        WHEN NOT (
            (OLD.last_expires_at IS NULL
             AND OLD.last_reservation_id IS NULL
             AND NEW.last_expires_at IS NOT NULL
             AND NEW.last_reservation_id IS NOT NULL)
            OR
            (OLD.last_expires_at IS NOT NULL
             AND OLD.last_reservation_id IS NOT NULL
             AND NEW.last_expires_at IS NOT NULL
             AND NEW.last_reservation_id IS NOT NULL
             AND (
                julianday(NEW.last_expires_at)>julianday(OLD.last_expires_at)
                OR (julianday(NEW.last_expires_at)=julianday(OLD.last_expires_at)
                    AND NEW.last_expires_at>OLD.last_expires_at)
                OR (julianday(NEW.last_expires_at)=julianday(OLD.last_expires_at)
                    AND NEW.last_expires_at=OLD.last_expires_at
                    AND NEW.last_reservation_id>OLD.last_reservation_id)
             ))
        )
        BEGIN
            SELECT RAISE(ABORT,
                'delivery allocation expiry checkpoint cursor must advance');
        END;
        "#,
    )?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_allocation_expiry_migration_is_repeatable_on_fresh_current_schema() {
        let connection = Connection::open_in_memory().expect("in-memory db should open");
        crate::store_schema::apply_migrations(&connection)
            .expect("fresh current schema should apply");

        migration_v234(&connection).expect("first repeat should succeed");
        migration_v234(&connection).expect("second repeat should succeed");

        for (object_type, object_name) in [
            (
                "table",
                "compute_delivery_allocation_expiry_worker_checkpoint",
            ),
            ("index", "idx_compute_reservations_delivery_expiry_due"),
            (
                "trigger",
                "trg_delivery_allocation_expiry_checkpoint_immutable_sweep",
            ),
            (
                "trigger",
                "trg_delivery_allocation_expiry_checkpoint_forward_only",
            ),
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type=?1 AND name=?2",
                    (object_type, object_name),
                    |row| row.get(0),
                )
                .expect("migration object should be queryable");
            assert_eq!(count, 1, "missing or duplicate {object_type} {object_name}");
        }
    }
}
