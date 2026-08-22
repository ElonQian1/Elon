use tauri::Webview;

use super::{display_error, LocalAiWebSessionState};

pub(super) fn set_content_surface_mode(webview: &Webview, enabled: bool) -> Result<(), String> {
    let script = if enabled {
        answer_surface_script()
    } else {
        restore_surface_script()
    };
    webview.eval(script).map_err(display_error)
}

pub(super) fn answer_surface_ready(state: &LocalAiWebSessionState) -> bool {
    if state.loading
        || state.semantic_cache_status != "live"
        || state.window_status == "blocked"
        || state.window_status == "error"
    {
        return false;
    }
    let Some(event) = state.semantic_event.as_ref() else {
        return false;
    };
    if event
        .get("streaming")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return false;
    }
    semantic_event_has_completed_assistant(event)
}

fn semantic_event_has_completed_assistant(event: &serde_json::Value) -> bool {
    event
        .get("messages")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|messages| {
            messages.iter().any(|message| {
                message.get("role").and_then(serde_json::Value::as_str) == Some("assistant")
                    && message.get("state").and_then(serde_json::Value::as_str) != Some("streaming")
            })
        })
}

fn answer_surface_script() -> &'static str {
    r#"
(function () {
  'use strict';
  var styleId = 'elon-official-answer-surface-style';
  var backdropId = 'elon-official-answer-surface-backdrop';
  var providerAttribute = 'data-elon-official-answer-provider';
  var pathAttribute = 'data-elon-official-answer-path';
  var rootAttribute = 'data-elon-official-answer-root';

  function clearSurface() {
    document.documentElement.removeAttribute('data-elon-official-answer-surface');
    document.documentElement.removeAttribute(providerAttribute);
    document.querySelectorAll('[' + rootAttribute + '], [' + pathAttribute + ']').forEach(function (node) {
      node.removeAttribute(rootAttribute);
      node.removeAttribute(pathAttribute);
    });
    var oldStyle = document.getElementById(styleId);
    if (oldStyle) oldStyle.remove();
    var oldBackdrop = document.getElementById(backdropId);
    if (oldBackdrop) oldBackdrop.remove();
  }

  function visible(node) {
    if (!(node instanceof HTMLElement) || !node.isConnected) return false;
    var rect = node.getBoundingClientRect();
    var style = window.getComputedStyle(node);
    return rect.width > 240 && rect.height > 40 &&
      style.display !== 'none' && style.visibility !== 'hidden';
  }

  function messageRole(node) {
    if (!(node instanceof HTMLElement)) return '';
    var owner = node.matches('[data-message-author-role]')
      ? node
      : node.querySelector('[data-message-author-role]');
    return owner ? String(owner.getAttribute('data-message-author-role') || '') : '';
  }

  function chatGptTarget() {
    var portal = window.__elonChatGptMessagePortalPolicy;
    var nodes = portal && typeof portal.findMessageNodes === 'function'
      ? portal.findMessageNodes(document)
      : Array.from(document.querySelectorAll(
          '[data-testid^="conversation-turn-"], [data-message-author-role]'
        ));
    return Array.from(nodes || []).reverse().find(function (node) {
      return messageRole(node) === 'assistant' && visible(node);
    }) || null;
  }

  function googleTarget() {
    var extractor = window.__elonGoogleWebMessageExtractor;
    if (extractor && typeof extractor.lastAnswerNode === 'function') {
      var extracted = extractor.lastAnswerNode();
      if (visible(extracted)) return extracted;
    }
    return Array.from(document.querySelectorAll(
      '[data-sfc-cp][data-hveid], [data-container-id][data-hveid], [role="main"] [data-hveid]'
    )).reverse().find(visible) || null;
  }

  clearSurface();
  var provider = /(^|\.)chatgpt\.com$/i.test(window.location.hostname)
    ? 'chatgpt'
    : 'google-ai-mode';
  var target = provider === 'chatgpt' ? chatGptTarget() : googleTarget();
  if (!visible(target)) return;

  var ancestor = target.parentElement;
  while (ancestor && ancestor !== document.documentElement) {
    ancestor.setAttribute(pathAttribute, 'true');
    ancestor = ancestor.parentElement;
  }
  target.setAttribute(rootAttribute, 'true');

  var style = document.createElement('style');
  style.id = styleId;
  style.textContent = [
    'html[data-elon-official-answer-surface], html[data-elon-official-answer-surface] body { box-sizing: border-box !important; min-height: 100% !important; overflow: auto !important; overscroll-behavior: contain !important; }',
    'html[data-elon-official-answer-surface] body { margin: 0 !important; padding: 20px clamp(16px, 4vw, 48px) 32px !important; }',
    'html[data-elon-official-answer-provider="google-ai-mode"], html[data-elon-official-answer-provider="google-ai-mode"] body { background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"], html[data-elon-official-answer-provider="chatgpt"] body { background: var(--main-surface-primary, #212121) !important; }',
    'html[data-elon-official-answer-surface] body > *:not([data-elon-official-answer-path]):not(#elon-official-answer-surface-backdrop) { display: none !important; }',
    '[data-elon-official-answer-path] > *:not([data-elon-official-answer-path]):not([data-elon-official-answer-root]) { display: none !important; }',
    '[data-elon-official-answer-path] { position: static !important; inset: auto !important; display: block !important; box-sizing: border-box !important; width: 100% !important; min-width: 0 !important; max-width: none !important; height: auto !important; min-height: 0 !important; max-height: none !important; margin: 0 !important; padding: 0 !important; overflow: visible !important; transform: none !important; contain: none !important; }',
    '#elon-official-answer-surface-backdrop { position: fixed !important; inset: 0 !important; z-index: -1 !important; background: #202124 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"] #elon-official-answer-surface-backdrop { background: var(--main-surface-primary, #212121) !important; }',
    '[data-elon-official-answer-root] { position: relative !important; z-index: 1 !important; box-sizing: border-box !important; width: 100% !important; min-width: 0 !important; height: auto !important; min-height: 0 !important; margin-block: 0 !important; }',
    'html[data-elon-official-answer-provider="chatgpt"] [data-elon-official-answer-root] { width: min(100%, 48rem) !important; max-width: 48rem !important; margin-inline: auto !important; }'
  ].join('\n');
  document.head.appendChild(style);

  var backdrop = document.createElement('div');
  backdrop.id = backdropId;
  backdrop.setAttribute('aria-hidden', 'true');
  document.body.appendChild(backdrop);
  document.documentElement.setAttribute(providerAttribute, provider);
  document.documentElement.setAttribute('data-elon-official-answer-surface', 'true');
  window.requestAnimationFrame(function () {
    window.scrollTo({ top: 0, left: 0, behavior: 'instant' });
  });
})();
"#
}

