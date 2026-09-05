use super::{
    official_quant_release_file_path, parse_release_version_code, persisted_release_uploader,
};
use axum::http::StatusCode;

#[test]
fn virtual_owner_is_not_written_into_the_user_foreign_key() {
    assert_eq!(persisted_release_uploader("local-owner"), None);
}

#[test]
fn persisted_users_remain_attributed_to_the_release() {
    assert_eq!(persisted_release_uploader("user-42"), Some("user-42"));
}

#[test]
fn official_quant_malformed_version_code_is_unprocessable() {
    assert_eq!(parse_release_version_code(Some("5"), true), Ok(Some(5)));
    assert_eq!(
        parse_release_version_code(Some("not-an-integer"), true),
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
    assert_eq!(
        parse_release_version_code(Some("not-an-integer"), false),
        Err(StatusCode::BAD_REQUEST)
    );
}

#[test]
fn official_quant_repair_path_is_derived_from_safe_server_identity() {
    let path = official_quant_release_file_path(
        std::path::Path::new("D:/server-data"),
        "yilong-quant/ignored",
        "rel_5/ignored",
        "../YilongQuant-release.apk",
    );
    assert_eq!(
        path,
        std::path::Path::new(
            "D:/server-data/project-releases/yilong-quantignored/rel_5ignored/YilongQuant-release.apk"
        )
    );
}
