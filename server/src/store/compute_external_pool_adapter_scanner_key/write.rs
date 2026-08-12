mod activation;
mod registration;
mod revocation;

use chrono::{SecondsFormat, Utc};

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
