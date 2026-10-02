use super::*;

#[test]
fn xhs_short_domains_use_public_mobile_metadata_without_widening_trust() {
    for host in [
        "xhslink.cn",
        "www.xhslink.cn",
        "xhslink.com",
        "www.xhslink.com",
        "www.xiaohongshu.com",
    ] {
        let url = public_url(&format!("https://{host}/o/example")).unwrap();
        assert_eq!(policy::site(&url), "小红书");
        assert!(policy::user_agent(&url).contains("Mobile/"));
        assert!(!policy::user_agent(&url).contains("MicroMessenger"));
        assert!(policy::embed(&url).is_none());
    }
    let fake = public_url("https://xhslink.cn.evil.example/o/example").unwrap();
    assert!(policy::generic(&fake));
    assert!(!policy::user_agent(&fake).contains("Mobile/"));
}

#[test]
fn xhs_mobile_hydration_uses_same_note_cover_and_camel_case_author() {
    let url = public_url(
        "https://www.xiaohongshu.com/discovery/item/0123456789abcdef01234567?xsec_token=fixture",
    )
    .unwrap();
    let html = r#"<title>小红书</title><script>window.__INITIAL_STATE__={"note":{"unused":undefined,"noteDetailMap":{"selected":{"note":{"noteId":"0123456789abcdef01234567","title":"Game creation","user":{"nickName":"Creator"},"imageList":[{"url":"https://sns-webpic-qc.xhscdn.com/cover.jpg?sign=fixture"}],"video":{}}}}}};</script>"#;
    let meta = media_page::parse(html, &url).unwrap();
    assert_eq!(meta.title, "Game creation");
    assert_eq!(meta.author, "Creator");
    assert!(meta.image.unwrap().ends_with("?sign=fixture"));
    assert!(media_page::parse(
        &html.replace("0123456789abcdef01234567", "aaaaaaaaaaaaaaaaaaaaaaaa"),
        &url
    )
    .is_none());
    assert!(media_page::parse(&html.replace("undefined", "runCode()"), &url).is_none());
    assert!(media_page::parse(
        &html.replace("sns-webpic-qc.xhscdn.com", "xhscdn.com.evil.example"),
        &url
    )
    .is_none());
}

#[tokio::test]
#[ignore = "read-only public XHS share network acceptance"]
async fn xhs_cn_live_reported_cards() {
    let mut previews = Vec::new();
    for sample in [
        "https://xhslink.cn/o/7QvqZwsx2Ch",
        "https://xhslink.cn/o/4Oatsa1J8RD",
    ] {
        let result = preview(public_url(sample).unwrap()).await;
        println!(
            "xhs_card status={} title={} author={} inline_cover={}",
            result.status,
            !result.title.is_empty(),
            !result.author.is_empty(),
            result.cover_data_url.is_some()
        );
        assert_eq!(result.url, sample);
        assert_eq!(result.site, "小红书");
        assert_eq!(result.status, "ready");
        assert!(!result.title.is_empty());
        assert!(!result.author.is_empty());
        assert!(result.cover_data_url.is_some());
        assert!(result.embed.is_none());
        // Only the public card, never response headers or redirected URL credentials.
        previews.push(result);
    }
    if let Ok(path) = std::env::var("ELON_XHS_CARD_EVIDENCE") {
        std::fs::write(path, serde_json::to_vec(&previews).unwrap()).unwrap();
    }
}
