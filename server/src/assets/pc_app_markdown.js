(function () {
  const kit = window.ElonPcKit || {};
  const escapeHtml = kit.escapeHtml || ((value) => String(value == null ? '' : value));
  const clean = kit.clean || ((value) => String(value == null ? '' : value).trim());

  function safeHref(value) {
    const raw = clean(value).replace(/[\u0000-\u001f\u007f\s]/g, '');
    try {
      const url = new URL(raw, window.location.href);
      if (['http:', 'https:', 'mailto:'].includes(url.protocol)) return url.toString();
    } catch (_) {}
    return '';
  }

  function tokenStore() {
    const values = [];
    return {
      put(html) {
        const key = `\u0000MD${values.length}\u0000`;
        values.push(html);
        return key;
      },
      restore(value) {
        return String(value).replace(/\u0000MD(\d+)\u0000/g, (_, index) => values[Number(index)] || '');
      }
    };
  }

  function renderInline(value) {
    const tokens = tokenStore();
    let text = String(value == null ? '' : value);
    text = text.replace(/`([^`]+)`/g, (_, code) => tokens.put(`<code>${escapeHtml(code)}</code>`));
    text = text.replace(/\[([^\]\n]+)\]\(([^)\n]+)\)/g, (_, label, href) => {
      const safe = safeHref(href);
      if (!safe) return `${label} (${href})`;
      return tokens.put(`<a href="${escapeHtml(safe)}" target="_blank" rel="noopener noreferrer">${escapeHtml(label)}</a>`);
    });
    text = escapeHtml(text);
    text = text.replace(/\*\*([^*\n]+)\*\*/g, '<strong>$1</strong>');
    text = text.replace(/(^|[\s(])\*([^*\n]+)\*/g, '$1<em>$2</em>');
    return tokens.restore(text);
  }

  function splitCells(line) {
    return line
      .replace(/^\s*\|/, '')
      .replace(/\|\s*$/, '')
      .split('|')
      .map((cell) => cell.trim());
  }

  function isTableSeparator(line) {
    return /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)+\|?\s*$/.test(line || '');
  }

  function renderTable(lines, start) {
    const headers = splitCells(lines[start]);
    const rows = [];
    let index = start + 2;
    while (index < lines.length && /\|/.test(lines[index]) && clean(lines[index])) {
      rows.push(splitCells(lines[index]));
      index += 1;
    }
    const head = `<thead><tr>${headers.map((cell) => `<th>${renderInline(cell)}</th>`).join('')}</tr></thead>`;
    const body = `<tbody>${rows.map((row) => `<tr>${headers.map((_, cellIndex) => `<td>${renderInline(row[cellIndex] || '')}</td>`).join('')}</tr>`).join('')}</tbody>`;
    return { html: `<div class="markdown-table-wrap"><table>${head}${body}</table></div>`, next: index };
  }

  function renderList(lines, start, ordered) {
    const tag = ordered ? 'ol' : 'ul';
    const pattern = ordered ? /^\s*\d+\.\s+(.+)$/ : /^\s*[-*+]\s+(.+)$/;
    const items = [];
    let index = start;
    while (index < lines.length) {
      const match = lines[index].match(pattern);
      if (!match) break;
      items.push(`<li>${renderInline(match[1])}</li>`);
      index += 1;
    }
    return { html: `<${tag}>${items.join('')}</${tag}>`, next: index };
  }

  function renderBlockquote(lines, start) {
    const parts = [];
    let index = start;
    while (index < lines.length && /^\s*>\s?/.test(lines[index])) {
      parts.push(lines[index].replace(/^\s*>\s?/, ''));
      index += 1;
    }
    return { html: `<blockquote>${renderBlocks(parts)}</blockquote>`, next: index };
  }

  function renderParagraph(lines, start) {
    const parts = [];
    let index = start;
    while (index < lines.length) {
      const line = lines[index];
      if (!clean(line)) break;
      if (/^\s*(```|#{1,6}\s+|[-*+]\s+|\d+\.\s+|>\s?)/.test(line)) break;
      if (index + 1 < lines.length && /\|/.test(line) && isTableSeparator(lines[index + 1])) break;
      parts.push(line);
      index += 1;
    }
    return { html: `<p>${parts.map(renderInline).join('<br>')}</p>`, next: index };
  }

  function renderBlocks(inputLines) {
    const lines = inputLines || [];
    const html = [];
    let index = 0;
    while (index < lines.length) {
      const line = lines[index];
      if (!clean(line)) {
        index += 1;
        continue;
      }
      const fence = line.match(/^\s*```([A-Za-z0-9_-]+)?\s*$/);
      if (fence) {
        const code = [];
        index += 1;
        while (index < lines.length && !/^\s*```\s*$/.test(lines[index])) {
          code.push(lines[index]);
          index += 1;
        }
        if (index < lines.length) index += 1;
        html.push(`<pre><code>${escapeHtml(code.join('\n'))}</code></pre>`);
        continue;
      }
      const heading = line.match(/^\s*(#{1,6})\s+(.+)$/);
      if (heading) {
        const level = Math.min(4, Math.max(3, heading[1].length + 2));
        html.push(`<h${level}>${renderInline(heading[2])}</h${level}>`);
        index += 1;
        continue;
      }
      if (index + 1 < lines.length && /\|/.test(line) && isTableSeparator(lines[index + 1])) {
        const table = renderTable(lines, index);
        html.push(table.html);
        index = table.next;
        continue;
      }
      if (/^\s*[-*+]\s+/.test(line)) {
        const list = renderList(lines, index, false);
        html.push(list.html);
        index = list.next;
        continue;
      }
      if (/^\s*\d+\.\s+/.test(line)) {
        const list = renderList(lines, index, true);
        html.push(list.html);
        index = list.next;
        continue;
      }
      if (/^\s*>\s?/.test(line)) {
        const quote = renderBlockquote(lines, index);
        html.push(quote.html);
        index = quote.next;
        continue;
      }
      const paragraph = renderParagraph(lines, index);
      html.push(paragraph.html);
      index = paragraph.next;
    }
    return html.join('');
  }

  function render(markdown) {
    const text = String(markdown == null ? '' : markdown).replace(/\r\n?/g, '\n');
    return renderBlocks(text.split('\n'));
  }

  function renderMessage(markdown, options) {
    const opts = options || {};
    const className = clean(opts.className);
    const raw = String(markdown == null ? '' : markdown);
    const body = opts.markdown === false ? escapeHtml(raw) : render(raw);
    const copyButton = opts.copy
      ? `<button class="message-copy" type="button" data-copy-markdown="${escapeHtml(raw)}" title="复制 Markdown 原文">复制</button>`
      : '';
    return `<div class="message-content markdown-body ${escapeHtml(className)}">${copyButton}<div class="markdown-rendered">${body}</div></div>`;
  }

  async function copyText(text) {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return;
    }
    const textarea = document.createElement('textarea');
    textarea.value = text;
    textarea.setAttribute('readonly', 'readonly');
    textarea.style.position = 'fixed';
    textarea.style.left = '-9999px';
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand('copy');
    textarea.remove();
  }

  function bindCopyButtons(root) {
    const scope = root || document;
    scope.querySelectorAll('[data-copy-markdown]').forEach((button) => {
      if (button.dataset.copyBound) return;
      button.dataset.copyBound = '1';
      button.addEventListener('click', async () => {
        const original = button.textContent;
        try {
          await copyText(button.dataset.copyMarkdown || '');
          button.textContent = '已复制';
          setTimeout(() => { button.textContent = original || '复制'; }, 1300);
        } catch (_) {
          button.textContent = '复制失败';
          setTimeout(() => { button.textContent = original || '复制'; }, 1300);
        }
      });
    });
  }

  window.ElonPcMarkdown = {
    render,
    renderMessage,
    bindCopyButtons
  };
})();
