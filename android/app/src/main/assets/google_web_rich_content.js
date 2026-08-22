(function (root, factory) {
  'use strict';

  const api = factory();
  if (typeof module === 'object' && module.exports) module.exports = api;
  if (root && (!root.__elonGoogleWebRichContent ||
      Number(root.__elonGoogleWebRichContent.version || 0) < api.version)) {
    root.__elonGoogleWebRichContent = Object.freeze(api);
  }
})(typeof window !== 'undefined' ? window : null, function () {
  'use strict';

  const MAX_BLOCKS = 160;
  const MAX_MARKDOWN_CHARS = 40000;
  const RICH_SCHEMA = 'yilong.rich-content.v1';
  const BLOCK_TAGS = new Set([
    'ARTICLE', 'ASIDE', 'BLOCKQUOTE', 'DIV', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6',
    'LI', 'OL', 'P', 'PRE', 'SECTION', 'TABLE', 'UL'
  ]);
  const SKIP_TAGS = new Set([
    'BUTTON', 'CANVAS', 'FORM', 'INPUT', 'NAV', 'NOSCRIPT', 'OPTION', 'SCRIPT',
    'SELECT', 'STYLE', 'SVG', 'TEMPLATE', 'TEXTAREA'
  ]);
  const SKIP_ROLES = new Set([
    'button', 'dialog', 'menu', 'menuitem', 'navigation', 'tab', 'tablist', 'toolbar'
  ]);

  function cleanText(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[ \t]+\n/g, '\n')
      .replace(/\n{3,}/g, '\n\n')
      .trim();
  }

  function cleanInline(value) {
    return String(value || '')
      .replace(/\u00a0/g, ' ')
      .replace(/[\s\u200b]+/g, ' ')
      .trim();
  }

  function escapeInline(value) {
    return cleanInline(value).replace(/([\\`*_[\]<>|])/g, '\\$1');
  }

  function escapeInlineFragment(value) {
    const raw = String(value || '').replace(/\u00a0/g, ' ');
    const leading = /^\s/.test(raw) ? ' ' : '';
    const trailing = /\s$/.test(raw) ? ' ' : '';
    const text = raw.replace(/[\s\u200b]+/g, ' ').trim()
      .replace(/([\\`*_[\]<>|])/g, '\\$1');
    return text ? leading + text + trailing : (leading || trailing);
  }

  function escapeTableCell(value) {
    return escapeInline(value).replace(/\r?\n/g, '<br>');
  }

  function safeLanguage(value) {
    const language = cleanInline(value).toLowerCase();
    return /^[a-z0-9_+.#-]{1,32}$/.test(language) ? language : '';
  }

  function blockInline(block) {
    return cleanText(block && block.markdown) || escapeInline(block && block.text);
  }

  function renderBlocks(input) {
    const blocks = Array.isArray(input) ? input.slice(0, MAX_BLOCKS) : [];
    const rendered = [];
    for (const block of blocks) {
      if (!block || !block.type) continue;
      if (block.type === 'heading') {
        const text = blockInline(block);
        if (text) rendered.push('#'.repeat(Math.min(6, Math.max(1, Number(block.level) || 2))) + ' ' + text);
      } else if (block.type === 'paragraph') {
        const text = blockInline(block);
        if (text) rendered.push(text);
      } else if (block.type === 'quote') {
        const text = blockInline(block);
        if (text) rendered.push(text.split(/\r?\n/).map((line) => '> ' + line).join('\n'));
      } else if (block.type === 'list') {
        const items = Array.isArray(block.items) ? block.items.slice(0, 60) : [];
        const lines = items.map((item, index) => {
          const text = typeof item === 'object' ? blockInline(item) : escapeInline(item);
          return text ? ((block.ordered ? String(index + 1) + '.' : '-') + ' ' + text) : '';
        }).filter(Boolean);
        if (lines.length) rendered.push(lines.join('\n'));
      } else if (block.type === 'table') {
        const rows = Array.isArray(block.rows) ? block.rows.slice(0, 40) : [];
        const width = Math.min(12, rows.reduce((max, row) =>
          Math.max(max, Array.isArray(row) ? row.length : 0), 0));
        if (rows.length && width) {
          const normalized = rows.map((row) => Array.from({ length: width }, (_, index) =>
            escapeTableCell(Array.isArray(row) ? row[index] : '')
          ));
          const lines = [
            '| ' + normalized[0].join(' | ') + ' |',
            '| ' + Array.from({ length: width }, () => '---').join(' | ') + ' |'
          ];
          normalized.slice(1).forEach((row) => lines.push('| ' + row.join(' | ') + ' |'));
          rendered.push(lines.join('\n'));
        }
      } else if (block.type === 'code') {
        const text = cleanText(block.text);
        if (text) {
          const longestFence = Math.max(3, ...Array.from(text.matchAll(/`+/g), (match) => match[0].length + 1));
          const fence = '`'.repeat(longestFence);
          rendered.push(fence + safeLanguage(block.language) + '\n' + text + '\n' + fence);
        }
      }
    }
    return rendered.join('\n\n').replace(/\n{3,}/g, '\n\n').slice(0, MAX_MARKDOWN_CHARS).trim();
  }

  function boundaryText(value) {
    const text = cleanInline(value).toLowerCase().replace(/[：:]+$/, '');
    return /^(?:related|web|more)\s+results?$/.test(text) ||
      /^(?:相关|网页|更多)(?:搜索)?结果$/.test(text);
  }

  function blockPlainText(block) {
    if (!block) return '';
    if (block.type === 'list') return cleanInline((block.items || []).map((item) =>
      typeof item === 'object' ? item.text || item.markdown : item
    ).join(' '));
    if (block.type === 'table') return cleanInline((block.rows || []).flat().join(' '));
    return cleanInline(block.text || block.markdown);
  }

  function trimBlocks(input, query) {
    const blocks = [];
    const normalizedQuery = cleanInline(query);
    for (const original of Array.isArray(input) ? input : []) {
      const plain = blockPlainText(original);
      if (boundaryText(plain)) break;
      if (!plain) continue;
      if (!blocks.length && normalizedQuery && plain === normalizedQuery) continue;
      if (!blocks.length && normalizedQuery && original.type === 'paragraph' &&
          plain.startsWith(normalizedQuery + ' ')) {
        const remaining = plain.slice(normalizedQuery.length).trim();
        if (remaining) blocks.push({ type: 'paragraph', text: remaining });
        continue;
      }
      blocks.push(original);
    }
    return blocks;
  }

  function partsFromBlocks(blocks, fallbackText, query) {
    const trimmed = trimBlocks(blocks, query);
    const richPart = weatherPart(trimmed);
    const proseBlocks = richPart
      ? trimmed.filter((block) => block !== richPart.sourceBlock)
      : trimmed;
    const markdown = renderBlocks(proseBlocks);
    const parts = markdown ? [{ type: 'markdown', text: markdown }] : [];
    if (richPart) parts.push(richPart.part);
    if (parts.length) return parts;
    const fallback = cleanText(fallbackText).slice(0, MAX_MARKDOWN_CHARS);
    return fallback ? [{ type: 'text', text: fallback }] : [];
  }

  function weatherPart(blocks) {
    const tableIndex = blocks.findIndex((block) => {
      if (!block || block.type !== 'table' || !Array.isArray(block.rows) || !block.rows.length) return false;
      const header = block.rows[0].map(cleanInline).join(' ').toLowerCase();
      return /(?:时间|時間|time)/.test(header) &&
        /(?:天气|天氣|weather|condition)/.test(header) &&
        /(?:气温|氣溫|温度|溫度|temperature|temp)/.test(header);
    });
    if (tableIndex < 0) return null;
    const table = blocks[tableIndex];
    const header = table.rows[0].map((value) => cleanInline(value).toLowerCase());
    const indexFor = (pattern) => header.findIndex((value) => pattern.test(value));
    const periodIndex = indexFor(/时间|時間|time/);
    const conditionIndex = indexFor(/天气|天氣|weather|condition/);
    const temperatureIndex = indexFor(/气温|氣溫|温度|溫度|temperature|temp/);
    const precipitationIndex = indexFor(/降水|降雨|precipitation|rain/);
    const windIndex = indexFor(/风|風|wind/);
    const rows = table.rows.slice(1, 25).map((row) => ({
      period: cleanInline(row[periodIndex]),
      condition: cleanInline(row[conditionIndex]),
      temperature: cleanInline(row[temperatureIndex]),
      precipitation: precipitationIndex >= 0 ? cleanInline(row[precipitationIndex]) : '',
      wind: windIndex >= 0 ? cleanInline(row[windIndex]) : ''
    })).filter((row) => row.period && row.condition && row.temperature);
    if (!rows.length) return null;
    const heading = blocks.slice(0, tableIndex).reverse().find((block) => block.type === 'heading');
    const title = cleanInline(heading && (heading.text || heading.markdown)) || '天气预报';
    return {
      sourceBlock: table,
      part: {
        type: 'rich_card',
        text: title,
        kind: 'weather',
        richContent: {
          schema: RICH_SCHEMA,
          kind: 'weather',
          source: 'official_dom',
          payload: { title, rows }
        }
      }
    };
  }

  function elementVisible(element) {
    if (!element || element.nodeType !== 1) return true;
    if (element.hidden || element.hasAttribute('inert') ||
        element.getAttribute('aria-hidden') === 'true') return false;
    const view = element.ownerDocument && element.ownerDocument.defaultView;
    if (view && typeof view.getComputedStyle === 'function') {
      const style = view.getComputedStyle(element);
      if (style.display === 'none' || style.visibility === 'hidden' ||
          style.visibility === 'collapse' || Number(style.opacity) === 0) return false;
    }
    return true;
  }

  function excludedElement(element) {
    if (!element || element.nodeType !== 1) return false;
    return SKIP_TAGS.has(element.tagName) || SKIP_ROLES.has(String(element.getAttribute('role') || '').toLowerCase()) ||
      !elementVisible(element);
  }

  function safePublicUrl(value) {
    try {
      const url = new URL(String(value || ''));
      if (url.protocol !== 'https:' || url.username || url.password) return '';
      if (url.origin === 'https://google.com' || url.origin === 'https://www.google.com') return '';
      return (url.origin + url.pathname).slice(0, 1200);
    } catch (_) {
      return '';
    }
  }

  function inlineMarkdown(node, skipNestedLists) {
    if (!node) return '';
    if (node.nodeType === 3) return escapeInlineFragment(node.nodeValue);
    if (node.nodeType !== 1 || excludedElement(node)) return '';
    const tag = node.tagName;
    if (tag === 'BR') return '\n';
    if (skipNestedLists && (tag === 'UL' || tag === 'OL' || node.getAttribute('role') === 'list')) return '';
    if (tag === 'IMG') return escapeInline(node.getAttribute('alt') || '');
    const children = Array.from(node.childNodes || [])
      .map((child) => inlineMarkdown(child, skipNestedLists)).join('')
      .replace(/[ \t]*\n[ \t]*/g, '\n')
      .replace(/[ \t]{2,}/g, ' ')
      .trim();
    if (!children) return '';
    if (tag === 'STRONG' || tag === 'B') return '**' + children + '**';
    if (tag === 'EM' || tag === 'I') return '*' + children + '*';
    if (tag === 'CODE' && (!node.parentElement || node.parentElement.tagName !== 'PRE')) return '`' + children + '`';
    if (tag === 'A') {
      const url = safePublicUrl(node.href || node.getAttribute('href'));
      return url ? '[' + children.replace(/[\[\]]/g, '\\$&') + '](' + url + ')' : children;
    }
    if (tag === 'MATH' || node.getAttribute('role') === 'math') {
      return children.includes('$') ? children : '$' + children + '$';
    }
    return children;
  }

  function roleOf(element) {
    return String(element && element.getAttribute && element.getAttribute('role') || '').toLowerCase();
  }

  function blockElement(element) {
    const role = roleOf(element);
    return !!element && element.nodeType === 1 &&
      (BLOCK_TAGS.has(element.tagName) || ['heading', 'list', 'table'].includes(role));
  }

  function directListItems(element) {
    return Array.from(element.children || []).filter((child) =>
      child.tagName === 'LI' || roleOf(child) === 'listitem'
    );
  }

  function tableRows(element) {
    return Array.from(element.querySelectorAll('tr, [role="row"]')).slice(0, 40)
      .map((row) => Array.from(row.children || [])
        .filter((cell) => ['TH', 'TD'].includes(cell.tagName) ||
          ['cell', 'columnheader', 'rowheader'].includes(roleOf(cell)))
        .slice(0, 12)
        .map((cell) => cleanInline(cell.innerText || cell.textContent)))
      .filter((row) => row.length);
  }

  function languageFor(element) {
    const className = String(element.className || '') + ' ' +
      String(element.querySelector('code') && element.querySelector('code').className || '');
    const match = className.match(/(?:language|lang)-([a-z0-9_+.#-]{1,32})/i);
    return match ? safeLanguage(match[1]) : '';
  }

  function addBlock(blocks, block) {
    if (!block || blocks.length >= MAX_BLOCKS || !blockPlainText(block)) return;
    const previous = blocks[blocks.length - 1];
    if (previous && previous.type === block.type && blockPlainText(previous) === blockPlainText(block)) return;
    blocks.push(block);
  }

  function collect(element, blocks) {
    if (!element || element.nodeType !== 1 || excludedElement(element) || blocks.length >= MAX_BLOCKS) return;
    const tag = element.tagName;
    const role = roleOf(element);
    if (/^H[1-6]$/.test(tag) || role === 'heading') {
      addBlock(blocks, {
        type: 'heading',
        level: Number(element.getAttribute('aria-level')) || Number(tag.slice(1)) || 2,
        markdown: inlineMarkdown(element)
      });
      return;
    }
    if (tag === 'UL' || tag === 'OL' || role === 'list') {
      const items = directListItems(element).map((item) => ({
        markdown: inlineMarkdown(item, true),
        text: cleanInline(item.innerText || item.textContent)
      })).filter((item) => item.markdown || item.text);
      addBlock(blocks, { type: 'list', ordered: tag === 'OL', items });
      return;
    }
    if (tag === 'TABLE' || role === 'table') {
      addBlock(blocks, { type: 'table', rows: tableRows(element) });
      return;
    }
    if (tag === 'PRE') {
      addBlock(blocks, {
        type: 'code',
        language: languageFor(element),
        text: cleanText(element.innerText || element.textContent)
      });
      return;
    }
    if (tag === 'P' || tag === 'BLOCKQUOTE') {
      addBlock(blocks, {
        type: tag === 'BLOCKQUOTE' ? 'quote' : 'paragraph',
        markdown: inlineMarkdown(element)
      });
      return;
    }

    const children = Array.from(element.children || []).filter((child) => !excludedElement(child));
    if (!children.some(blockElement)) {
      addBlock(blocks, { type: 'paragraph', markdown: inlineMarkdown(element) });
      return;
    }
    for (const child of children) {
      if (blockElement(child)) collect(child, blocks);
      else addBlock(blocks, { type: 'paragraph', markdown: inlineMarkdown(child) });
      if (blocks.length >= MAX_BLOCKS) break;
    }
  }

  function parts(container, fallbackText, query) {
    try {
      const blocks = [];
      collect(container, blocks);
      return partsFromBlocks(blocks, fallbackText, query);
    } catch (_) {
      return partsFromBlocks([], fallbackText, query);
    }
  }

  return Object.freeze({
    version: 2,
    renderBlocks,
    partsFromBlocks,
    weatherPart,
    parts
  });
});
