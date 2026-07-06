    use super::migration_v84;
    use rusqlite::{params, Connection, OptionalExtension};

    fn create_minimal_billing_schema(conn: &Connection) {
        conn.execute_batch(
            r#"
            CREATE TABLE users (
              id            TEXT PRIMARY KEY,
              phone         TEXT UNIQUE,
              email         TEXT UNIQUE,
              password_hash TEXT NOT NULL,
              nickname      TEXT,
              role          TEXT NOT NULL DEFAULT 'user',
              status        TEXT NOT NULL DEFAULT 'active',
              created_at    TEXT NOT NULL,
              updated_at    TEXT NOT NULL
            );

            CREATE TABLE user_balance (
              user_id     TEXT PRIMARY KEY,
              balance_fen INTEGER NOT NULL DEFAULT 0,
              updated_at  TEXT NOT NULL
            );

            CREATE TABLE recharge_records (
              id          TEXT PRIMARY KEY,
              user_id     TEXT NOT NULL,
              amount_fen  INTEGER NOT NULL,
              method      TEXT NOT NULL DEFAULT 'manual',
              operator_id TEXT NOT NULL DEFAULT 'admin',
              note        TEXT,
              created_at  TEXT NOT NULL
            );

            CREATE TABLE billing_config (
              key        TEXT PRIMARY KEY,
              value      TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            INSERT INTO billing_config (key, value, updated_at)
            VALUES ('new_user_trial_credit_fen', '100', 'now');
            "#,
        )
        .expect("minimal billing schema should apply");
    }

    #[test]
    fn migration_backfills_trial_credit_to_30000_fen() {
        let conn = Connection::open_in_memory().expect("in-memory db should open");
        create_minimal_billing_schema(&conn);

        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, NULL, ?2, 'hash', NULL, 'user', 'active', 'now', 'now')",
            params!["usr_no_trial", "no-trial@example.com"],
        )
        .expect("active user without trial should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, NULL, ?2, 'hash', NULL, 'user', 'active', 'now', 'now')",
            params!["usr_old_trial", "old-trial@example.com"],
        )
        .expect("active user with old trial should insert");
        conn.execute(
            "INSERT INTO users (id, phone, email, password_hash, nickname, role, status, created_at, updated_at)
             VALUES (?1, NULL, ?2, 'hash', NULL, 'user', 'disabled', 'now', 'now')",
            params!["usr_disabled", "disabled@example.com"],
        )
        .expect("disabled user should insert");
        conn.execute(
            "INSERT INTO user_balance (user_id, balance_fen, updated_at)
             VALUES ('usr_old_trial', 50, 'now')",
            [],
        )
        .expect("old balance should insert");
        conn.execute(
            "INSERT INTO recharge_records (id, user_id, amount_fen, method, operator_id, note, created_at)
             VALUES ('rch_old_trial', 'usr_old_trial', 100, 'new_user_trial', 'system', 'old trial', 'now')",
            [],
        )
        .expect("old trial record should insert");

        migration_v84(&conn).expect("trial credit migration should apply");

        let config: String = conn
            .query_row(
                "SELECT value FROM billing_config WHERE key = 'new_user_trial_credit_fen'",
                [],
                |row| row.get(0),
            )
            .expect("trial config should exist");
        assert_eq!(config, "30000");

        let no_trial_balance: i64 = conn
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = 'usr_no_trial'",
                [],
                |row| row.get(0),
            )
            .expect("new trial balance should exist");
        assert_eq!(no_trial_balance, 30_000);

        let old_trial_balance: i64 = conn
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = 'usr_old_trial'",
                [],
                |row| row.get(0),
            )
            .expect("old trial balance should exist");
        assert_eq!(old_trial_balance, 29_950);

        let old_trial_grants: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(amount_fen), 0)
                   FROM recharge_records
                  WHERE user_id = 'usr_old_trial'
                    AND method = 'new_user_trial'
                    AND operator_id = 'system'",
                [],
                |row| row.get(0),
            )
            .expect("trial grants should sum");
        assert_eq!(old_trial_grants, 30_000);

        let disabled_balance: Option<i64> = conn
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = 'usr_disabled'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("disabled lookup should work");
        assert_eq!(disabled_balance, None);

        migration_v84(&conn).expect("trial credit migration should stay idempotent");
        let old_trial_balance_after_second_run: i64 = conn
            .query_row(
                "SELECT balance_fen FROM user_balance WHERE user_id = 'usr_old_trial'",
                [],
                |row| row.get(0),
            )
            .expect("old trial balance should still exist");
        assert_eq!(old_trial_balance_after_second_run, 29_950);
    }
