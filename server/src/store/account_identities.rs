//! First-party account identities. Provider credentials never enter this store.

use chrono::{Duration, Utc};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use super::{hash_password, hash_token, new_id, now, PublicUser, Store};

#[derive(Debug, Clone, Serialize)]
pub struct IssuedIdentityChallenge {
    pub id: String,
    pub provider: String,
    pub mode: String,
    pub nonce: String,
    pub expires_at: String,
}

#[derive(Debug, Clone)]
pub struct IdentityChallenge {
    pub id: String,
    pub provider: String,
    pub mode: String,
    pub user_id: Option<String>,
    pub nonce_hash: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifiedIdentity {
    pub provider: String,
    pub issuer: String,
    pub subject: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkedIdentity {
    pub id: String,
    pub provider: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IdentityCompletion {
    pub user: PublicUser,
    pub identity: LinkedIdentity,
    pub created_user: bool,
}

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("登录挑战不存在、已过期或已使用")]
    InvalidChallenge,
    #[error("此身份已绑定其他一龙账号")]
    IdentityOwnedByAnother,
    #[error("该邮箱已有一龙账号，请先用原方式登录后再绑定 Google")]
    ExistingAccountRequiresBind,
    #[error("至少保留一种可用登录方式")]
    CannotUnlinkLastLogin,
    #[error("绑定身份不存在")]
    IdentityNotFound,
    #[error(transparent)]
    Store(#[from] anyhow::Error),
    #[error(transparent)]
    Database(#[from] rusqlite::Error),
}

impl Store {
    pub fn record_identity_audit_event(
        &self,
        user_id: Option<&str>,
        provider: &str,
        action: &str,
        outcome: &str,
        request_id: Option<&str>,
        reason_code: Option<&str>,
    ) -> Result<(), IdentityError> {
        self.conn()?.execute(
            "INSERT INTO auth_identity_audit
             (id, user_id, provider, action, outcome, request_id, reason_code, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                new_id("ida"),
                user_id,
                provider,
                action,
                outcome,
                request_id,
                reason_code,
                now()
            ],
        )?;
        Ok(())
    }

    pub fn create_identity_challenge(
        &self,
        provider: &str,
        mode: &str,
        user_id: Option<&str>,
        platform: &str,
    ) -> Result<IssuedIdentityChallenge, IdentityError> {
        let id = new_id("idc");
        let nonce = format!("n_{}", Uuid::new_v4().simple());
        let created_at = now();
        let expires_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM auth_identity_challenges
             WHERE consumed_at IS NOT NULL OR expires_at <= ?1",
            [&created_at],
        )?;
        conn.execute(
            "INSERT INTO auth_identity_challenges
                 (id, provider, mode, user_id, nonce_hash, platform, expires_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                provider,
                mode,
                user_id,
                hash_token(&nonce),
                platform,
                expires_at,
                created_at,
            ],
        )?;
        Ok(IssuedIdentityChallenge {
            id,
            provider: provider.to_string(),
            mode: mode.to_string(),
            nonce,
            expires_at,
        })
    }

    pub fn identity_challenge(
        &self,
        challenge_id: &str,
    ) -> Result<IdentityChallenge, IdentityError> {
        let conn = self.conn()?;
        conn.query_row(
            "SELECT id, provider, mode, user_id, nonce_hash, expires_at, consumed_at
             FROM auth_identity_challenges WHERE id = ?1",
            [challenge_id],
            |row| {
                Ok(IdentityChallenge {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    mode: row.get(2)?,
                    user_id: row.get(3)?,
                    nonce_hash: row.get(4)?,
                    expires_at: row.get(5)?,
                    consumed_at: row.get(6)?,
                })
            },
        )
        .optional()?
        .filter(|challenge| {
            challenge.consumed_at.is_none() && challenge.expires_at.as_str() > now().as_str()
        })
        .ok_or(IdentityError::InvalidChallenge)
    }

    pub fn complete_identity_challenge(
        &self,
        challenge_id: &str,
        identity: &VerifiedIdentity,
    ) -> Result<IdentityCompletion, IdentityError> {
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        let challenge = challenge_in_transaction(&transaction, challenge_id)?;
        if challenge.provider != identity.provider
            || challenge.consumed_at.is_some()
            || challenge.expires_at.as_str() <= now().as_str()
            || challenge.nonce_hash != hash_token(&identity.nonce)
        {
            return Err(IdentityError::InvalidChallenge);
        }

        let existing_owner = transaction
            .query_row(
                "SELECT user_id FROM user_identities
                 WHERE provider = ?1 AND issuer = ?2 AND subject = ?3",
                params![identity.provider, identity.issuer, identity.subject],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        let timestamp = now();
        let (user_id, created_user) = if challenge.mode == "bind" {
            let intended_user = challenge
                .user_id
                .as_deref()
                .ok_or(IdentityError::InvalidChallenge)?;
            if existing_owner
                .as_deref()
                .is_some_and(|owner| owner != intended_user)
            {
                audit(
                    &transaction,
                    Some(intended_user),
                    "google",
                    "bind",
                    "conflict",
                )?;
                consume_challenge(&transaction, challenge_id)?;
                transaction.commit()?;
                return Err(IdentityError::IdentityOwnedByAnother);
            }
            (intended_user.to_string(), false)
        } else if challenge.mode == "login" {
            if let Some(owner) = existing_owner {
                (owner, false)
            } else {
                if transaction
                    .query_row(
                        "SELECT 1 FROM users WHERE email = ?1",
                        [identity.email.to_ascii_lowercase()],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some()
                {
                    audit(&transaction, None, "google", "login", "requires_bind")?;
                    consume_challenge(&transaction, challenge_id)?;
                    transaction.commit()?;
                    return Err(IdentityError::ExistingAccountRequiresBind);
                }
                let user_id = new_id("usr");
                let inaccessible_password = hash_password(&Uuid::new_v4().to_string());
                transaction.execute(
                    "INSERT INTO users
                     (id, phone, email, password_hash, nickname, role, status, created_at, updated_at,
                      password_login_enabled)
                     VALUES (?1, NULL, ?2, ?3, ?4, 'user', 'active', ?5, ?5, 0)",
                    params![
                        user_id,
                        identity.email.to_ascii_lowercase(),
                        inaccessible_password,
                        identity.display_name,
                        timestamp,
                    ],
                )?;
                (user_id, true)
            }
        } else {
            return Err(IdentityError::InvalidChallenge);
        };

        transaction.execute(
            "INSERT INTO user_identities
             (id, user_id, provider, issuer, subject, email, display_name, avatar_url, created_at,
              last_login_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(provider, issuer, subject) DO UPDATE SET
               email = excluded.email,
               display_name = excluded.display_name,
               avatar_url = excluded.avatar_url,
               last_login_at = excluded.last_login_at",
            params![
                new_id("idi"),
                user_id,
                identity.provider,
                identity.issuer,
                identity.subject,
                identity.email.to_ascii_lowercase(),
                identity.display_name,
                identity.avatar_url,
                timestamp,
            ],
        )?;
        transaction.execute(
            "UPDATE auth_identity_challenges SET consumed_at = ?1 WHERE id = ?2",
            params![timestamp, challenge_id],
        )?;
        audit(
            &transaction,
            Some(&user_id),
            &identity.provider,
            &challenge.mode,
            if created_user {
                "created_user"
            } else {
                "success"
            },
        )?;
        let user = user_in_transaction(&transaction, &user_id)?;
        let linked = identity_for_subject(&transaction, identity)?;
        transaction.commit()?;
        Ok(IdentityCompletion {
            user,
            identity: linked,
            created_user,
        })
    }

    pub fn list_linked_identities(
        &self,
        user_id: &str,
    ) -> Result<Vec<LinkedIdentity>, IdentityError> {
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT id, provider, email, display_name, avatar_url, created_at, last_login_at
             FROM user_identities WHERE user_id = ?1 ORDER BY created_at",
        )?;
        let rows = statement.query_map([user_id], linked_identity_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn unlink_identity(&self, user_id: &str, identity_id: &str) -> Result<(), IdentityError> {
        let mut conn = self.conn()?;
        let transaction = conn.transaction()?;
        let provider = transaction
            .query_row(
                "SELECT provider FROM user_identities WHERE id = ?1 AND user_id = ?2",
                params![identity_id, user_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(IdentityError::IdentityNotFound)?;
        let password_enabled: i64 = transaction.query_row(
            "SELECT password_login_enabled FROM users WHERE id = ?1",
            [user_id],
            |row| row.get(0),
        )?;
        let identity_count: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM user_identities WHERE user_id = ?1",
            [user_id],
            |row| row.get(0),
        )?;
        if password_enabled == 0 && identity_count <= 1 {
            return Err(IdentityError::CannotUnlinkLastLogin);
        }
        transaction.execute(
            "DELETE FROM user_identities WHERE id = ?1 AND user_id = ?2",
            params![identity_id, user_id],
        )?;
        audit(&transaction, Some(user_id), &provider, "unlink", "success")?;
        transaction.commit()?;
        Ok(())
    }
}

fn challenge_in_transaction(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<IdentityChallenge, IdentityError> {
    transaction
        .query_row(
            "SELECT id, provider, mode, user_id, nonce_hash, expires_at, consumed_at
             FROM auth_identity_challenges WHERE id = ?1",
            [id],
            |row| {
                Ok(IdentityChallenge {
                    id: row.get(0)?,
                    provider: row.get(1)?,
                    mode: row.get(2)?,
                    user_id: row.get(3)?,
                    nonce_hash: row.get(4)?,
                    expires_at: row.get(5)?,
                    consumed_at: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or(IdentityError::InvalidChallenge)
}

fn user_in_transaction(
    transaction: &Transaction<'_>,
    user_id: &str,
) -> Result<PublicUser, IdentityError> {
    Ok(transaction.query_row(
        "SELECT id, phone, email, nickname, role, status, avatar_data_url
         FROM users WHERE id = ?1 AND status = 'active'",
        [user_id],
        |row| {
            let phone: Option<String> = row.get(1)?;
            let email: Option<String> = row.get(2)?;
            Ok(PublicUser {
                id: row.get(0)?,
                account: email.or(phone).unwrap_or_else(|| user_id.to_string()),
                nickname: row.get(3)?,
                role: row.get(4)?,
                status: row.get(5)?,
                avatar_data_url: row.get(6)?,
            })
        },
    )?)
}

fn identity_for_subject(
    transaction: &Transaction<'_>,
    identity: &VerifiedIdentity,
) -> Result<LinkedIdentity, IdentityError> {
    Ok(transaction.query_row(
        "SELECT id, provider, email, display_name, avatar_url, created_at, last_login_at
         FROM user_identities WHERE provider = ?1 AND issuer = ?2 AND subject = ?3",
        params![identity.provider, identity.issuer, identity.subject],
        linked_identity_row,
    )?)
}

fn linked_identity_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LinkedIdentity> {
    Ok(LinkedIdentity {
        id: row.get(0)?,
        provider: row.get(1)?,
        email: row.get(2)?,
        display_name: row.get(3)?,
        avatar_url: row.get(4)?,
        created_at: row.get(5)?,
        last_login_at: row.get(6)?,
    })
}

fn audit(
    transaction: &Transaction<'_>,
    user_id: Option<&str>,
    provider: &str,
    action: &str,
    outcome: &str,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT INTO auth_identity_audit
         (id, user_id, provider, action, outcome, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![new_id("ida"), user_id, provider, action, outcome, now()],
    )?;
    Ok(())
}

fn consume_challenge(transaction: &Transaction<'_>, challenge_id: &str) -> rusqlite::Result<()> {
    transaction.execute(
        "UPDATE auth_identity_challenges SET consumed_at = ?1 WHERE id = ?2",
        params![now(), challenge_id],
    )?;
    Ok(())
}
