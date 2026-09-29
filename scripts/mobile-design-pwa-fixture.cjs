// Local-only acceptance fixture. Serves production assets; never forwards a request.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const crypto = require('node:crypto');

const ROOT = path.resolve(__dirname, '..');
const ASSETS = path.join(ROOT, 'server/src/assets');
const SCENARIOS = new Set(['login', 'register', 'projects', 'empty', 'chat_result', 'account_security']);
const USER = { id: 'mobile-v2-fixture', account: '演示账户', nickname: '离线演示账户' };
const PROJECTS = [
  { id: 'preview-work', name: '移动协作工具', description: '整理需求、跟踪任务与查看交付结果', member_count: 3 },
  { id: 'preview-long', name: '一个较长的项目名称，用于检查换行和大字体', description: '长说明应保持可读，项目操作与内容层级清楚。', member_count: 128 },
  { id: 'preview-joint', name: '联合项目示例', description: '共同查看项目进度', member_count: 8, is_joint: true }
].map(project => ({ ...project, title: project.name, owner_id: USER.id, owner_account: USER.account,
  role: 'owner', status: 'idle', created_at: '2026-09-28T00:00:00Z', updated_at: '2026-09-28T00:00:00Z' }));
const RESULT = '已整理需求。\n\n构建结果：成功\n验证结果：请查看详细记录。\n\n这是离线布局示例，不代表实际任务已完成。';
const EMPTY_READS = new Set(['/api/me/friends', '/api/me/groups', '/api/me/assets/esk',
  '/api/me/assets/esk/exchange-account', '/api/me/assets/esk/exchanges', '/api/me/progression',
  '/api/memories', '/api/projects/preview-work/space', '/api/projects/preview-work/workspace/health',
  '/api/store/joined', '/api/store/projects', '/api/user/mobile-v2-fixture/agent',
  '/api/user/mobile-v2-fixture/usage/stats']);

function productionIncludes() {
  const declarations = fs.readFileSync(path.join(ROOT, 'server/src/web.rs'), 'utf8');
  const includes = new Map();
  for (const match of declarations.matchAll(/const\s+(\w+):[^=]+?=\s*include_(str|bytes)!\(\s*"([^"]+)"\s*\);/g)) {
    includes.set(match[1], { kind: match[2], file: path.resolve(ROOT, 'server/src', match[3]) });
  }
  return includes;
}

function productionAsset(includes, name) {
  const source = includes.get(name);
  if (!source || path.relative(ROOT, source.file).startsWith('..')) throw Error('Unknown production asset: ' + name);
  return { kind: source.kind, content: fs.readFileSync(source.file) };
}

function productionTemplate(includes) {
  return fs.readFileSync(path.join(ASSETS, 'web_page.html'), 'utf8')
    .replace(/__(\w+_PNG_B64)__/g, (_, name) => {
      const source = productionAsset(includes, includes.has(name) ? name : name.replace(/_B64$/, ''));
      return source.kind === 'bytes' ? source.content.toString('base64') : source.content.toString('utf8').trim();
    })
    .replace(/__UI_TUNER_[A-Z0-9_]+__/g, '')
    .replace('</body>', '<script src="/fixture-navigation.js"></script></body>');
}

