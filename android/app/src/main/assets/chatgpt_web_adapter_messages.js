(function () {
  'use strict';

  if (window.__elonChatGptMessages || location.origin !== 'https://chatgpt.com') return;

  const MAX_MESSAGES = 80;
  const MAX_MESSAGE_LENGTH = 40000;
  const MAX_STRUCTURED_PARTS = 16;
  const FILE_PATH_EXTENSION = /\.(?:pdf|docx?|xlsx?|csv|pptx?|txt|md|json|xml|ya?ml|zip|rar|7z|tar|gz|png|jpe?g|gif|webp|svg|mp3|wav|m4a|ogg|mp4|mov|webm)$/i;
  const COMPLEX_PART_TYPES = new Set(['artifact', 'audio', 'video', 'math', 'chart', 'map', 'interactive']);
  const messageActionPolicy = window.__elonChatGptMessageActionPolicy;
  let lastStructuredTypes = new Set();
  let lastComplexOutput = false;

  function cleanText(value) {
    return String(value || '').replace(/\u00a0/g, ' ').replace(/\n{3,}/g, '\n\n').trim();
  }

  function isVisible(node) {
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.display !== 'none' && style.visibility !== 'hidden';
  }

  function escapeMarkdown(value) {
    return String(value || '').replace(/([\\`*_[\]<>#])/g, '\\$1');
  }

  function childrenMarkdown(node, context) {
    return Array.from(node.childNodes).map((child) => markdown(child, context)).join('');
  }

  function fencedCode(value, language) {
    const longest = Math.max(0, ...Array.from(String(value).matchAll(/`+/g), (match) => match[0].length));
    const fence = '`'.repeat(Math.max(3, longest + 1));
    return '\n\n' + fence + language + '\n' + String(value).replace(/\n$/, '') + '\n' + fence + '\n\n';
  }

  function listMarkdown(node, ordered) {
    const items = Array.from(node.children).filter((child) => child.tagName === 'LI');
    return '\n' + items.map((item, index) => {
      const marker = ordered ? String(index + 1) + '. ' : '- ';
      const value = childrenMarkdown(item, { inList: true }).trim().replace(/\n/g, '\n  ');
      return marker + value;
    }).join('\n') + '\n\n';
  }

  function tableMarkdown(node) {
    const rows = Array.from(node.querySelectorAll('tr')).map((row) =>
      Array.from(row.querySelectorAll(':scope > th, :scope > td')).map((cell) =>
        cleanText(childrenMarkdown(cell, {})).replace(/\|/g, '\\|')
      )
    ).filter((row) => row.length);
    if (!rows.length) return '';
    const width = Math.max(...rows.map((row) => row.length));
    const normalized = rows.map((row) => Array.from({ length: width }, (_, index) => row[index] || ''));
    const header = normalized[0];
    const body = normalized.slice(1);
    return '\n\n| ' + header.join(' | ') + ' |\n| ' + header.map(() => '---').join(' | ') + ' |\n'
      + body.map((row) => '| ' + row.join(' | ') + ' |').join('\n') + '\n\n';
  }

  function markdown(node, context) {
    if (node.nodeType === Node.TEXT_NODE) return escapeMarkdown(node.nodeValue);
    if (node.nodeType !== Node.ELEMENT_NODE) return '';
    const tag = node.tagName;
    if (tag === 'BR') return '\n';
    if (tag === 'HR') return '\n\n---\n\n';
    if (tag === 'PRE') {
      const code = node.querySelector('code') || node;
      const match = String(code.className || '').match(/language-([A-Za-z0-9_+-]+)/);
      return fencedCode(code.textContent || '', match ? match[1] : '');
    }
    if (tag === 'CODE') {
      const value = String(node.textContent || '').replace(/`/g, '\\`');
      return '`' + value + '`';
    }
    if (node.matches('.katex, [data-testid*="math" i]')) {
      const annotation = node.querySelector('annotation[encoding="application/x-tex"]');
      const value = cleanText(annotation ? annotation.textContent : node.textContent);
      return value ? '`' + value.replace(/`/g, '\\`') + '`' : '';
    }
    if (tag === 'STRONG' || tag === 'B') return '**' + childrenMarkdown(node, context) + '**';
    if (tag === 'EM' || tag === 'I') return '*' + childrenMarkdown(node, context) + '*';
    if (tag === 'S' || tag === 'DEL') return '~~' + childrenMarkdown(node, context) + '~~';
    if (tag === 'A') {
      const text = childrenMarkdown(node, context) || escapeMarkdown(node.textContent || '链接');
      try {
        const url = new URL(node.getAttribute('href') || '', location.origin);
        return url.protocol === 'http:' || url.protocol === 'https:' ? '[' + text + '](' + url.href + ')' : text;
      } catch {
        return text;
      }
    }
    if (/^H[1-6]$/.test(tag)) {
      return '\n\n' + '#'.repeat(Number(tag.slice(1))) + ' ' + childrenMarkdown(node, context).trim() + '\n\n';
    }
    if (tag === 'UL') return listMarkdown(node, false);
    if (tag === 'OL') return listMarkdown(node, true);
    if (tag === 'TABLE') return tableMarkdown(node);
    if (tag === 'BLOCKQUOTE') {
      return '\n\n' + childrenMarkdown(node, context).trim().split('\n').map((line) => '> ' + line).join('\n') + '\n\n';
    }
    if (tag === 'DETAILS') {
      const summary = node.querySelector(':scope > summary');
      const title = cleanText(summary ? summary.textContent : '详情');
      const body = Array.from(node.childNodes)
        .filter((child) => child !== summary)
        .map((child) => markdown(child, context)).join('').trim();
      return '\n\n**' + escapeMarkdown(title) + '**\n\n' + body + '\n\n';
    }
    if (tag === 'DL') {
      return '\n\n' + Array.from(node.children).map((child) => {
        const value = childrenMarkdown(child, context).trim();
        return child.tagName === 'DT' ? '**' + value + '**' : ': ' + value;
      }).join('\n') + '\n\n';
    }
    if (tag === 'INPUT' && String(node.getAttribute('type')).toLowerCase() === 'checkbox') {
      return node.checked ? '[x] ' : '[ ] ';
    }
    if (tag === 'SUP') return '^' + childrenMarkdown(node, context) + '^';
    if (tag === 'SUB') return '~' + childrenMarkdown(node, context) + '~';
    if (tag === 'MARK') return '**' + childrenMarkdown(node, context) + '**';
    if (tag === 'P') return childrenMarkdown(node, context).trim() + '\n\n';
    if (tag === 'LI' && !context.inList) return '- ' + childrenMarkdown(node, { inList: true }).trim() + '\n';
    if (tag === 'IMG') {
      const alt = cleanText(node.getAttribute('alt'));
      return alt ? '[图片：' + escapeMarkdown(alt) + ']' : '[图片]';
    }
    return childrenMarkdown(node, context);
  }

  function messageRole(node) {
    const own = node.getAttribute('data-message-author-role');
    const nested = node.querySelector('[data-message-author-role]');
    const role = own || (nested && nested.getAttribute('data-message-author-role')) || '';
    return role === 'user' || role === 'assistant' ? role : '';
  }

  function roleNode(node) {
    return node.matches('[data-message-author-role]')
      ? node
      : node.querySelector('[data-message-author-role]') || node;
  }

  function contentNode(node) {
    const owner = roleNode(node);
    return owner.querySelector('.markdown, [data-message-content], .whitespace-pre-wrap') || owner;
  }

  function structuredLabel(node, fallback) {
    return cleanText([
      node.getAttribute('aria-label'),
      node.getAttribute('title'),
      node.getAttribute('alt'),
      node.getAttribute('download'),
      node.textContent
    ].filter(Boolean).join(' ')).slice(0, 180) || fallback;
  }

  function linkPart(node) {
    let path = '';
    try { path = new URL(node.getAttribute('href') || '', location.origin).pathname; }
    catch { return null; }
    const metadata = cleanText([
      node.getAttribute('data-testid'),
      node.getAttribute('aria-label'),
      node.getAttribute('title')
    ].filter(Boolean).join(' ')).toLowerCase();
    const isFile = !!node.getAttribute('download') ||
      FILE_PATH_EXTENSION.test(path) ||
      /download|file|attachment/.test(metadata);
    return {
      type: isFile ? 'file' : 'citation',
      text: structuredLabel(node, isFile ? '文件' : '引用')
    };
  }

  function structuredParts(content) {
    const parts = [];
    const seen = new Set();
    function add(type, label, node) {
      const key = type + '|' + label;
      if (seen.has(key) || parts.length >= MAX_STRUCTURED_PARTS || !isVisible(node)) return;
      seen.add(key);
      parts.push({ type, text: label });
      lastStructuredTypes.add(type);
      if (COMPLEX_PART_TYPES.has(type)) lastComplexOutput = true;
    }
    Array.from(content.querySelectorAll('img')).forEach((node) => {
      add('image', structuredLabel(node, '图片'), node);
    });
    Array.from(content.querySelectorAll('a[href]')).forEach((node) => {
      const part = linkPart(node);
      if (part) add(part.type, part.text, node);
    });
    Array.from(content.querySelectorAll(
      '[data-testid*="artifact" i], [data-testid*="canvas" i], [data-testid*="code-interpreter" i], iframe'
    )).forEach((node) => {
      add('artifact', structuredLabel(node, '交互内容'), node);
    });
    Array.from(content.querySelectorAll('audio')).forEach((node) => {
      add('audio', structuredLabel(node, '音频'), node);
    });
    Array.from(content.querySelectorAll('video')).forEach((node) => {
      add('video', structuredLabel(node, '视频'), node);
    });
    Array.from(content.querySelectorAll('.katex, [data-testid*="math" i]')).forEach((node) => {
      const annotation = node.querySelector('annotation[encoding="application/x-tex"]');
      add('math', structuredLabel(annotation || node, '数学公式'), node);
    });
    Array.from(content.querySelectorAll(
      'canvas, .mermaid, [data-testid*="chart" i], [data-testid*="diagram" i], [aria-label*="chart" i], [aria-label*="图表"]'
    )).forEach((node) => {
      add('chart', structuredLabel(node, '图表'), node);
    });
    Array.from(content.querySelectorAll(
      '[data-testid*="map" i], [aria-label*="map" i], [aria-label*="地图"]'
    )).forEach((node) => {
      add('map', structuredLabel(node, '地图'), node);
    });
    Array.from(content.querySelectorAll(
      'details, [role="tree"], [role="grid"], [data-testid*="interactive" i], '
      + '[data-testid*="output" i], [data-testid*="viewer" i], [data-testid*="preview" i]'
    )).forEach((node) => {
      add('interactive', structuredLabel(node, '交互内容'), node);
    });
    return parts;
  }

  function messageContent(node, role) {
    const content = contentNode(node);
    if (role === 'assistant' && content.querySelector('table, pre, blockquote, ol, ul')) {
      lastComplexOutput = true;
    }
    const value = role === 'assistant'
      ? cleanText(childrenMarkdown(content, {}))
      : cleanText(content.innerText || content.textContent);
    return value.slice(0, MAX_MESSAGE_LENGTH);
  }

  function messageNodes() {
    const main = document.querySelector('main');
    if (!main) return [];
    const turns = Array.from(main.querySelectorAll('[data-testid^="conversation-turn-"]'));
    return turns.length ? turns : Array.from(main.querySelectorAll('[data-message-author-role]'));
  }

  function readMessageWindow(streaming) {
    const nodes = messageNodes();
    const startIndex = Math.max(0, nodes.length - MAX_MESSAGES);
    const seen = new Set();
    lastStructuredTypes = new Set();
    lastComplexOutput = false;
    const messages = nodes.slice(startIndex).map((node, index) => {
      const role = messageRole(node);
      const text = messageContent(node, role);
      const parts = structuredParts(contentNode(node));
      const globalIndex = startIndex + index;
      const baseId = node.getAttribute('data-message-id')
        || node.getAttribute('data-testid')
        || node.id
        || role + '-' + globalIndex;
      const id = seen.has(baseId) ? baseId + '-' + globalIndex : baseId;
      seen.add(id);
      return {
        id,
        role,
        state: 'completed',
        content: [{ type: role === 'assistant' ? 'markdown' : 'text', text }].concat(parts)
      };
    }).filter((message) => message.role && (message.content[0].text || message.content.length > 1));
    if (streaming && messages.length && messages[messages.length - 1].role === 'assistant') {
      messages[messages.length - 1].state = 'streaming';
    }
    return { messages, observedCount: nodes.length, startIndex };
  }

  function readMessages(streaming) {
    return readMessageWindow(streaming).messages;
  }

  function lastAssistantTurn() {
    return messageNodes().reverse().find((node) => messageRole(node) === 'assistant') || null;
  }

  function regenerateButton() {
    const turn = lastAssistantTurn();
    if (!turn) return null;
    return Array.from(turn.querySelectorAll('button, [role="button"]')).find((button) =>
      isVisible(button) && messageActionPolicy && messageActionPolicy.isRegenerateControl(button)
    ) || null;
  }

  function messageOverflowButton() {
    const turn = lastAssistantTurn();
    if (!turn) return null;
    return Array.from(turn.querySelectorAll('button, [role="button"]')).find((button) =>
      isVisible(button) && messageActionPolicy && messageActionPolicy.isOverflowControl(button)
    ) || null;
  }

  function visibleRegenerateMenuItem() {
    if (!messageActionPolicy) return null;
    return Array.from(document.querySelectorAll(
      '[role="menuitem"], [role="option"], [data-radix-menu-content] button, [data-headlessui-portal] button'
    )).find((item) => isVisible(item) && messageActionPolicy.isRegenerateControl(item)) || null;
  }

  function dismissOverflowMenu() {
    document.dispatchEvent(new KeyboardEvent('keydown', {
      key: 'Escape',
      code: 'Escape',
      bubbles: true,
      cancelable: true
    }));
  }

  function regenerate(result) {
    const button = regenerateButton();
    if (button) {
      button.click();
      return result('regenerate_response', true, '');
    }
    const overflow = messageOverflowButton();
    if (!overflow) return result('regenerate_response', false, '官网当前没有可用的重新生成入口。');
    overflow.click();
    const deadline = Date.now() + 2000;
    function invokeMenuItem() {
      const item = visibleRegenerateMenuItem();
      if (item) {
        item.click();
        return result('regenerate_response', true, '');
      }
      if (Date.now() >= deadline) {
        dismissOverflowMenu();
        return result('regenerate_response', false, '官网消息菜单中没有可用的重新生成入口。');
      }
      window.setTimeout(invokeMenuItem, 75);
    }
    window.setTimeout(invokeMenuItem, 50);
  }

  function capabilities() {
    const values = ['message_copy', 'rich_text'];
    if (regenerateButton() || messageOverflowButton()) values.push('message_regenerate');
    if (lastStructuredTypes.size || lastComplexOutput) values.push('complex_output');
    return values;
  }

  window.__elonChatGptMessages = Object.freeze({
    capabilities,
    readMessages,
    readMessageWindow,
    regenerate
  });
})();
