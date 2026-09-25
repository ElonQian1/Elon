//! The read adapter reuses the same private history and media owners as Android.
#[cfg(windows)]
pub(super) fn scripts() -> String {
    macro_rules! asset {
        ($name:literal) => {
            include_str!(concat!(
                "../../../../android/app/src/main/assets/",
                $name,
                ".js"
            ))
        };
    }
    [
        asset!("chatgpt_web_private_json_request"),
        asset!("chatgpt_web_private_auth_context"),
        asset!("chatgpt_web_text_blocks"),
        asset!("chatgpt_web_private_file_citation"),
        asset!("chatgpt_web_private_history_projection"),
        asset!("chatgpt_web_private_image_pointer"),
        asset!("chatgpt_web_private_content_source"),
        asset!("chatgpt_web_private_library_download"),
        asset!("chatgpt_web_private_library_raster_policy"),
        asset!("chatgpt_web_private_canvas_text_export"),
        asset!("chatgpt_web_private_generated_image_download"),
        asset!("chatgpt_web_private_file_download"),
        asset!("chatgpt_web_conversation_content"),
        asset!("chatgpt_web_conversation_projection"),
        asset!("chatgpt_web_conversation_assets"),
        asset!("chatgpt_web_conversation_reader"),
    ]
    .join("\n")
}
