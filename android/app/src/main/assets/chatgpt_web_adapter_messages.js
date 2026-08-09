(function () {
  'use strict';

  if (window.__elonChatGptMessages || location.origin !== 'https://chatgpt.com') return;

  const MAX_MESSAGES = 80;
  const MAX_MESSAGE_LENGTH = 40000;

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

  function messageContent(node, role) {
    const owner = roleNode(node);
    const content = owner.querySelector('.markdown, [data-message-content], .whitespace-pre-wrap') || owner;
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

  function readMessages(streaming) {
    const seen = new Set();
    const messages = messageNodes().slice(-MAX_MESSAGES).map((node, index) => {
      const role = messageRole(node);
      const text = messageContent(node, role);
      const baseId = node.getAttribute('data-message-id')
        || node.getAttribute('data-testid')
        || node.id
        || role + '-' + index;
      const id = seen.has(baseId) ? baseId + '-' + index : baseId;
      seen.add(id);
      return { id, role, state: 'completed', content: [{ type: role === 'assistant' ? 'markdown' : 'text', text }] };
    }).filter((message) => message.role && message.content[0].text);
    if (streaming && messages.length && messages[messages.length - 1].role === 'assistant') {
      messages[messages.length - 1].state = 'streaming';
    }
    return messages;
  }

  function lastAssistantTurn() {
    return messageNodes().reverse().find((node) => messageRole(node) === 'assistant') || null;
  }

  function regenerateButton() {
    const turn = lastAssistantTurn();
    if (!turn) return null;
    const labels = ['regenerate', 'try again', '重新生成', '重试'];
    return Array.from(turn.querySelectorAll('button')).find((button) => {
      if (!isVisible(button)) return false;
      const value = cleanText([
        button.getAttribute('data-testid'),
        button.getAttribute('aria-label'),
        button.getAttribute('title'),
        button.textContent
      ].filter(Boolean).join(' ')).toLowerCase();
      return labels.some((label) => value.includes(label));
    }) || null;
  }

  function regenerate(result) {
    const button = regenerateButton();
    if (!button) return result('regenerate_response', false, '官网当前没有可用的重新生成入口。');
    button.click();
    result('regenerate_response', true, '');
  }

  function capabilities() {
    const values = ['message_copy', 'rich_text'];
    if (regenerateButton()) values.push('message_regenerate');
    return values;
  }

  window.__elonChatGptMessages = Object.freeze({ capabilities, readMessages, regenerate });
})();
