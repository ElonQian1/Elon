/* Read-only snapshot DOM. Marked is used only as a lexer, never as an HTML renderer. */
(function (root) {
  'use strict';
  const el = (tag, text, cls) => {
    const node = document.createElement(tag);
    if (text != null) node.textContent = String(text);
    if (cls) node.className = cls;
    return node;
  };
  // Lucide 1.8.0 icon data; ISC/MIT notices in ai_share_icons.LICENSE.txt.
  const icons = {
    back: [['path', { d: 'm12 19-7-7 7-7' }], ['path', { d: 'M19 12H5' }]],
    search: [['path', { d: 'm21 21-4.34-4.34' }], ['circle', { cx: '11', cy: '11', r: '8' }]],
    retry: [['path', { d: 'M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8' }], ['path', { d: 'M21 3v5h-5' }]],
    close: [['path', { d: 'M18 6 6 18' }], ['path', { d: 'm6 6 12 12' }]],
  };
  function button(label, action, icon) {
    const node = el('button', icon ? null : label, 'ai-share-action');
    node.type = 'button'; node.title = label; node.setAttribute('aria-label', label); node.onclick = action;
    if (icons[icon]) {
      const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      Object.entries({ viewBox: '0 0 24 24', width: '24', height: '24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true' }).forEach(([k, v]) => svg.setAttribute(k, v));
      icons[icon].forEach(([tag, attrs]) => {
        const child = document.createElementNS(svg.namespaceURI, tag);
        Object.entries(attrs).forEach(([k, v]) => child.setAttribute(k, v)); svg.append(child);
      });
      node.append(svg);
    }
    return node;
  }
  function link(href) {
    try {
      const url = new URL(href);
      const credential = /(?:^|[?&#])(?:access_token|refresh_token|api_key|authorization|cookie|token|signature)=/i.test(url.search + url.hash);
      return ['https:', 'http:'].includes(url.protocol) && !url.username && !url.password && !credential ? url.href : null;
    } catch { return null; }
  }
  function decoded(text) {
    const named = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: '\u00a0' };
    return String(text || '').replace(/&(#x[\da-f]+|#\d+|amp|lt|gt|quot|apos|nbsp);/gi, (raw, value) => {
      if (value[0] !== '#') return named[value.toLowerCase()] || raw;
      const number = value[1].toLowerCase() === 'x' ? parseInt(value.slice(2), 16) : Number(value.slice(1));
      return number > 0 && number <= 0x10ffff ? String.fromCodePoint(number) : raw;
    });
  }
  function tokens(parent, items, depth = 0) {
    if (depth > 40) { parent.append(el('span', items.map(t => t.raw || t.text || '').join(''))); return; }
    (items || []).forEach(t => {
      let node;
      switch (t.type) {
        case 'space': return;
        case 'heading': node = el('h' + Math.min(6, Math.max(2, t.depth))); break;
        case 'paragraph': node = el('p'); break;
        case 'strong': node = el('strong'); break;
        case 'em': node = el('em'); break;
        case 'del': node = el('del'); break;
        case 'blockquote': node = el('blockquote'); break;
        case 'br': parent.append(el('br')); return;
        case 'hr': parent.append(el('hr')); return;
        case 'code': {
          const pre = el('pre'); pre.append(el('code', t.text));
          if (t.lang) pre.setAttribute('aria-label', t.lang.split(/\s/)[0]);
          parent.append(pre); return;
        }
        case 'codespan': parent.append(el('code', decoded(t.text))); return;
        case 'html': parent.append(document.createTextNode(t.raw || t.text || '')); return;
        case 'image': parent.append(el('span', decoded(t.text) || '图片未包含在分享中', 'ai-share-unavailable')); return;
        case 'link': {
          const href = link(t.href); node = el(href ? 'a' : 'span');
          if (href) { node.href = href; node.target = '_blank'; node.rel = 'noopener noreferrer'; node.referrerPolicy = 'no-referrer'; }
          break;
        }
        case 'list': {
          node = el(t.ordered ? 'ol' : 'ul'); if (t.ordered && t.start) node.start = t.start;
          t.items.forEach(item => {
            const li = el('li');
            if (item.task) { const check = el('input'); check.type = 'checkbox'; check.disabled = true; check.checked = !!item.checked; check.setAttribute('aria-label', item.checked ? '已完成' : '未完成'); li.append(check); }
            tokens(li, item.tokens, depth + 1); node.append(li);
          }); parent.append(node); return;
        }
        case 'table': {
          const wrap = el('div', null, 'ai-share-table'), table = el('table'), head = el('thead'), body = el('tbody');
          const row = (cells, header) => { const tr = el('tr'); cells.forEach(cell => { const td = el(header ? 'th' : 'td'); if (header) td.scope = 'col'; tokens(td, cell.tokens, depth + 1); tr.append(td); }); return tr; };
          head.append(row(t.header, true)); t.rows.forEach(cells => body.append(row(cells, false)));
          table.append(head, body); wrap.append(table); parent.append(wrap); return;
        }
        default:
          if (t.tokens) { tokens(parent, t.tokens, depth + 1); return; }
          parent.append(document.createTextNode(decoded(t.text || t.raw))); return;
      }
      if (t.tokens) tokens(node, t.tokens, depth + 1); else node.textContent = decoded(t.text);
      parent.append(node);
    });
  }
  function markdown(text) {
    const body = el('div', null, 'ai-share-markdown');
    try { tokens(body, root.marked.lexer(String(text || ''), { gfm: true, breaks: true })); }
    catch { body.textContent = text || ''; }
    return body;
  }
  function richCard(card) {
    const section = el('section', null, 'ai-share-rich-card');
    section.append(el('h3', card.title));
    if (card.description) section.append(el('p', card.description));
    const value = el('p', null, 'ai-share-value');
    if (card.symbol) value.append(el('span', card.symbol));
    if (card.primary_value) value.append(el('strong', card.primary_value));
    if (card.secondary_value) value.append(el('span', card.secondary_value));
    if (['positive', 'negative', 'neutral'].includes(card.trend)) value.dataset.trend = card.trend;
    section.append(value);
    const selected = (card.periods || []).filter(p => p.selected).map(p => p.label).join(' · ');
    if (selected) section.append(el('small', selected));
    const metrics = el('dl');
    (card.metrics || []).forEach(m => metrics.append(el('dt', m.label), el('dd', m.value)));
    section.append(metrics);
    const points = card.points || [], series = card.series || [];
    if (points.length && series.length) {
      const canvas = el('canvas', null, 'ai-share-chart'); canvas.width = 720; canvas.height = 240;
      canvas.setAttribute('role', 'img'); canvas.setAttribute('aria-label', card.title || '图表');
      const ctx = canvas.getContext('2d');
      const values = points.flatMap(p => p.values).filter(Number.isFinite), low = Math.min(...values), high = Math.max(...values);
      const colors = ['#8EA7D5', '#7FAFBA', '#D2B572', '#67BEA0'];
      if (ctx && values.length) series.forEach((_, index) => {
        ctx.beginPath(); ctx.strokeStyle = colors[index % colors.length]; ctx.lineWidth = 3;
        let started = false;
        points.forEach((p, i) => {
          if (!Number.isFinite(p.values[index])) { started = false; return; }
          const x = 12 + 696 * i / Math.max(1, points.length - 1), y = 224 - 208 * (p.values[index] - low) / (high - low || 1);
          if (started) ctx.lineTo(x, y); else { ctx.moveTo(x, y); started = true; }
        }); ctx.stroke();
      });
      const details = el('details'), table = el('table'), tr = el('tr');
      details.append(el('summary', '图表数据'));
      tr.append(el('th', '时间')); series.forEach(s => tr.append(el('th', s.label))); table.append(tr);
      points.forEach(p => {
        const row = el('tr'); row.append(el('th', p.label));
        series.forEach((s, i) => row.append(el('td', Number.isFinite(p.values[i]) ? (s.value_prefix || '') + p.values[i] + (s.value_suffix || '') : '')));
        table.append(row);
      });
      const wrap = el('div', null, 'ai-share-table'); wrap.append(table); details.append(wrap); section.append(canvas, details);
    }
    return section;
  }
  function part(part, media) {
    const section = el('section', null, 'ai-share-part');
    if (part.type === 'image') { section.append(media(part)); return section; }
    if (part.type === 'rich_card' && part.card) { section.append(richCard(part.card)); return section; }
    if (part.label) section.append(el('h3', part.label));
    if (part.text_block?.complete) {
      const block = part.text_block;
      if (block.title && block.title !== part.label) section.append(el('h3', block.title));
      if (part.type === 'code') { const pre = el('pre'); pre.append(el('code', block.content)); section.append(pre); }
      else section.append(markdown(block.content));
    } else if (['table', 'math'].includes(part.type) && part.caption) section.append(markdown(part.caption));
    else if (part.caption) section.append(el('p', part.caption));
    if (part.type === 'unavailable') section.classList.add('ai-share-unavailable');
    return section;
  }
  function message(message, media, senderName) {
    const block = el('article', null, 'ai-share-message');
    if (message.gap_before) block.append(el('p', '中间内容未分享', 'ai-share-gap'));
    const kind = message.role === 'user' ? 'user' : 'ai', row = el('div', null, 'bubble-row ' + kind);
    const bubble = el('div', null, 'bubble ' + kind), label = el('small', kind === 'user' ? (senderName || '分享者') : 'ChatGPT', 'ai-share-speaker');
    if (message.created_at_ms > 0) { const date = new Date(message.created_at_ms); if (!Number.isNaN(date.valueOf())) label.append(el('time', ' · ' + date.toLocaleString())); }
    bubble.append(label, markdown(message.content));
    (message.parts || []).forEach(p => bubble.append(part(p, media)));
    row.append(bubble); block.append(row); return block;
  }
  root.ElonAiShareRich = { el, button, markdown, message };
})(globalThis);
