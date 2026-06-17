use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension, Transaction};

use super::{
    account_columns, clean_optional, hash_password, hash_token, new_id, normalize_account, now,
    ExternalAccountOrigin, ExternalAccountSession, ExternalAccountSessionInput,
    ExternalAccountUpsert, ExternalAppAuthorizationCode, ExternalAppAuthorizationExchange,
    ExternalAppGroupLink, ExternalAppGroupSeed, PublicUser, Store,
};

const EXTERNAL_OWNER_DOMAIN: &str = "external.elon.local";

impl Store {
    pub fn external_account_origin_hint(
        &self,
        account: &str,
    ) -> Result<Option<ExternalAccountOrigin>> {
        let account = normalize_account(account)?;
        let conn = self.conn()?;
        conn.query_row(
            "SELECT app_id, external_user_id, account, display_name, avatar_url,
                    main_user_id, status, updated_at
             FROM external_app_accounts
             WHERE account = ?1 AND status = 'active'
             ORDER BY updated_at DESC
             LIMIT 1",
            params![account],
            external_account_origin_from_row,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn upsert_external_app_account(
        &self,
        app_id: &str,
        input: ExternalAccountUpsert,
    ) -> Result<ExternalAccountOrigin> {
        let app_id = normalize_app_id(app_id)?;
        let external_user_id = normalize_external_user_id(&input.external_user_id)?;
        let account = normalize_account(&input.account)?;
        let display_name = clean_optional(input.display_name.as_deref());
        let avatar_url = clean_optional(input.avatar_url.as_deref());
        let status = normalize_external_status(input.status.as_deref())?;
        let ts = now();

        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO external_app_accounts (
                app_id, external_user_id, account, display_name, avatar_url,
                main_user_id, status, created_at, updated_at, last_seen_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?7, ?7)
             ON CONFLICT(app_id, external_user_id) DO UPDATE SET
                account = excluded.account,
                display_name = excluded.display_name,
                avatar_url = excluded.avatar_url,
                status = excluded.status,
                updated_at = excluded.updated_at,
                last_seen_at = excluded.last_seen_at",
            params![
                app_id,
                external_user_id,
                account,
                display_name,
                avatar_url,
                status,
                ts
            ],
        )?;

        load_external_account_origin(&conn, &app_id, &external_user_id)
    }

    pub fn create_external_app_session(
        &self,
        app_id: &str,
        group_seeds: &[ExternalAppGroupSeed],
        input: ExternalAccountSessionInput,
    ) -> Result<ExternalAccountSession> {
        let app_id = normalize_app_id(app_id)?;
        let external_user_id = normalize_external_user_id(&input.external_user_id)?;
        let account = normalize_account(&input.account)?;
        let display_name = clean_optional(input.display_name.as_deref()).map(ToOwned::to_owned);
        let avatar_url = clean_optional(input.avatar_url.as_deref()).map(ToOwned::to_owned);
        let ts = now();

        let main_user_id = {
            let conn = self.conn()?;
            let tx = conn.unchecked_transaction()?;
            let default_groups = ensure_external_app_default_groups_tx(&tx, &app_id, group_seeds)?;

            let linked_user_id = tx
                .query_row(
                    "SELECT main_user_id
                     FROM external_app_accounts
                     WHERE app_id = ?1 AND external_user_id = ?2",
                    params![app_id, external_user_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()?
                .flatten();

            let main_user_id = match linked_user_id {
                Some(user_id) if user_exists_tx(&tx, &user_id)? => user_id,
                _ => match active_user_id_by_account_tx(&tx, &account)? {
                    Some(existing_user_id) => existing_user_id,
                    None => create_external_shadow_user_tx(
                        &tx,
                        &app_id,
                        &account,
                        display_name.as_deref(),
                        avatar_url.as_deref(),
                        &ts,
                    )?,
                },
            };

            tx.execute(
                "INSERT INTO external_app_accounts (
                    app_id, external_user_id, account, display_name, avatar_url,
                    main_user_id, status, created_at, updated_at, last_seen_at
                 )
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7, ?7)
                 ON CONFLICT(app_id, external_user_id) DO UPDATE SET
                    account = excluded.account,
                    display_name = excluded.display_name,
                    avatar_url = excluded.avatar_url,
                    main_user_id = excluded.main_user_id,
                    status = 'active',
                    updated_at = excluded.updated_at,
                    last_seen_at = excluded.last_seen_at",
                params![
                    app_id,
                    external_user_id,
                    account,
                    display_name,
                    avatar_url,
                    main_user_id,
                    ts
                ],
            )?;

            for group in default_groups.iter().filter(|group| group.auto_join) {
                tx.execute(
                    "INSERT OR IGNORE INTO friend_group_members (
                        group_id, user_id, created_at, last_read_at
                     )
                     VALUES (?1, ?2, ?3, ?3)",
                    params![group.group_id, main_user_id, ts],
                )?;
            }

            tx.commit()?;
            main_user_id
        };

        let user = self.public_user_by_id(&main_user_id)?;
        let (token, expires_at) = self.create_session(
            &main_user_id,
            input.device_name.as_deref(),
            input.apk_version.as_deref(),
        )?;
        let account = self
            .external_app_account(&app_id, &external_user_id)?
            .ok_or_else(|| anyhow!("外部账号同步失败"))?;
        let default_groups = self.ensure_external_app_default_groups(&app_id, group_seeds)?;

        Ok(ExternalAccountSession {
            token,
            expires_at,
            user,
            account,
            default_groups,
        })
    }