fn restore_surface_script() -> &'static str {
    r#"
(function () {
  'use strict';
  document.documentElement.removeAttribute('data-elon-official-answer-surface');
  document.documentElement.removeAttribute('data-elon-official-answer-provider');
  document.querySelectorAll('[data-elon-official-answer-root], [data-elon-official-answer-path]').forEach(function (node) {
    node.removeAttribute('data-elon-official-answer-root');
    node.removeAttribute('data-elon-official-answer-path');
  });
  var style = document.getElementById('elon-official-answer-surface-style');
  if (style) style.remove();
  var backdrop = document.getElementById('elon-official-answer-surface-backdrop');
  if (backdrop) backdrop.remove();
})();
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn answer_surface_requires_a_completed_assistant_message() {
        assert!(semantic_event_has_completed_assistant(&json!({
            "messages": [
                {"role":"user","state":"completed"},
                {"role":"assistant","state":"completed"}
            ]
        })));
        assert!(!semantic_event_has_completed_assistant(&json!({
            "messages": [{"role":"assistant","state":"streaming"}]
        })));
        assert!(!semantic_event_has_completed_assistant(&json!({
            "messages": [{"role":"user","state":"completed"}]
        })));
    }

    #[test]
    fn answer_surface_script_is_pc_scoped_and_reversible() {
        let apply = answer_surface_script();
        let restore = restore_surface_script();
        assert!(apply.contains("__elonChatGptMessagePortalPolicy"));
        assert!(apply.contains("__elonGoogleWebMessageExtractor"));
        assert!(apply.contains("lastAnswerNode"));
        assert!(apply.contains("data-elon-official-answer-root"));
        assert!(apply.contains("data-elon-official-answer-path"));
        assert!(apply.contains("data-elon-official-answer-provider"));
        assert!(apply.contains(r"chatgpt\.com"));
        assert!(apply.contains("max-width: 48rem"));
        assert!(!apply.contains("cloneNode"));
        assert!(restore.contains("style.remove()"));
        assert!(restore.contains("backdrop.remove()"));
    }
}