function createFixture({ handleSyntheticRequest } = {}) {
  const sessions = new Set(), sockets = new Set();
  const audit = { logins: 0, rejectedWrites: 0, outgoingMessagesRejected: 0 };
  const includes = productionIncludes(), html = productionTemplate(includes);
  // These production routes use Android resources rather than files in server/src/assets.
  const routedImages = new Map([
    ['/assets/ic_project_members_toolbar.png', 'PROJECT_MEMBERS_TOOLBAR_ICON_PNG'],
    ['/assets/ic_side_menu_folder_closed.png', 'SIDE_MENU_FOLDER_CLOSED_ICON_PNG']
  ].map(([route, name]) => [route, productionAsset(includes, name).content]));
  const scenario = req => {
    const ref = new URL(req.headers.referer || '/', 'http://127.0.0.1');
    return ref.searchParams.get('fixture') || 'projects';
  };
  const authenticated = req => sessions.has(String(req.headers.authorization || '').replace(/^Bearer /, ''));
  const server = http.createServer(async (req, res) => {
    const origin = 'http://127.0.0.1:' + server.address().port;
    if (req.headers.host !== new URL(origin).host || (req.headers.origin && req.headers.origin !== origin)) {
      res.writeHead(403); res.end(); return;
    }
    const url = new URL(req.url, origin), p = url.pathname;
    const json = (value, status = 200) => {
      res.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
      res.end(JSON.stringify(value));
    };
    if (req.method === 'POST' && p === '/api/auth/login') {
      let body = '';
      try {
        for await (const chunk of req) {
          body += chunk;
          if (Buffer.byteLength(body) > 4096) { json({ error: 'fixture login too large' }, 413); return; }
        }
        const credentials = JSON.parse(body);
        if (credentials.account !== 'mobile-v2-fixture' || credentials.password !== 'offline-fixture-only') {
          json({ error: '只接受公开的离线测试账号' }, 401); return;
        }
        const token = 'fixture-' + crypto.randomBytes(24).toString('hex');
        sessions.add(token); audit.logins++;
        json({ token, user: USER }); return;
      } catch { json({ error: 'invalid fixture login' }, 400); return; }
    }
    if (handleSyntheticRequest && await handleSyntheticRequest(req, res, url, authenticated(req))) return;
    if (req.method !== 'GET') { audit.rejectedWrites++; json({ error: 'fixture is read-only' }, 405); return; }
    if (p === '/fixture/info') { json({ schema: 'elon.mobile_design_fixture.v1', synthetic: true, productionNetwork: false }); return; }
    if (p === '/') {
      if (!SCENARIOS.has(url.searchParams.get('fixture') || 'login')) { json({ error: 'unknown fixture' }, 400); return; }
      res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store',
        'content-security-policy': "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob:; connect-src 'self' ws://127.0.0.1:" + server.address().port + "; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'self'" });
      res.end(html); return;
    }
    if (p === '/fixture-navigation.js') {
      res.writeHead(200, { 'content-type': 'text/javascript; charset=utf-8' });
      res.end(fs.readFileSync(path.join(__dirname, 'mobile-design-pwa-navigation.js'))); return;
    }
    if (routedImages.has(p)) {
      res.writeHead(200, { 'content-type': 'image/png' }); res.end(routedImages.get(p)); return;
    }
    if (/^\/assets\/[\w.-]+\.(js|css|png|svg)$/.test(p)) {
      const file = path.join(ASSETS, path.basename(p));
      if (fs.existsSync(file)) {
        res.writeHead(200, { 'content-type': ({ '.js': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml' })[path.extname(file)] });
        res.end(fs.readFileSync(file)); return;
      }
    }
    // A fixture must not persist its synthetic API responses through the production worker.
    if (p === '/sw.js') { res.writeHead(404); res.end(); return; }
    if (p === '/api/auth/federation/providers') { json({ providers: [{ id: 'google', configured: false }] }); return; }
    if (p.startsWith('/api/')) {
      if (!authenticated(req)) { json({ error: 'fixture login required' }, 401); return; }
      if (p === '/api/me') { json({ user: USER }); return; }
      if (p === '/api/me/archive') { json({ projects: scenario(req) === 'empty' ? [] : PROJECTS }); return; }
      if (p === '/api/auth/identities') { json({ identities: [] }); return; }
      if (p === '/api/auth/security') {
        json({ password: { enabled: true, changed_at: null }, recovery: { available_code_count: 4 },
          sessions: [{ id: 'fixture-session', device_name: 'Web 演示设备', current: true, active: true }] }); return;
      }
      if (EMPTY_READS.has(p)) {
        json({ friends: [], groups: [], projects: [], agents: [], posts: [], articles: [], nodes: [], members: [], items: [], conversations: [] }); return;
      }
      json({ error: 'API outside the declared offline fixture' }, 404); return;
    }
    res.writeHead(404); res.end();
  });
  server.on('upgrade', (req, socket) => {
    const origin = 'http://127.0.0.1:' + server.address().port;
    const url = new URL(req.url, origin);
    if (req.headers.host !== new URL(origin).host || req.headers.origin !== origin ||
        !sessions.has(url.searchParams.get('token')) ||
        !['/ws/projects/preview-work', '/ws/app'].includes(url.pathname) ||
        req.headers['sec-websocket-version'] !== '13' || !/^[A-Za-z0-9+/]{22}==$/.test(req.headers['sec-websocket-key'] || '')) {
      socket.end('HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\n'); return;
    }
    // Bounded one-way test stream, not a general-purpose WebSocket implementation.
    const accept = crypto.createHash('sha1').update(req.headers['sec-websocket-key'] + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
    socket.write('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ' + accept + '\r\n\r\n');
    sockets.add(socket);
    socket.on('error', () => socket.destroy());
    socket.on('close', () => sockets.delete(socket));
    socket.on('data', () => { audit.outgoingMessagesRejected++; socket.destroy(); });
    if (url.pathname === '/ws/projects/preview-work') {
      const payload = Buffer.from(JSON.stringify({ type: 'done', message: RESULT }));
      const header = Buffer.alloc(4); header[0] = 0x81; header[1] = 126; header.writeUInt16BE(payload.length, 2);
      socket.write(Buffer.concat([header, payload]));
    }
  });
  return { server, audit, async listen(port = 0) {
    await new Promise((resolve, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', resolve); });
    return 'http://127.0.0.1:' + server.address().port;
  }, async close() {
    for (const socket of sockets) socket.destroy();
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  } };
}

module.exports = { createFixture, SCENARIOS };
if (require.main === module) {
  const fixture = createFixture();
  const port = Number(process.argv[2] || 0);
  if (!Number.isInteger(port) || port < 0 || port > 65535) throw Error('invalid fixture port');
  fixture.listen(port).then(origin => console.log(JSON.stringify({ schema: 'elon.mobile_design_fixture.v1', origin,
    synthetic: true, productionNetwork: false, accountMutation: false })));
  for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => fixture.close().then(() => process.exit(0)));
}
