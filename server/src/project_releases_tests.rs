use super::persisted_release_uploader;

#[test]
fn virtual_owner_is_not_written_into_the_user_foreign_key() {
    assert_eq!(persisted_release_uploader("local-owner"), None);
}

#[test]
fn persisted_users_remain_attributed_to_the_release() {
    assert_eq!(persisted_release_uploader("user-42"), Some("user-42"));
}
