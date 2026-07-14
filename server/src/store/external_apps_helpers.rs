use super::*;

pub(super) fn ensure_external_app_default_groups_tx(
    tx: &Transaction<'_>,
    app_id: &str,
    group_seeds: &[ExternalAppGroupSeed],
) -> Result<Vec<ExternalAppGroupLink>> {
    let owner_id = ensure_external_app_owner_tx(tx, app_id)?;
    let ts = now();
    let mut links = Vec::new();
    for seed in group_seeds.iter().filter(|seed| seed.app_id == app_id) {
        tx.execute(
            "INSERT OR IGNORE INTO friend_groups (id, name, owner_user_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![seed.group_id, seed.name, owner_id, ts],
        )?;
        tx.execute(
            "UPDATE friend_groups SET name = ?1, updated_at = ?2 WHERE id = ?3",
            params![seed.name, ts, seed.group_id],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO friend_group_members (
                group_id, user_id, created_at, last_read_at
             )
             VALUES (?1, ?2, ?3, ?3)",
            params![seed.group_id, owner_id, ts],
        )?;
        tx.execute(
            "INSERT INTO external_app_groups (
                app_id, external_group_id, group_id, name, position,
                auto_join, created_at, updated_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(app_id, external_group_id) DO UPDATE SET
                group_id = excluded.group_id,
                name = excluded.name,
                position = excluded.position,
                auto_join = excluded.auto_join,
                updated_at = excluded.updated_at",
            params![
                app_id,
                seed.external_group_id,
                seed.group_id,
                seed.name,
                seed.position,
                seed.auto_join,
                ts
            ],
        )?;
        links.push(ExternalAppGroupLink {
            app_id: app_id.to_string(),
            external_group_id: seed.external_group_id.clone(),
            group_id: seed.group_id.clone(),
            name: seed.name.clone(),
            position: seed.position,
            auto_join: seed.auto_join,
        });
    }
    Ok(links)
}

pub(super) fn ensure_external_app_owner_tx(tx: &Transaction<'_>, app_id: &str) -> Result<String> {
    let owner_id = format!("usr_external_{}", app_id);
    let account = format!("{}@{}", owner_id, EXTERNAL_OWNER_DOMAIN);
    let ts = now();
    tx.execute(
        "INSERT OR IGNORE INTO users (
            id, phone, email, password_hash, nickname, role, status, created_at, updated_at
         )
         VALUES (?1, NULL, ?2, ?3, ?4, 'admin', 'active', ?5, ?5)",
        params![
            owner_id,
            account,
            hash_password(&format!("external-owner:{app_id}:{}", uuid::Uuid::new_v4())),
            format!("{} 官方", app_id),
            ts
        ],
    )?;
    Ok(owner_id)
}

pub(super) fn create_external_shadow_user_tx(
    tx: &Transaction<'_>,
    app_id: &str,
    account: &str,
    display_name: Option<&str>,
    avatar_url: Option<&str>,
    ts: &str,
) -> Result<String> {
    let user_id = new_id("usr");
    let (phone, email) = account_columns(account);
    tx.execute(
        "INSERT INTO users (
            id, phone, email, password_hash, nickname, role, status,
            avatar_data_url, created_at, updated_at
         )
         VALUES (?1, ?2, ?3, ?4, ?5, 'user', 'active', ?6, ?7, ?7)",
        params![
            user_id,
            phone,
            email,
            hash_password(&format!("external:{app_id}:{}", uuid::Uuid::new_v4())),
            display_name,
            avatar_url,
            ts
        ],
    )?;
    Ok(user_id)
}

