// Verify production rendering keeps legacy chat URLs playable on the HTTPS PWA origin.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const playwright = require(process.env.PLAYWRIGHT_MODULE || 'playwright');

(async () => {
  for (const engine of ['chromium', 'webkit']) {
    const browser = await playwright[engine].launch({ headless: true, ...(engine === 'chromium' ? { channel: 'msedge' } : {}) });
    try {
      const page = await browser.newPage();
      await page.route('https://pwa.example.test/**', route => route.fulfill({ contentType: 'text/html', body: '<main id="chat"></main>' }));
      await page.goto('https://pwa.example.test/web');
      await page.addScriptTag({ content: fs.readFileSync(path.join(__dirname, '../server/src/assets/social_chat_view.js'), 'utf8') });
      const result = await page.evaluate(() => {
        window.ElonArticles = { mount() {} };
        const list = document.querySelector('#chat');
        const view = ElonSocialChatView.create({ list, resetTimeline() {}, time() { return Date.now(); }, user() { return { id: 'synthetic' }; },
          friendName() { return 'test'; }, append() {
            const block = document.createElement('div'); block.className = 'chat-message-block';
            const bubble = document.createElement('div'); block.append(bubble); list.append(block); return bubble;
          } });
        const urls = ['http://pwa.example.test:8080/api/user/test/chat-attachments/group/voice.m4a?download=1',
          'https://other.example.test/api/user/test/chat-attachments/group/file.txt', 'http://pwa.example.test:8080/article/1', 'javascript:alert(1)'];
        view.render([{ id: 'local', content: '', outgoing: true, attachments: urls.map(url => ({ url, kind: 'file', display_name: 'test' })) }], 'friend', { id: 'friend' }, false);
        return Array.from(list.querySelectorAll('a')).map(link => link.href);
      });
      assert.deepEqual(result, ['https://pwa.example.test/api/user/test/chat-attachments/group/voice.m4a?download=1',
        'https://other.example.test/api/user/test/chat-attachments/group/file.txt', 'http://pwa.example.test:8080/article/1']);
      console.log(JSON.stringify({ engine, status: 'passed', sameServerMedia: 'current-https-origin', foreignUrls: 'preserved', unsafeScheme: 'rejected' }));
    } finally { await browser.close(); }
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
