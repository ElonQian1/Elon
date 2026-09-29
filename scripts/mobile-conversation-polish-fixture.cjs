// Read-only synthetic conversations around the production mobile page.
const { createFixture } = require('./mobile-design-pwa-fixture.cjs');
const fs = require('node:fs');
const path = require('node:path');
const avatar = 'data:image/svg+xml;base64,' + Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" width="80" height="120"><rect width="80" height="120" fill="#32689a"/><circle cx="40" cy="40" r="22" fill="#e7eef7"/><path d="M10 120V94a30 30 0 0160 0v26" fill="#e7eef7"/></svg>').toString('base64');
const members = [{ user_id: 'polish-member', display_name: '林悦', avatar_data_url: avatar },
  { user_id: 'mobile-v2-fixture', display_name: '演示用户' }];
const group = { id: 'polish-group', name: '产品协作群', member_count: 2, members, unread_count: 2,
  last_message: '新版页面已整理好，先检查输入和阅读体验。', last_message_at: '2026-09-29T03:22:00Z' };
const messages = [{ id: 'polish-1', sender_user_id: 'polish-member', sender_name: '林悦', sender_avatar_data_url: avatar,
  content: '新版页面已整理好，先检查输入和阅读体验。', created_at: '2026-09-29T03:20:00Z' },
{ id: 'polish-2', sender_user_id: 'mobile-v2-fixture', outgoing: true, content: '好的，重点看手机上的文字、附件和输入区域。', created_at: '2026-09-29T03:21:00Z' },
{ id: 'polish-3', sender_user_id: 'polish-member', sender_name: '林悦', sender_avatar_data_url: avatar,
  content: '这是离线布局示例，不连接真实群聊。', created_at: '2026-09-29T03:22:00Z' }];
function createConversationFixture() {
  const fixture = createFixture();
  const [base] = fixture.server.listeners('request');
  fixture.server.removeListener('request', base);
  fixture.server.on('request', (req, res) => {
    const origin = 'http://127.0.0.1:' + fixture.server.address().port;
    const url = new URL(req.url, origin);
    if (req.headers.host !== new URL(origin).host || (req.headers.origin && req.headers.origin !== origin)) return base(req, res);
    const json = body => { res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' }); res.end(JSON.stringify(body)); };
    if (req.method === 'GET' && req.headers.authorization?.startsWith('Bearer fixture-')) {
      if (new URL(req.headers.referer || '/', origin).searchParams.get('fixture') === 'empty') return base(req, res);
      if (url.pathname === '/api/me/groups') return json({ groups: [group] });
      if (url.pathname === '/api/me/friends') return json({ friends: [{ id: 'polish-friend', nickname: '林悦', account: 'fixture-friend', avatar_data_url: avatar, last_message: '可以开始评审了。' }] });
      if (url.pathname.startsWith('/api/me/groups/polish-group/')) return json({ messages, members, posts: [], items: [], ai_members: [] });
    }
    if (req.method === 'GET' && url.pathname === '/fixture-navigation.js') {
      const navigation = new URL(req.headers.referer || '/', origin).searchParams.get('home') === '1'
        ? '' : fs.readFileSync(path.join(__dirname, 'mobile-design-pwa-navigation.js'), 'utf8');
      res.writeHead(200, { 'content-type': 'text/javascript' });
      res.end(navigation + `
        if(new URLSearchParams(location.search).get('conversation')==='group'){
          let attempts=0;
          const open=setInterval(()=>{
            const row=[...document.querySelectorAll('.conversation-item')].find(el=>el.textContent.includes('产品协作群'));
            if(row){
              clearInterval(open);row.click();
              if(new URLSearchParams(location.search).get('draft')==='1'){
                let waits=0;
                const draft=setInterval(()=>{
                  const input=document.querySelector('#messageInput'), placeholder=document.querySelector('#inputPlaceholder');
                  if(placeholder?.getBoundingClientRect().height>0){
                    clearInterval(draft);placeholder.click();input.value='尚未发送的群聊草稿\\n检查输入区展开后的布局';
                    input.dispatchEvent(new Event('input',{bubbles:true}));
                  }else if(++waits>100)clearInterval(draft);
                },100);
              }
            }else if(++attempts>100)clearInterval(open);
          },100);
        }`);
      return;
    }
    return base(req, res);
  });
  return fixture;
}
module.exports = { createConversationFixture };
if (require.main === module) {
  const fixture = createConversationFixture();
  fixture.listen(Number(process.argv[2] || 43782)).then(origin => console.log(JSON.stringify({ origin, synthetic: true, writes: false })));
  for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => fixture.close().then(() => process.exit(0)));
}