    pub fn ensure_external_app_default_groups(
        &self,
        app_id: &str,
        group_seeds: &[ExternalAppGroupSeed],
    ) -> Result<Vec<ExternalAppGroupLink>> {
        let app_id = normalize_app_id(app_id)?;
        let conn = self.conn()?;
        let tx = conn.unchecked_transaction()?;
        let links = ensure_external_app_default_groups_tx(&tx, &app_id, group_seeds)?;
        tx.commit()?;
        Ok(links)
    }

    pub fn create_external_app_authorization_code(
        &self,
        app_id: &str,
        user_id: &str,
        scopes: Vec<String>,
        redirect_uri: Option<&str>,
    ) -> Result<ExternalAppAuthorizationCode> {
        let app_id = normalize_app_id(app_id)?;
        let user = self.public_user_by_id(user_id)?;
        let scopes = normalize_scopes(scopes);
        let code = format!("eac_{}", uuid::Uuid::new_v4().simple());
        let code_hash = hash_token(&code);
        let created_at = now();
        let expires_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();
        let redirect_uri = clean_optional(redirect_uri).map(ToOwned::to_owned);
        let scopes_json = serde_json::to_string(&scopes)?;

        self.conn()?.execute(
            "INSERT INTO external_app_auth_codes (
                id, app_id, code_hash, user_id, scopes_json, redirect_uri,
                expires_at, created_at
             )
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id("eac"),
                app_id,
                code_hash,
                user.id,
                scopes_json,
                redirect_uri,
                expires_at,
                created_at
            ],
        )?;

        Ok(ExternalAppAuthorizationCode {
            code,
            app_id,
            user_id: user.id,
            scopes,
            redirect_uri,
            expires_at,
        })
    }

    pub fn exchange_external_app_authorization_code(
        &self,
        app_id: &str,
        code: &str,
    ) -> Result<ExternalAppAuthorizationExchange> {
        let app_id = normalize_app_id(app_id)?;
        let code = code.trim();
        if code.is_empty() {
            return Err(anyhow!("授权码不能为空"));
        }
        let code_hash = hash_token(code);
        let ts = now();
        let conn = self.conn()?;

        let row = conn
            .query_row(
                "SELECT id, user_id, scopes_json, redirect_uri, created_at
                 FROM external_app_auth_codes
                 WHERE app_id = ?1
                   AND code_hash = ?2
                   AND consumed_at IS NULL
                   AND expires_at > ?3
                 LIMIT 1",
                params![app_id, code_hash, ts],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("授权码无效或已过期"))?;

        let updated = conn.execute(
            "UPDATE external_app_auth_codes
             SET consumed_at = ?1
             WHERE id = ?2 AND consumed_at IS NULL",
            params![ts, row.0],
        )?;
        if updated != 1 {
            return Err(anyhow!("授权码无效或已过期"));
        }

        let user = public_user_by_id_conn(&conn, &row.1)?;
        let scopes = serde_json::from_str::<Vec<String>>(&row.2).unwrap_or_default();
        Ok(ExternalAppAuthorizationExchange {
            app_id,
            user,
            scopes,
            redirect_uri: row.3,
            created_at: row.4,
        })
    }

    fn external_app_account(
        &self,
        app_id: &str,
        external_user_id: &str,
    ) -> Result<Option<ExternalAccountOrigin>> {
        let conn = self.conn()?;
        load_external_account_origin(&conn, app_id, external_user_id)
            .optional_or_not_found("外部账号不存在")
    }

    fn public_user_by_id(&self, user_id: &str) -> Result<PublicUser> {
        let conn = self.conn()?;
        public_user_by_id_conn(&conn, user_id)
    }
}

trait OptionalOrigin {
    fn optional_or_not_found(self, not_found: &str) -> Result<Option<ExternalAccountOrigin>>;
}

impl OptionalOrigin for Result<ExternalAccountOrigin> {
    fn optional_or_not_found(self, not_found: &str) -> Result<Option<ExternalAccountOrigin>> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(error) if error.to_string().contains(not_found) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn ensure_external_app_default_groups_tx(
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

fn ensure_external_app_owner_tx(tx: &Transaction<'_>, app_id: &str) -> Result<String> {
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

fn create_external_shadow_user_tx(
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

fn active_user_id_by_account_tx(tx: &Transaction<'_>, account: &str) -> Result<Option<String>> {
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

fn user_exists_tx(tx: &Transaction<'_>, user_id: &str) -> Result<bool> {
    Ok(tx
        .query_row(
            "SELECT 1 FROM users WHERE id = ?1 AND status = 'active' LIMIT 1",
            params![user_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some())
}

fn public_user_by_id_conn(conn: &rusqlite::Connection, user_id: &str) -> Result<PublicUser> {
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

fn load_external_account_origin(
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

fn external_account_origin_from_row(
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

fn normalize_app_id(app_id: &str) -> Result<String> {
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

fn normalize_external_user_id(external_user_id: &str) -> Result<String> {
    let external_user_id = external_user_id.trim();
    if external_user_id.is_empty() {
        return Err(anyhow!("外部用户 ID 不能为空"));
    }
    if external_user_id.chars().count() > 128 {
        return Err(anyhow!("外部用户 ID 不能超过 128 个字符"));
    }
    Ok(external_user_id.to_string())
}

fn normalize_external_status(status: Option<&str>) -> Result<String> {
    let status = status.unwrap_or("active").trim();
    match status {
        "active" | "disabled" => Ok(status.to_string()),
        _ => Err(anyhow!("外部账号状态只能是 active 或 disabled")),
    }
}

fn normalize_scopes(scopes: Vec<String>) -> Vec<String> {
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
