use serde_json::to_string;
use tauri::{Url, Webview};

use super::adapter_command;

pub(super) fn start(page: &Webview, provider_id: &str, url: &Url) -> bool {
    script(provider_id, url.path()).is_some_and(|script| page.eval(script).is_ok())
}

fn script(provider_id: &str, path: &str) -> Option<String> {
    if provider_id != "chatgpt" || !adapter_command::is_safe_conversation_path(path) {
        return None;
    }
    let encoded_path = to_string(path).ok()?;
    Some(format!(
        r#"(function(path){{
'use strict';
function navigate(){{location.assign(new URL(path,location.origin).href);}}
try{{
  var transport=window.__elonChatGptPrivateTransport;
  var nativeBridge=window.elonChatGptNative;
  var emit=function(event){{
    if(nativeBridge&&typeof nativeBridge.postMessage==='function'){{
      nativeBridge.postMessage(JSON.stringify(event));
    }}
  }};
  if(transport&&transport.conversationPrefetchEnabled===true&&
      typeof transport.prefetchConversation==='function'&&
      transport.prefetchConversation(path,emit,navigate)===true)return;
}}catch(_){{}}
navigate();
}})({encoded_path});"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chatgpt_cached_conversations_prefetch_before_navigation() {
        let script = script("chatgpt", "/c/conversation-123").unwrap();

        assert!(script.contains("transport.prefetchConversation(path,emit,navigate)"));
        assert!(script.contains("nativeBridge.postMessage(JSON.stringify(event))"));
        assert!(script.contains("if(transport&&transport.conversationPrefetchEnabled===true"));
        assert!(script.contains("navigate();"));
    }

    #[test]
    fn project_conversation_paths_use_the_same_private_prefetch() {
        assert!(script("chatgpt", "/g/g-p-roadmap/c/conversation-123").is_some());
    }

    #[test]
    fn google_and_invalid_paths_keep_the_existing_navigation_fallback() {
        assert!(script("google-ai-mode", "/c/thread-123").is_none());
        assert!(script("chatgpt", "/share/unsafe").is_none());
        assert!(script("chatgpt", "https://evil.example/c/conversation").is_none());
    }
}
