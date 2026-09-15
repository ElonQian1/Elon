//! Stable failure codes only; CA response text and credential material stay private.
pub(super) fn classify(error: &anyhow::Error) -> &'static str {
    for cause in error.chain() {
        if let Some(error) = cause.downcast_ref::<instant_acme::Error>() {
            return match error {
                instant_acme::Error::Api(problem) => problem_code(problem.r#type.as_deref()),
                instant_acme::Error::Timeout(_) => "TIMEOUT",
                instant_acme::Error::Json(_) => "RESPONSE_JSON",
                instant_acme::Error::Unsupported(_) => "UNSUPPORTED",
                instant_acme::Error::Crypto | instant_acme::Error::KeyRejected => "CRYPTO",
                _ => "ACME_CLIENT",
            };
        }
        if let Some(error) = cause.downcast_ref::<std::io::Error>() {
            return match error.kind() {
                std::io::ErrorKind::AddrInUse => "PORT_IN_USE",
                std::io::ErrorKind::PermissionDenied => "PERMISSION_DENIED",
                std::io::ErrorKind::TimedOut => "TIMEOUT",
                _ => "IO",
            };
        }
        if cause.downcast_ref::<rustls::Error>().is_some() {
            return "TLS";
        }
    }
    "FAILED"
}

fn problem_code(kind: Option<&str>) -> &'static str {
    match kind.and_then(|v| v.strip_prefix("urn:ietf:params:acme:error:")) {
        Some("connection") => "CA_CONNECTION",
        Some("tls") => "CA_TLS",
        Some("unauthorized") => "CA_UNAUTHORIZED",
        Some("rejectedIdentifier") => "CA_IDENTIFIER",
        Some("rateLimited") => "CA_RATE_LIMIT",
        Some("badNonce") => "CA_NONCE",
        Some("malformed") => "CA_MALFORMED",
        Some("serverInternal") => "CA_INTERNAL",
        Some("caa") => "CA_CAA",
        _ => "CA_REJECTED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_actionable_kind_without_response_text() {
        let problem = instant_acme::Problem {
            r#type: Some("urn:ietf:params:acme:error:tls".into()),
            detail: Some("secret response text".into()),
            status: Some(400),
            subproblems: vec![],
        };
        let error =
            anyhow::Error::from(instant_acme::Error::Api(problem)).context("private context");
        assert_eq!(classify(&error), "CA_TLS");
        assert_eq!(
            classify(&std::io::Error::from(std::io::ErrorKind::AddrInUse).into()),
            "PORT_IN_USE"
        );
        assert_eq!(
            problem_code(Some("unexpected private payload")),
            "CA_REJECTED"
        );
    }
}
