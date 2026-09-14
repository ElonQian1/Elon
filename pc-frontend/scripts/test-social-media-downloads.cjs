const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const ts = require('typescript')
const React = require('react')
const { renderToStaticMarkup } = require('react-dom/server')

// Exercise the production TS/TSX components without adding a second test runtime.
for (const extension of ['.ts', '.tsx']) {
  require.extensions[extension] = (module, filename) => {
    const source = fs.readFileSync(filename, 'utf8')
    module._compile(ts.transpileModule(source, {
      fileName: filename,
      compilerOptions: { jsx: ts.JsxEmit.ReactJSX, module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, esModuleInterop: true },
    }).outputText, filename)
  }
}
require.extensions['.css'] = module => { module.exports = new Proxy({}, { get: (_, key) => key === '__esModule' ? false : key }) }
global.location = new URL('http://127.0.0.1:7799/pc/friends')
global.window = { __ELON_PC_BOOTSTRAP__: { mode: 'local', cloudBaseUrl: 'https://cloud.example' } }
const sourceRoot = path.resolve(__dirname, '../src')
const { cloudResourceUrl } = require(path.join(sourceRoot, 'lib/cloudResourceUrl.ts'))
const Attachments = require(path.join(sourceRoot, 'features/friends/SocialMessageAttachments.tsx')).default
const Avatar = require(path.join(sourceRoot, 'features/friends/SocialAvatar.tsx')).default
const DownloadLink = require(path.join(sourceRoot, 'features/project-download/ProjectDownloadLink.tsx')).default
const render = (component, props) => renderToStaticMarkup(React.createElement(component, props))

assert.equal(cloudResourceUrl('/app/ElonSpeed-latest.apk'), 'https://cloud.example/app/ElonSpeed-latest.apk')
assert.equal(cloudResourceUrl('/api/node-agent/download/windows-client'), 'https://cloud.example/api/node-agent/download/windows-client')
assert.equal(cloudResourceUrl('/pc'), 'https://cloud.example/pc')
assert.equal(cloudResourceUrl('https://files.example/image.png'), 'https://files.example/image.png')
for (const bad of ['', 'javascript:alert(1)', '//other.example/file', 'file:///C:/file', 'C:\\private.png', '/\\other.example/x', 'https://u:p@files.example/x']) {
  assert.equal(cloudResourceUrl(bad), '', bad)
}
window.__ELON_PC_BOOTSTRAP__ = { mode: 'cloud' }
global.location = new URL('https://workbench.example/pc/friends')
assert.equal(cloudResourceUrl('/app/latest.apk'), 'https://workbench.example/app/latest.apk')
window.__ELON_PC_BOOTSTRAP__ = { mode: 'local', cloudBaseUrl: 'https://cloud.example' }

const image = render(Attachments, { attachments: [{ kind: 'image', url: '/api/user/demo/chat-attachments/chat/photo.png', display_name: '测试图片' }] })
assert.match(image, /<img[^>]+src="https:\/\/cloud.example\/api\/user\/demo\/chat-attachments\/chat\/photo.png"/)
assert.match(image, /aria-label="查看图片：测试图片"/)
const audio = render(Attachments, { attachments: [{ kind: 'audio', url: '/voice.m4a', duration_seconds: 6, transcription: '测试转写' }] })
assert.match(audio, /<audio[^>]+controls=""[^>]+src="https:\/\/cloud.example\/voice.m4a"/)
assert.match(audio, /6 秒/)
assert.match(audio, /测试转写/)
assert.doesNotMatch(audio, /autoplay/i)
const mixed = render(Attachments, { attachments: [{ mime_type: 'image/jpeg', url: '/photo' }, { mime_type: 'audio/mp4', url: '/voice' }, { kind: 'file', display_name: '文档.pdf', url: '/document.pdf' }] })
assert.match(mixed, /<img/)
assert.match(mixed, /<audio/)
assert.match(mixed, /href="https:\/\/cloud.example\/document.pdf" download=""/)
assert.match(render(Attachments, { attachments: [{ display_name: '旧附件', url: null }] }), /附件地址不可用/)
assert.doesNotMatch(render(Attachments, { attachments: null }), /<(img|audio|a)\s/)
assert.match(render(Avatar, { userId: 'member-10', name: '群成员' }), /src="https:\/\/cloud.example\/api\/users\/member-10\/avatar"/)
assert.match(render(Avatar, { userId: 'me', name: '我', avatar: 'data:image/png;base64,YQ==' }), /src="data:image\/png;base64,YQ=="/)
assert.match(render(Avatar, { name: '访客' }), /访客的默认头像/)

const props = { enabled: true, memberProtected: false, platform: 'windows', className: 'card', onMemberDownload() {}, children: '下载' }
for (const url of ['/app/ElonSpeed-latest.apk', '/api/node-agent/download/windows-client', '/api/node-agent/download/linux']) {
  const html = render(DownloadLink, { ...props, url })
  assert.match(html, /<a /)
  assert.ok(html.includes(`href="https://cloud.example${url}"`))
  assert.match(html, /download=""/)
  assert.doesNotMatch(html, /target=/)
}
assert.doesNotMatch(render(DownloadLink, { ...props, platform: 'web', url: '/pc' }), /download=/)
assert.match(render(DownloadLink, { ...props, enabled: false, url: '/pending.apk' }), /<button[^>]+disabled=""/)
assert.match(render(DownloadLink, { ...props, memberProtected: true, url: '/api/store/projects/yilong-quant/downloads/android' }), /<button/)
assert.doesNotMatch(render(DownloadLink, { ...props, memberProtected: true, url: '/api/store/projects/yilong-quant/downloads/android' }), /<a /)
const friendsSource = fs.readFileSync(path.join(sourceRoot, 'features/friends/FriendsPage.tsx'), 'utf8')
assert.match(friendsSource, /!recalled && <SocialMessageAttachments attachments=\{m.attachments\}/, 'recalled attachments must not remain visible')
assert.match(friendsSource, /<SocialAvatar userId=\{m.sender_user_id\}/, 'all senders, including those outside the nine-member preview, need avatar lookup')
console.log('PASS: social image/audio/file rendering, avatar lookup, recall gating, cloud resource resolution and popup-free download links')
