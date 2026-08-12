#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/inserts.rs"]
mod inserts;

pub(super) use fixture::*;
pub(super) use inserts::*;

pub(super) const AT: &str = "2026-08-13T00:00:00.000000000Z";

pub(super) fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

pub(super) fn object_count(connection: &rusqlite::Connection, name: &str) -> i64 {
    connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name=?1",
            [name],
            |row| row.get(0),
        )
        .unwrap()
}

pub(super) fn object_sql(connection: &rusqlite::Connection, kind: &str, name: &str) -> String {
    connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type=?1 AND name=?2",
            [kind, name],
            |row| row.get(0),
        )
        .unwrap()
}
