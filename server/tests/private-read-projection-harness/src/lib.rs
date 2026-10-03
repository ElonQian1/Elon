//! Tests production duplicate-key validation, digest rules and durable SQLite outbox policy.
//! Does not substitute for real node credential, TLS listener or APK acceptance.
#![allow(dead_code)]
mod device_sources;
#[path = "../../../src/private_read_projection.rs"]
mod private_read_projection;
