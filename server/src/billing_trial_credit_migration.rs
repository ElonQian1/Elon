use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migration_v84(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        INSERT INTO billing_config (key, value, updated_at)
        VALUES ('new_user_trial_credit_fen', '30000', datetime('now'))
        ON CONFLICT(key) DO UPDATE SET
          value = '30000',
          updated_at = datetime('now')
        WHERE TRIM(billing_config.value) = '100';

        INSERT OR IGNORE INTO user_balance (user_id, balance_fen, updated_at)
        SELECT id, 0, datetime('now')
          FROM users
         WHERE status = 'active';

        UPDATE user_balance
           SET balance_fen = balance_fen + (
                 SELECT 30000 - COALESCE(SUM(r.amount_fen), 0)
                   FROM recharge_records r
                  WHERE r.user_id = user_balance.user_id
                    AND r.method = 'new_user_trial'
                    AND r.operator_id = 'system'
               ),
               updated_at = datetime('now')
         WHERE user_id IN (SELECT id FROM users WHERE status = 'active')
           AND (
                 SELECT COALESCE(SUM(r.amount_fen), 0)
                   FROM recharge_records r
                  WHERE r.user_id = user_balance.user_id
                    AND r.method = 'new_user_trial'
                    AND r.operator_id = 'system'
               ) < 30000;

        INSERT INTO recharge_records
          (id, user_id, amount_fen, method, operator_id, note, created_at)
        SELECT
          'rch_' || lower(hex(randomblob(16))),
          u.id,
          30000 - COALESCE(SUM(r.amount_fen), 0),
          'new_user_trial',
          'system',
          CASE
            WHEN COALESCE(SUM(r.amount_fen), 0) = 0
            THEN 'new user trial credit'
            ELSE 'new user trial credit top-up to 30000 fen'
          END,
          datetime('now')
          FROM users u
          LEFT JOIN recharge_records r
            ON r.user_id = u.id
           AND r.method = 'new_user_trial'
           AND r.operator_id = 'system'
         WHERE u.status = 'active'
         GROUP BY u.id
        HAVING COALESCE(SUM(r.amount_fen), 0) < 30000;
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "billing_trial_credit_migration_tests.rs"]
mod tests;