pub(super) fn maybe_grant_external_app_trial_credit_tx(
    tx: &Transaction<'_>,
    app_id: &str,
    user_id: &str,
    ts: &str,
) -> Result<Option<ExternalAppTrialCredit>> {
    let config_key = format!("external_app_{}_trial_credit_fen", app_id);
    let amount_fen = tx
        .query_row(
            "SELECT value FROM billing_config WHERE key = ?1",
            params![config_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|value| value.trim().parse::<i64>().ok())
        .unwrap_or(0);
    if amount_fen <= 0 {
        return Ok(None);
    }

    let operator_id = format!("external_app:{app_id}");
    let already_granted = tx
        .query_row(
            "SELECT 1
             FROM recharge_records
             WHERE user_id = ?1 AND method = ?2 AND operator_id = ?3
             LIMIT 1",
            params![user_id, EXTERNAL_APP_TRIAL_METHOD, operator_id.as_str()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if already_granted {
        return Ok(None);
    }

    tx.execute(
        "INSERT OR IGNORE INTO user_balance (user_id, balance_fen, updated_at)
         VALUES (?1, 0, ?2)",
        params![user_id, ts],
    )?;
    tx.execute(
        "UPDATE user_balance
         SET balance_fen = balance_fen + ?1, updated_at = ?2
         WHERE user_id = ?3",
        params![amount_fen, ts, user_id],
    )?;
    let balance_after_fen: i64 = tx.query_row(
        "SELECT balance_fen FROM user_balance WHERE user_id = ?1",
        params![user_id],
        |row| row.get(0),
    )?;
    tx.execute(
        "INSERT INTO recharge_records
         (id, user_id, amount_fen, method, operator_id, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            new_id("rch"),
            user_id,
            amount_fen,
            EXTERNAL_APP_TRIAL_METHOD,
            operator_id.as_str(),
            format!("{app_id} external app trial credit"),
            ts
        ],
    )?;

    Ok(Some(ExternalAppTrialCredit {
        app_id: app_id.to_string(),
        amount_fen,
        balance_after_fen,
    }))
}

pub(super) fn active_user_id_by_account_tx(
    tx: &Transaction<'_>,
    account: &str,
) -> Result<Option<String>> {
    tx.query_row(
        "SELECT id
         FROM users
         WHERE (phone = ?1 OR email = ?1 OR id = ?1) AND status = 'active'
         LIMIT 1",
        params![account],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn user_exists_tx(tx: &Transaction<'_>, user_id: &str) -> Result<bool> {
    Ok(tx
        .query_row(
            "SELECT 1 FROM users WHERE id = ?1 AND status = 'active' LIMIT 1",
            params![user_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

pub(super) fn public_user_by_id_conn(
    conn: &rusqlite::Connection,
    user_id: &str,
) -> Result<PublicUser> {
    conn.query_row(
        "SELECT id, phone, email, nickname, role, status, avatar_data_url
         FROM users
         WHERE id = ?1 AND status = 'active'",
        params![user_id],
        |row| {
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            Ok(PublicUser {
                id: row.get(0)?,
                account: email
                    .or(phone)
                    .unwrap_or_else(|| row.get(0).unwrap_or_default()),
                nickname: row.get(3)?,
                role: row.get(4)?,
                status: row.get(5)?,
                avatar_data_url: row.get(6)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("用户不存在或已停用"))
}

pub(super) fn load_external_account_origin(
    conn: &rusqlite::Connection,
    app_id: &str,
    external_user_id: &str,
) -> Result<ExternalAccountOrigin> {
    conn.query_row(
        "SELECT app_id, external_user_id, account, display_name, avatar_url,
                main_user_id, status, updated_at
         FROM external_app_accounts
         WHERE app_id = ?1 AND external_user_id = ?2",
        params![app_id, external_user_id],
        external_account_origin_from_row,
    )
    .optional()?
    .ok_or_else(|| anyhow!("外部账号不存在"))
}

pub(super) fn external_account_origin_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ExternalAccountOrigin> {
    Ok(ExternalAccountOrigin {
        app_id: row.get(0)?,
        external_user_id: row.get(1)?,
        account: row.get(2)?,
        display_name: row.get(3)?,
        avatar_url: row.get(4)?,
        main_user_id: row.get(5)?,
        status: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

pub(super) fn normalize_app_id(app_id: &str) -> Result<String> {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return Err(anyhow!("外部应用 ID 不能为空"));
    }
    if !app_id
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        return Err(anyhow!("外部应用 ID 只能包含小写字母、数字、- 或 _"));
    }
    Ok(app_id.to_string())
}

pub(super) fn normalize_external_user_id(external_user_id: &str) -> Result<String> {
    let external_user_id = external_user_id.trim();
    if external_user_id.is_empty() {
        return Err(anyhow!("外部用户 ID 不能为空"));
    }
    if external_user_id.chars().count() > 128 {
        return Err(anyhow!("外部用户 ID 不能超过 128 个字符"));
    }
    Ok(external_user_id.to_string())
}

pub(super) fn normalize_external_status(status: Option<&str>) -> Result<String> {
    let status = status.unwrap_or("active").trim();
    match status {
        "active" | "disabled" => Ok(status.to_string()),
        _ => Err(anyhow!("外部账号状态只能是 active 或 disabled")),
    }
}

pub(super) fn normalize_scopes(scopes: Vec<String>) -> Vec<String> {
    let mut normalized = scopes
        .into_iter()
        .map(|scope| scope.trim().to_string())
        .filter(|scope| !scope.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    if normalized.is_empty() {
        normalized.push("profile".to_string());
        normalized.push("chat_center".to_string());
    }
    normalized
}
