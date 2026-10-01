//! Additive membership metadata; existing clients and mention directories keep their contract.
use anyhow::Result;
use rusqlite::Connection;

pub(crate) fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS friend_group_management (
            group_id TEXT PRIMARY KEY REFERENCES friend_groups(id),
            invitation_policy TEXT NOT NULL DEFAULT 'members' CHECK(invitation_policy IN ('members','admins','approval')),
            revision INTEGER NOT NULL DEFAULT 1,
            dissolved INTEGER NOT NULL DEFAULT 0 CHECK(dissolved IN (0,1))
        );
        INSERT OR IGNORE INTO friend_group_management(group_id) SELECT id FROM friend_groups;
        CREATE TABLE IF NOT EXISTS friend_group_admins (
            group_id TEXT NOT NULL, user_id TEXT NOT NULL,
            PRIMARY KEY(group_id,user_id),
            FOREIGN KEY(group_id,user_id) REFERENCES friend_group_members(group_id,user_id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS friend_group_invitation_requests (
            id TEXT PRIMARY KEY, group_id TEXT NOT NULL REFERENCES friend_groups(id),
            actor_id TEXT NOT NULL REFERENCES users(id), user_ids TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected')),
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_group_invitation_pending ON friend_group_invitation_requests(group_id,status,created_at);
        CREATE TABLE IF NOT EXISTS friend_group_membership_actions (
            group_id TEXT NOT NULL REFERENCES friend_groups(id), request_id TEXT NOT NULL,
            actor_id TEXT NOT NULL, request_json TEXT NOT NULL, receipt_json TEXT NOT NULL, created_at TEXT NOT NULL,
            PRIMARY KEY(group_id,request_id)
        );
        CREATE TRIGGER IF NOT EXISTS group_management_created AFTER INSERT ON friend_groups BEGIN
            INSERT OR IGNORE INTO friend_group_management(group_id) VALUES(NEW.id);
        END;
        CREATE TRIGGER IF NOT EXISTS group_roster_joined AFTER INSERT ON friend_group_members BEGIN
            UPDATE friend_group_management SET revision=revision+1 WHERE group_id=NEW.group_id;
        END;
        CREATE TRIGGER IF NOT EXISTS group_roster_left AFTER DELETE ON friend_group_members BEGIN
            UPDATE friend_group_management SET revision=revision+1 WHERE group_id=OLD.group_id;
            DELETE FROM friend_group_admins WHERE group_id=OLD.group_id AND user_id=OLD.user_id;
        END;
        CREATE TRIGGER IF NOT EXISTS group_roster_admin_added AFTER INSERT ON friend_group_admins BEGIN
            UPDATE friend_group_management SET revision=revision+1 WHERE group_id=NEW.group_id;
        END;
        CREATE TRIGGER IF NOT EXISTS group_roster_admin_removed AFTER DELETE ON friend_group_admins BEGIN
            UPDATE friend_group_management SET revision=revision+1 WHERE group_id=OLD.group_id;
        END;
        CREATE TRIGGER IF NOT EXISTS group_roster_owner_changed AFTER UPDATE OF owner_user_id,name ON friend_groups BEGIN
            UPDATE friend_group_management SET revision=revision+1 WHERE group_id=NEW.id;
        END;
        CREATE TRIGGER IF NOT EXISTS group_roster_profile_changed AFTER UPDATE OF nickname,avatar_data_url ON users BEGIN
            UPDATE friend_group_management SET revision=revision+1
            WHERE group_id IN (SELECT group_id FROM friend_group_members WHERE user_id=NEW.id);
        END;
    "#)?;
    Ok(())
}
