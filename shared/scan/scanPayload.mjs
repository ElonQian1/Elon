// Public, side-effect-free scan semantics. Never navigate while decoding.
export const MAX_SCAN_LENGTH = 8192;
const accountId = /^(?:[a-f0-9]{32}|[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}|usr_[a-z0-9_-]{1,96})$/i;
const unsafe = /[\u0000-\u001f\u007f\u202a-\u202e\u2066-\u2069]/;
const labels = { friend: '一龙账号', url: '网页链接', phone: '电话号码', sms: '短信', email: '电子邮件', geo: '地理位置', wifi: 'Wi-Fi 信息', contact: '联系人', text: '文本' };
export function parseScanPayload(value) {
  const raw = typeof value === 'string' ? value.trim() : '';
  if (!raw) throw new Error('二维码内容为空');
  if (raw.length > MAX_SCAN_LENGTH) throw new Error('二维码内容过长');
  let kind = 'text', target = '';
  const friend = /^yilong:\/\/friend\/v1\/([^/?#]+)$/i.exec(raw);
  if (accountId.test(raw)) { kind = 'friend'; target = raw; }
  else if (friend && accountId.test(friend[1])) { kind = 'friend'; target = friend[1]; }
  else if (/^WIFI:/i.test(raw)) kind = 'wifi';
  else if (/^BEGIN:VCARD(?:\r?\n)/i.test(raw) || /^MECARD:/i.test(raw)) kind = 'contact';
  else if (!unsafe.test(raw) && !raw.includes('\\')) {
    if (/^https?:\/\//i.test(raw) && !/\s/.test(raw)) {
      try { const url = new URL(raw); if (url.hostname && !url.username && !url.password) { kind = 'url'; target = raw; } } catch { /* Present untrusted content as text. */ }
    } else if (/^tel:\+?[\d ()-]{3,40}$/i.test(raw)) { kind = 'phone'; target = `tel:${raw.slice(4).replace(/[ ()-]/g, '')}`; }
    else if (/^(?:sms|smsto):\+?[\d ()-]{3,40}(?::[^\r\n]*)?$/i.test(raw)) {
      kind = 'sms'; const [number, ...body] = raw.replace(/^[^:]+:/, '').split(':');
      target = `sms:${number.replace(/[ ()-]/g, '')}${body.length ? '?body=' + encodeURIComponent(body.join(':')) : ''}`;
    } else if (/^mailto:[^\s@?]+@[^\s@?]+$/i.test(raw)) { kind = 'email'; target = raw; }
    else if (/^geo:-?\d+(?:\.\d+)?,-?\d+(?:\.\d+)?$/i.test(raw)) {
      const [lat, lng] = raw.slice(4).split(',').map(Number);
      if (Math.abs(lat) <= 90 && Math.abs(lng) <= 180) { kind = 'geo'; target = raw; }
    }
  }
  return { raw, kind, title: labels[kind], target };
}
export function friendQrPayload(id) {
  if (!accountId.test(id)) throw new Error('账号 ID 无效');
  return `yilong://friend/v1/${id}`;
}
