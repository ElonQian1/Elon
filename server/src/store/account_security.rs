//! Password, offline recovery-code, and revocable device-session controls.

use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use thiserror::Error;

use super::{
    account_security_support::{
        generate_recovery_code, record_security_request, recovery_code_hash, security_audit,
        security_request_outcome,
    },
    hash_password, hash_token, new_id, normalize_account, now, validate_password, verify_password,
    Store,
};

const RECOVERY_CODE_COUNT: usize = 8;

#[derive(Debug, Clone, Serialize)]
pub struct AccountSession {
    pub id: String,
    pub device_name: Option<String>,
    pub apk_version: Option<String>,
    pub trusted_device: bool,
    pub current: bool,
    pub created_at: String,
    pub last_seen_at: Option<String>,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountSecuritySnapshot {
    pub schema_version: u32,
    pub password: PasswordSecurityStatus,
    pub recovery: RecoverySecurityStatus,
    pub sessions: Vec<AccountSession>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PasswordSecurityStatus {
    pub enabled: bool,
    pub changed_at: Option<String>,
    pub can_set: bool,
    pub can_change: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoverySecurityStatus {
    pub mode: &'static str,
    pub available_code_count: u32,
    pub external_delivery_configured: bool,
    pub external_delivery_state: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct PasswordMutation {
    pub replayed: bool,
    pub revoked_session_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecoveryCodeRotation {
    pub batch_id: String,
    pub codes: Vec<String>,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRevocation {
    pub revoked: bool,
    pub current_session: bool,
}

#[derive(Debug, Error)]
pub enum AccountSecurityError {
    #[error("当前密码不正确")]
    InvalidCurrentPassword,
    #[error("恢复码无效、已使用或已撤销")]
    InvalidRecoveryCode,
    #[error("登录会话不存在或已经失效")]
    SessionNotFound,
    #[error("请求参数无效: {0}")]
    InvalidInput(String),
    #[error(transparent)]
    Store(#[from] anyhow::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

impl Store {
    pub fn account_security_snapshot(
        &self,
        user_id: &str,
        current_token: &str,
    ) -> Result<AccountSecuritySnapshot, AccountSecurityError> {
        let (password_enabled, changed_at, recovery_count) = {
            let conn = self.conn()?;
            let (enabled, changed_at) = conn.query_row(
                "SELECT password_login_enabled, password_changed_at FROM users WHERE id = ?1 AND status = 'active'",
                [user_id],
                |row| Ok((row.get::<_, i64>(0)? != 0, row.get::<_, Option<String>>(1)?)),
            )?;
            let count = conn.query_row(
                "SELECT COUNT(*) FROM account_recovery_codes
                 WHERE user_id = ?1 AND used_at IS NULL AND revoked_at IS NULL",
                [user_id],
                |row| row.get::<_, u32>(0),
            )?;
            (enabled, changed_at, count)
        };
        Ok(AccountSecuritySnapshot {
            schema_version: 1,
            password: PasswordSecurityStatus {
                enabled: password_enabled,
                changed_at,
                can_set: !password_enabled,
                can_change: password_enabled,
            },
            recovery: RecoverySecurityStatus {
                mode: "offline_recovery_codes",
                available_code_count: recovery_count,
                external_delivery_configured: false,
                external_delivery_state: "reserved_not_configured",
            },
            sessions: self.list_account_sessions(user_id, current_token)?,
        })
    }

    pub fn list_account_sessions(
        &self,
        user_id: &str,
        current_token: &str,
    ) -> Result<Vec<AccountSession>, AccountSecurityError> {
        let current_hash = hash_token(current_token);
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT id, device_name, apk_version, trusted_device, token_hash,
                    created_at, last_seen_at, expires_at
             FROM sessions
             WHERE user_id = ?1 AND expires_at > ?2 AND revoked_at IS NULL
             ORDER BY current DESC, last_seen_at DESC, created_at DESC"
                .replace(
                    "current DESC",
                    "CASE WHEN token_hash = ?3 THEN 1 ELSE 0 END DESC",
                )
                .as_str(),
        )?;
        let rows = statement.query_map(params![user_id, now(), current_hash], |row| {
            let token_hash: String = row.get(4)?;
            Ok(AccountSession {
                id: row.get(0)?,
                device_name: row.get(1)?,
                apk_version: row.get(2)?,
                trusted_device: row.get::<_, i64>(3)? != 0,
                current: token_hash == current_hash,
                created_at: row.get(5)?,
                last_seen_at: row.get(6)?,
                expires_at: row.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn change_account_password(
        &self,
        user_id: &str,
        current_token: &str,
        current_password: Option<&str>,
        new_password: &str,
        request_id: &str,
    ) -> Result<PasswordMutation, AccountSecurityError> {
        validate_password(new_password)?;
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        if security_request_outcome(&transaction, user_id, "password_change", request_id)?.is_some()
        {
            return Ok(PasswordMutation {
                replayed: true,
                revoked_session_count: 0,
            });
        }
        let (enabled, stored_hash): (i64, String) = transaction.query_row(
            "SELECT password_login_enabled, password_hash FROM users WHERE id = ?1 AND status = 'active'",
            [user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if enabled != 0 {
            let supplied = current_password.unwrap_or_default();
            if !verify_password(supplied, &stored_hash) {
                security_audit(
                    &transaction,
                    Some(user_id),
                    "password_change",
                    "rejected",
                    None,
                    Some(request_id),
                    Some("invalid_current_password"),
                )?;
                transaction.commit()?;
                return Err(AccountSecurityError::InvalidCurrentPassword);
            }
            if verify_password(new_password, &stored_hash) {
                return Err(AccountSecurityError::InvalidInput(
                    "新密码不能与当前密码相同".to_string(),
                ));
            }
        }
        let timestamp = now();
        transaction.execute(
            "UPDATE users SET password_hash = ?1, password_login_enabled = 1,
                    password_changed_at = ?2, updated_at = ?2 WHERE id = ?3",
            params![hash_password(new_password), timestamp, user_id],
        )?;
        let current_hash = hash_token(current_token);
        let revoked = transaction.execute(
            "UPDATE sessions SET revoked_at = ?1, revocation_reason = 'password_changed'
             WHERE user_id = ?2 AND token_hash != ?3 AND revoked_at IS NULL",
            params![timestamp, user_id, current_hash],
        )? as u64;
        record_security_request(
            &transaction,
            user_id,
            "password_change",
            request_id,
            "completed",
        )?;
        security_audit(
            &transaction,
            Some(user_id),
            "password_change",
            "success",
            None,
            Some(request_id),
            None,
        )?;
        transaction.commit()?;
        Ok(PasswordMutation {
            replayed: false,
            revoked_session_count: revoked,
        })
    }

    pub fn rotate_account_recovery_codes(
        &self,
        user_id: &str,
        current_password: Option<&str>,
        request_id: &str,
    ) -> Result<RecoveryCodeRotation, AccountSecurityError> {
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        if let Some(batch_id) =
            security_request_outcome(&transaction, user_id, "recovery_codes_rotate", request_id)?
        {
            return Ok(RecoveryCodeRotation {
                batch_id,
                codes: Vec::new(),
                replayed: true,
            });
        }
        let (enabled, stored_hash): (i64, String) = transaction.query_row(
            "SELECT password_login_enabled, password_hash FROM users WHERE id = ?1 AND status = 'active'",
            [user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if enabled != 0 && !verify_password(current_password.unwrap_or_default(), &stored_hash) {
            return Err(AccountSecurityError::InvalidCurrentPassword);
        }
        let timestamp = now();
        transaction.execute(
            "UPDATE account_recovery_codes SET revoked_at = ?1
             WHERE user_id = ?2 AND used_at IS NULL AND revoked_at IS NULL",
            params![timestamp, user_id],
        )?;
        let batch_id = new_id("rcb");
        let mut codes = Vec::with_capacity(RECOVERY_CODE_COUNT);
        for _ in 0..RECOVERY_CODE_COUNT {
            let code = generate_recovery_code();
            transaction.execute(
                "INSERT INTO account_recovery_codes
                 (id, user_id, batch_id, code_hash, last_four, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    new_id("rcv"),
                    user_id,
                    batch_id,
                    recovery_code_hash(&code),
                    code.chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>(),
                    timestamp,
                ],
            )?;
            codes.push(code);
        }
        record_security_request(
            &transaction,
            user_id,
            "recovery_codes_rotate",
            request_id,
            &batch_id,
        )?;
        security_audit(
            &transaction,
            Some(user_id),
            "recovery_codes_rotate",
            "success",
            None,
            Some(request_id),
            None,
        )?;
        transaction.commit()?;
        Ok(RecoveryCodeRotation {
            batch_id,
            codes,
            replayed: false,
        })
    }

    pub fn recover_account_password(
        &self,
        account: &str,
        recovery_code: &str,
        new_password: &str,
        request_id: &str,
    ) -> Result<PasswordMutation, AccountSecurityError> {
        let account = normalize_account(account)?;
        validate_password(new_password)?;
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        let user_id = transaction
            .query_row(
                "SELECT id FROM users
                 WHERE (phone = ?1 OR email = ?1 OR id = ?1) AND status = 'active'",
                [&account],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(AccountSecurityError::InvalidRecoveryCode)?;
        if security_request_outcome(&transaction, &user_id, "password_recover", request_id)?
            .is_some()
        {
            return Ok(PasswordMutation {
                replayed: true,
                revoked_session_count: 0,
            });
        }
        let timestamp = now();
        let updated = transaction.execute(
            "UPDATE account_recovery_codes SET used_at = ?1
             WHERE user_id = ?2 AND code_hash = ?3 AND used_at IS NULL AND revoked_at IS NULL",
            params![timestamp, user_id, recovery_code_hash(recovery_code)],
        )?;
        if updated == 0 {
            security_audit(
                &transaction,
                Some(&user_id),
                "password_recover",
                "rejected",
                None,
                Some(request_id),
                Some("invalid_recovery_code"),
            )?;
            transaction.commit()?;
            return Err(AccountSecurityError::InvalidRecoveryCode);
        }
        transaction.execute(
            "UPDATE account_recovery_codes SET revoked_at = ?1
             WHERE user_id = ?2 AND used_at IS NULL AND revoked_at IS NULL",
            params![timestamp, user_id],
        )?;
        transaction.execute(
            "UPDATE users SET password_hash = ?1, password_login_enabled = 1,
                    password_changed_at = ?2, updated_at = ?2 WHERE id = ?3",
            params![hash_password(new_password), timestamp, user_id],
        )?;
        let revoked = transaction.execute(
            "UPDATE sessions SET revoked_at = ?1, revocation_reason = 'password_recovered'
             WHERE user_id = ?2 AND revoked_at IS NULL",
            params![timestamp, user_id],
        )? as u64;
        record_security_request(
            &transaction,
            &user_id,
            "password_recover",
            request_id,
            "completed",
        )?;
        security_audit(
            &transaction,
            Some(&user_id),
            "password_recover",
            "success",
            None,
            Some(request_id),
            None,
        )?;
        transaction.commit()?;
        Ok(PasswordMutation {
            replayed: false,
            revoked_session_count: revoked,
        })
    }

    pub fn revoke_account_session(
        &self,
        user_id: &str,
        current_token: &str,
        session_id: &str,
    ) -> Result<SessionRevocation, AccountSecurityError> {
        let timestamp = now();
        let current_hash = hash_token(current_token);
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        let is_current = transaction
            .query_row(
                "SELECT token_hash = ?1 FROM sessions
                 WHERE id = ?2 AND user_id = ?3 AND revoked_at IS NULL AND expires_at > ?4",
                params![current_hash, session_id, user_id, timestamp],
                |row| row.get::<_, bool>(0),
            )
            .optional()?
            .ok_or(AccountSecurityError::SessionNotFound)?;
        transaction.execute(
            "UPDATE sessions SET revoked_at = ?1, revocation_reason = 'user_revoked'
             WHERE id = ?2 AND user_id = ?3 AND revoked_at IS NULL",
            params![timestamp, session_id, user_id],
        )?;
        security_audit(
            &transaction,
            Some(user_id),
            "session_revoke",
            "success",
            Some(session_id),
            None,
            None,
        )?;
        transaction.commit()?;
        Ok(SessionRevocation {
            revoked: true,
            current_session: is_current,
        })
    }

    pub fn revoke_other_account_sessions(
        &self,
        user_id: &str,
        current_token: &str,
    ) -> Result<u64, AccountSecurityError> {
        let timestamp = now();
        let current_hash = hash_token(current_token);
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        let revoked = transaction.execute(
            "UPDATE sessions SET revoked_at = ?1, revocation_reason = 'user_revoked_others'
             WHERE user_id = ?2 AND token_hash != ?3 AND revoked_at IS NULL",
            params![timestamp, user_id, current_hash],
        )? as u64;
        security_audit(
            &transaction,
            Some(user_id),
            "sessions_revoke_others",
            "success",
            None,
            None,
            None,
        )?;
        transaction.commit()?;
        Ok(revoked)
    }
}
