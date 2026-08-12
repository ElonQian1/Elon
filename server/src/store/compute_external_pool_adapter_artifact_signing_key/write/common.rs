use chrono::{SecondsFormat, Utc};

pub(super) fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true)
}
