/* Display-only transforms. The immutable export and original URLs are never rewritten. */
(() => {
  'use strict';
  const markers = { link: '链接', channels: '视频号', image: '图片', video: '视频', audio: '语音', file: '文件', forward: '聊天记录' };
  function text(row, cards = []) {
    let value = row.text.trim();
    const prefix = markers[row.kind] && `[${markers[row.kind]}]`;
    const exported = !!prefix && value.startsWith(prefix);
    if (exported) value = value.slice(prefix.length).trim();
    if (exported && row.filename && value === row.filename) return '';
    for (const card of cards) value = value.split(card.url).join('');
    // Only the exported title line is redundant. Commentary and unsupported links survive.
    return value.split('\n').filter(line => !(exported && cards.some(c => c.site !== '视频号' && c.title && line.trim() === c.title.trim()))).join('\n').trim();
  }
  function identity(sender) {
    const name = sender.trim() || '未知发送者';
    let hash = 0; for (let i = 0; i < name.length; i++) hash = (Math.imul(hash, 31) + name.charCodeAt(i)) | 0;
    const colors = ['#356859', '#65528a', '#376586', '#855040', '#536b32', '#80546f'];
    return { initial: Array.from(name)[0], color: colors[(hash >>> 0) % colors.length] };
  }
  const duration = seconds => Number.isFinite(seconds) && seconds > 0 ? `${Math.floor(seconds / 60)}:${String(Math.floor(seconds % 60)).padStart(2, '0')}` : '';
  globalThis.ElonRecordPresentation = { text, identity, duration };
})();
