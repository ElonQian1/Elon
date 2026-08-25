const WIN_COMMON_RICH_CONTENT_ADAPTER: &str = include_str!("rich_content_dom_adapter.js");
const WIN_RICH_CONTENT_ADAPTER: &str = include_str!("google_rich_content_adapter.js");
const WIN_PRIVATE_CONVERSATION_BRIDGE: &str =
    include_str!("google_win_private_conversation_bridge.js");
const PRIVATE_RESPONSE_TAP: &str =
    include_str!("../../../../android/app/src/main/assets/google_web_private_response_tap.js");
const PRIVATE_THREAD_DIRECTORY: &str =
    include_str!("../../../../android/app/src/main/assets/google_web_private_thread_directory.js");

const ADAPTER_ASSETS: &[(&str, &str)] = &[
    (
        "google_web_answer_candidate_policy.js",
        include_str!(
            "../../../../android/app/src/main/assets/google_web_answer_candidate_policy.js"
        ),
    ),
    (
        "google_web_private_reply_observer.js",
        include_str!(
            "../../../../android/app/src/main/assets/google_web_private_reply_observer.js"
        ),
    ),
    (
        "google_web_private_thread_directory.js",
        include_str!(
            "../../../../android/app/src/main/assets/google_web_private_thread_directory.js"
        ),
    ),
    (
        "google_web_query_policy.js",
        include_str!("../../../../android/app/src/main/assets/google_web_query_policy.js"),
    ),
    (
        "google_web_rich_content.js",
        include_str!("../../../../android/app/src/main/assets/google_web_rich_content.js"),
    ),
    (
        "google_web_message_extractor.js",
        include_str!("../../../../android/app/src/main/assets/google_web_message_extractor.js"),
    ),
    (
        "google_web_composer_bridge.js",
        include_str!("../../../../android/app/src/main/assets/google_web_composer_bridge.js"),
    ),
    (
        "google_web_send_policy.js",
        include_str!("../../../../android/app/src/main/assets/google_web_send_policy.js"),
    ),
    (
        "google_web_adapter.js",
        include_str!("../../../../android/app/src/main/assets/google_web_adapter.js"),
    ),
];

#[cfg(test)]
pub(super) fn adapter_asset_names() -> Vec<&'static str> {
    ADAPTER_ASSETS.iter().map(|(name, _)| *name).collect()
}

pub(super) fn initialization_script(adapter_version: u32) -> String {
    let response_research_capture = include_str!("win_web_response_research_capture.js")
        .replace("__PROVIDER_ID__", "google-ai-mode");
    let adapters = ADAPTER_ASSETS
        .iter()
        .map(|(name, source)| {
            let shared = format!("window.__elonGoogleWebBootstrapStage = '{name}';\n{source}");
            if *name == "google_web_rich_content.js" {
                format!(
                    "{shared}\nwindow.__elonGoogleWebBootstrapStage = 'rich_content_dom_adapter.js';\n{WIN_COMMON_RICH_CONTENT_ADAPTER}\nwindow.__elonGoogleWebBootstrapStage = 'google_rich_content_adapter.js';\n{WIN_RICH_CONTENT_ADAPTER}"
                )
            } else if *name == "google_web_adapter.js" {
                format!(
                    "{shared}\nwindow.__elonGoogleWebBootstrapStage = 'google_win_private_conversation_bridge.js';\n{WIN_PRIVATE_CONVERSATION_BRIDGE}"
                )
            } else {
                shared
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    r#"
(function () {
  'use strict';
  function allowedOrigin() {
    return location.origin === 'https://google.com' || location.origin === 'https://www.google.com';
  }
  if (!allowedOrigin()) return;
  window.__elonGoogleWebBootstrapStage = 'google_web_private_thread_directory.js';
  __PRIVATE_THREAD_DIRECTORY_SOURCE__
  window.__elonGoogleWebBootstrapStage = 'google_web_private_response_tap.js';
  window.__elonGoogleWebPrivateResearchEnabled = true;
  __PRIVATE_RESPONSE_TAP_SOURCE__
  __RESPONSE_RESEARCH_CAPTURE__

  function invoke(payload) {
    if (!allowedOrigin()) return;
    var internalInvoke = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    var publicInvoke = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    var call = internalInvoke || publicInvoke;
    if (typeof call === 'function') {
      Promise.resolve(call('publish_local_ai_web_event', { payload: String(payload || '') })).catch(function () {});
    }
  }

  window.elonGoogleWebNative = Object.freeze({ postMessage: invoke });
  if (!window.__elonWinGoogleWebDiagnosticsInstalled) {
    window.__elonWinGoogleWebDiagnosticsInstalled = true;
    window.addEventListener('error', function (event) {
      invoke(JSON.stringify({
        type: 'browser_diagnostic', kind: 'page_error',
        detail: String(event && event.message || 'Google AI 页面脚本加载失败。').slice(0, 240),
        url: location.origin + location.pathname
      }));
    });
    window.addEventListener('unhandledrejection', function () {
      invoke(JSON.stringify({
        type: 'browser_diagnostic', kind: 'promise_rejection',
        detail: 'Google AI 页面尚未完成初始化，可显示官方窗口确认。',
        url: location.origin + location.pathname
      }));
    });
  }

  function documentToken() {
    var words = new Uint32Array(4);
    if (window.crypto && typeof window.crypto.getRandomValues === 'function') {
      window.crypto.getRandomValues(words);
    } else {
      for (var index = 0; index < words.length; index += 1) {
        words[index] = Math.floor(Math.random() * 0xffffffff) >>> 0;
      }
    }
    return 'doc_win_' + Array.from(words, function (word) {
      return word.toString(16).padStart(8, '0');
    }).join('');
  }
  if (!/^doc_[a-z0-9_]{3,80}$/.test(String(window.__elonGoogleWebDocumentToken || ''))) {
    window.__elonGoogleWebDocumentToken = documentToken();
  }

  function installAdapter() {
    try {
      window.__elonGoogleWebAdapterVersion = __ADAPTER_VERSION__;
      __ADAPTER_ASSETS__
      if (!window.__elonGoogleWebBridge || typeof window.__elonGoogleWebBridge.command !== 'function') {
        throw new Error('bridge_missing');
      }
    } catch (error) {
      var errorName = String(error && error.name || 'Error').replace(/[^A-Za-z0-9_]/g, '').slice(0, 40);
      invoke(JSON.stringify({
        type: 'browser_diagnostic', kind: 'adapter_bootstrap_failed',
        detail: 'Google AI 语义桥初始化失败（' + (errorName || 'Error') + '）。',
        url: location.origin + location.pathname
      }));
    }
  }

  function installWhenReady() {
    if (!(document.documentElement instanceof Node)) {
      window.setTimeout(installWhenReady, 0);
      return;
    }
    installAdapter();
  }
  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', installWhenReady, { once: true });
  } else {
    installWhenReady();
  }
})();
"#
    .replace("__ADAPTER_VERSION__", &adapter_version.to_string())
    .replace("__PRIVATE_THREAD_DIRECTORY_SOURCE__", PRIVATE_THREAD_DIRECTORY)
    .replace("__PRIVATE_RESPONSE_TAP_SOURCE__", PRIVATE_RESPONSE_TAP)
    .replace("__RESPONSE_RESEARCH_CAPTURE__", &response_research_capture)
    .replace("__ADAPTER_ASSETS__", &adapters)
}
