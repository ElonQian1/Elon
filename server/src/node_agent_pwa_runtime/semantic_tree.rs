use super::{cdp::CdpClient, CaptureDiagnostic};
use serde_json::{json, Value};
use std::time::Instant;

const MAX_UI_NODES: usize = 400;
const MAX_UI_TREE_BYTES: usize = 512 * 1024;

pub(super) async fn capture(
    cdp: &mut CdpClient,
    session: &str,
    deadline: Instant,
) -> Result<Value, CaptureDiagnostic> {
    let result = cdp
        .command(
            "Runtime.evaluate",
            json!({
                "expression": semantic_tree_expression(),
                "returnByValue": true,
            }),
            Some(session),
            deadline,
        )
        .await?;
    let value = result
        .pointer("/result/value")
        .cloned()
        .ok_or_else(|| semantic_error("页面 UI 语义树没有返回可验证结果"))?;
    validate(value)
}

fn validate(value: Value) -> Result<Value, CaptureDiagnostic> {
    if value.get("schema").and_then(Value::as_str) != Some("elon.web.semantic-tree.v1") {
        return Err(semantic_error("页面 UI 语义树 schema 无效"));
    }
    let node_count = value
        .get("nodes")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| semantic_error("页面 UI 语义树缺少 nodes"))?;
    if node_count > MAX_UI_NODES {
        return Err(semantic_error("页面 UI 语义树超过节点上限"));
    }
    let bytes =
        serde_json::to_vec(&value).map_err(|_| semantic_error("页面 UI 语义树无法序列化"))?;
    if bytes.len() > MAX_UI_TREE_BYTES {
        return Err(semantic_error("页面 UI 语义树超过 512KiB 上限"));
    }
    Ok(value)
}

fn semantic_tree_expression() -> &'static str {
    r#"(() => {
      const maxNodes = 400;
      const clean = (value, max = 160) => String(value || '').replace(/\s+/g, ' ').trim().slice(0, max);
      const visible = (element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0'
          && rect.width > 0 && rect.height > 0;
      };
      const inferredRole = (element) => {
        const explicit = clean(element.getAttribute('role'), 48);
        if (explicit) return explicit;
        const tag = element.tagName.toLowerCase();
        if (tag === 'a' && element.hasAttribute('href')) return 'link';
        if (tag === 'button') return 'button';
        if (tag === 'textarea') return 'textbox';
        if (tag === 'select') return 'combobox';
        if (tag === 'img') return 'img';
        if (tag === 'input') {
          const type = String(element.getAttribute('type') || 'text').toLowerCase();
          if (type === 'checkbox') return 'checkbox';
          if (type === 'radio') return 'radio';
          if (type === 'range') return 'slider';
          if (['button', 'submit', 'reset'].includes(type)) return 'button';
          return 'textbox';
        }
        return tag;
      };
      const stableSelector = (element) => {
        if (element.id) return `#${CSS.escape(element.id)}`;
        const testId = element.getAttribute('data-testid');
        if (testId) return `[data-testid="${String(testId).replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"]`;
        const parts = [];
        let current = element;
        while (current && current !== document.body && parts.length < 6) {
          let part = current.tagName.toLowerCase();
          const parent = current.parentElement;
          if (parent) {
            const siblings = Array.from(parent.children).filter((child) => child.tagName === current.tagName);
            if (siblings.length > 1) part += `:nth-of-type(${siblings.indexOf(current) + 1})`;
          }
          parts.unshift(part);
          current = parent;
        }
        return `body > ${parts.join(' > ')}`;
      };
      const isInteractive = (element, role) => {
        if (['button', 'link', 'textbox', 'checkbox', 'radio', 'combobox', 'slider', 'menuitem', 'tab'].includes(role)) return true;
        return element.hasAttribute('tabindex') || element.hasAttribute('onclick') || element.isContentEditable;
      };
      const candidates = Array.from(document.body ? document.body.querySelectorAll('*') : [])
        .filter((element) => !['script', 'style', 'meta', 'link', 'noscript'].includes(element.tagName.toLowerCase()))
        .filter(visible)
        .filter((element) => {
          const role = inferredRole(element);
          return isInteractive(element, role)
            || element.children.length === 0
            || element.hasAttribute('aria-label')
            || element.hasAttribute('data-testid');
        });
      const selected = candidates.slice(0, maxNodes);
      const nodes = selected.map((element, index) => {
        const role = inferredRole(element);
        const interactive = isInteractive(element, role);
        const rect = element.getBoundingClientRect();
        const style = getComputedStyle(element);
        const inputType = element.tagName.toLowerCase() === 'input'
          ? String(element.getAttribute('type') || 'text').toLowerCase()
          : null;
        const sensitive = inputType === 'password';
        const label = clean(
          element.getAttribute('aria-label')
            || element.getAttribute('title')
            || element.getAttribute('placeholder')
            || (sensitive ? '' : element.textContent),
        );
        const selector = stableSelector(element);
        const parent = element.parentElement;
        return {
          id: `web-node-${index + 1}`,
          selector,
          parentSelector: parent && parent !== document.body ? stableSelector(parent) : null,
          tag: element.tagName.toLowerCase(),
          role,
          label,
          interactive,
          disabled: Boolean(element.disabled || element.getAttribute('aria-disabled') === 'true'),
          checked: typeof element.checked === 'boolean' ? element.checked : null,
          selected: typeof element.selected === 'boolean' ? element.selected : null,
          inputType,
          bounds: {
            left: Math.round(rect.left), top: Math.round(rect.top),
            width: Math.round(rect.width), height: Math.round(rect.height),
          },
          style: {
            display: clean(style.display, 32), color: clean(style.color, 64),
            backgroundColor: clean(style.backgroundColor, 64), fontSize: clean(style.fontSize, 32),
            fontWeight: clean(style.fontWeight, 32), borderRadius: clean(style.borderRadius, 64),
          },
        };
      });
      return {
        schema: 'elon.web.semantic-tree.v1',
        title: clean(document.title, 160),
        route: `${location.pathname}${location.search}${location.hash}`.slice(0, 2048),
        viewport: { width: window.innerWidth, height: window.innerHeight, deviceScaleFactor: window.devicePixelRatio },
        nodeCount: nodes.length,
        interactiveCount: nodes.filter((node) => node.interactive).length,
        truncated: candidates.length > maxNodes,
        nodes,
      };
    })()"#
}

fn semantic_error(message: impl Into<String>) -> CaptureDiagnostic {
    CaptureDiagnostic::new(
        "SEMANTIC_TREE_FAILED",
        message,
        true,
        "修复页面 DOM 或缩小页面范围后重试；不要退化为整桌面截图",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_tree_contract_is_bounded_and_rejects_invalid_schema() {
        let valid = json!({
            "schema":"elon.web.semantic-tree.v1",
            "nodeCount":1,
            "interactiveCount":1,
            "nodes":[{"selector":"#save","role":"button","label":"保存"}]
        });
        assert!(validate(valid).is_ok());
        assert!(validate(json!({"schema":"unknown","nodes":[]})).is_err());
        assert!(semantic_tree_expression().contains("inputType === 'password'"));
        assert!(semantic_tree_expression().contains("data-testid"));
    }
}
