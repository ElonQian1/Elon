// Browser interaction tests use isolated fixtures and intercepted APIs; no real chat is modified.
const assert = require('node:assert/strict')
const fs = require('node:fs/promises')
const path = require('node:path')
const { pathToFileURL } = require('node:url')
const root = path.resolve(__dirname, '..')
const { chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH || 'playwright')
const fixturePath = path.join(root, 'pc-frontend/.ai-tmp/group-revisions-test.html')
const original = '明天八点开会\n请带材料 🐲'
const changed = '明天九点开会\n请带材料 🐲'
const message = { id: 'm', sender_user_id: 'author', outgoing: true, content: original, revision: 1, created_at: '2026-09-01T08:00:00Z' }

async function main() {
  const { createServer } = await import(pathToFileURL(path.join(root, 'pc-frontend/node_modules/vite/dist/node/index.js')).href)
  const server = await createServer({ root: path.join(root, 'pc-frontend'), configFile: path.join(root, 'pc-frontend/vite.config.ts'), server: { host: '127.0.0.1', port: 0 }, logLevel: 'error' })
  await fs.mkdir(path.dirname(fixturePath), { recursive: true })
  await fs.writeFile(fixturePath, `<!doctype html><html><head><meta charset="utf-8"></head><body style="background:#07090d;color:#f8f7f4"><p>隔离交互测试数据</p><textarea aria-label="聊天草稿">保留普通聊天草稿</textarea><div id="app"></div><script type="module">
  import React, {useState} from 'react'; import {createRoot} from 'react-dom/client';
  import Actions from '/src/features/friends/GroupMessageRevisionActions.tsx';
  function Fixture(){const [message,setMessage]=useState(${JSON.stringify(message)}); window.replaceMessage=setMessage; return React.createElement(React.Fragment,null,React.createElement('p',{id:'current-text'},message.content),React.createElement(Actions,{groupId:'g',message,own:message.outgoing,onSaved:patch=>setMessage(value=>({...value,...patch}))}));}createRoot(document.getElementById('app')).render(React.createElement(Fixture));</script></body></html>`)
  let browser
  try {
    await server.listen()
    const address = server.httpServer.address()
    browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || 'msedge' })
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } })
    const errors = []
    page.on('pageerror', error => errors.push(error.message))
    let versions, failNext, conflictNext, patches
    const reset = () => { versions = [{ revision: 1, content: original, created_at: message.created_at, edited_by: 'author' }]; failNext = false; conflictNext = false; patches = [] }
    reset()
    await page.route('**/api/me/groups/**', async route => {
      const request = route.request(), url = new URL(request.url())
      if (request.method() === 'PATCH') {
        const data = request.postDataJSON(); patches.push(data)
        if (failNext) { failNext = false; return route.fulfill({ status: 503, json: { error: '测试网络失败' } }) }
        if (conflictNext) { conflictNext = false; versions.push({ revision: 2, content: '其他设备的修改', created_at: '2026-09-02T08:00:00Z', edited_by: 'author' }) }
        if (data.expected_revision !== versions.length) return route.fulfill({ status: 409, json: { error: '消息已在其他设备修改' } })
        if (versions[versions.length - 1].content !== data.content) versions.push({ revision: versions.length + 1, content: data.content, created_at: '2026-09-03T08:00:00Z', edited_by: 'author' })
        return route.fulfill({ json: { message: { id: 'm', group_id: 'g', content: data.content, revision: versions.length, edited_at: versions[versions.length - 1].created_at } } })
      }
      const before = Number(url.searchParams.get('before_revision') || 1000)
      // Smaller pages exercise pagination regardless of the client's preferred limit.
      const limit = Number(url.searchParams.get('limit')) === 1 ? 1 : 2
      const found = [...versions].reverse().filter(v => v.revision < before)
      return route.fulfill({ json: { message_id: 'm', current_revision: versions.length, revisions: found.slice(0, limit), next_before_revision: found.length > limit ? found[limit - 1].revision : null } })
    })

    for (const surface of ['PC', 'PWA']) {
      reset()
      await page.setViewportSize(surface === 'PWA' ? { width: 390, height: 844 } : { width: 1280, height: 800 })
      if (surface === 'PC') await page.goto(`http://127.0.0.1:${address.port}/pc/.ai-tmp/group-revisions-test.html`)
      else {
        await page.goto(`http://127.0.0.1:${address.port}/pc/.ai-tmp/group-revisions-test.html`)
        await page.locator('#app').evaluate(node => node.remove())
        await page.addStyleTag({ path: path.join(root, 'server/src/assets/orbital_mobile_theme.css') })
        await page.addStyleTag({ path: path.join(root, 'server/src/assets/group_message_revisions.css') })
        await page.addScriptTag({ path: path.join(root, 'server/src/assets/group_message_revisions.js') })
        await page.evaluate(seed => {
          const bubble = document.createElement('div'); bubble.id = 'pwa-bubble'; document.body.append(bubble)
          window.testMessage = seed
          window.drawRevision = () => {
            bubble.replaceChildren(); const text = document.createElement('p'); text.id = 'current-text'; text.textContent = window.testMessage.content; bubble.append(text)
            const api = (...args) => fetch(...args)
            const refresh = async () => { const data = await (await api('/api/me/groups/g/messages/m/revisions?limit=1')).json(); window.testMessage = { ...window.testMessage, ...data.revisions[0] }; window.drawRevision() }
            window.ElonGroupMessageRevisions.reconcile('g', [window.testMessage])
            window.ElonGroupMessageRevisions.mount(bubble, 'g', window.testMessage, api, refresh)
          }; window.drawRevision()
        }, message)
      }
      await page.getByRole('button', { name: '编辑', exact: true }).click()
      const geometry = await page.locator('dialog').evaluate(dialog => ({ width: dialog.getBoundingClientRect().width, viewport: innerWidth, scroll: dialog.scrollWidth, client: dialog.clientWidth, background: getComputedStyle(dialog).backgroundColor }))
      assert.ok(geometry.width <= geometry.viewport && geometry.scroll <= geometry.client, `${surface} dialog must fit the viewport`)
      assert.notEqual(geometry.background, 'rgb(255, 255, 255)', `${surface} dialog must retain the dark surface`)
      const editor = page.getByRole('textbox', { name: '消息文字' })
      await editor.fill(changed)
      failNext = true
      await page.getByRole('button', { name: '保存修改', exact: true }).click()
      await page.getByText(/测试网络失败/).waitFor()
      assert.equal(await editor.inputValue(), changed, `${surface} failed save must preserve draft`)
      await page.getByRole('button', { name: '保存修改', exact: true }).click()
      await page.getByRole('button', { name: '已编辑 · 1 次', exact: true }).waitFor()
      assert.equal(await page.getByRole('textbox', { name: '聊天草稿' }).inputValue(), '保留普通聊天草稿')
      assert.equal(await page.locator('#current-text').textContent(), changed)
      await page.getByRole('button', { name: '编辑', exact: true }).click()
      const finalText = '后天九点开会 <img src=x onerror=alert(1)>'
      await editor.fill(finalText)
      await page.getByRole('button', { name: '保存修改', exact: true }).click()
      await page.getByRole('button', { name: '已编辑 · 2 次', exact: true }).click()
      await page.getByRole('heading', { name: '第 3 版', exact: true }).waitFor()
      await page.getByRole('button', { name: '查看更早版本' }).click()
      await page.getByRole('heading', { name: '第 1 版 · 原始文字' }).waitFor()
      assert.equal(await page.locator('dialog pre').last().textContent(), original)
      assert.equal(await page.locator('dialog img').count(), 0, `${surface} history must render raw text safely`)
      await page.locator('dialog summary').first().click()
      assert.ok(await page.locator('dialog ins').first().textContent())
      await page.keyboard.press('Escape')
      await page.locator('dialog').waitFor({ state: 'detached' })

      // Reload the fixture with v1 so conflict checks don't rely on prior state.
      versions = [versions[0]]; conflictNext = true
      if (surface === 'PC') await page.evaluate(seed => window.replaceMessage(seed), message)
      else await page.evaluate(seed => { window.testMessage = seed; window.drawRevision() }, message)
      await page.getByRole('button', { name: '编辑', exact: true }).click()
      await editor.fill('冲突时保留我的草稿')
      await page.getByRole('button', { name: '保存修改', exact: true }).click()
      const acknowledge = page.getByRole('button', { name: '已核对，继续编辑我的草稿' })
      await acknowledge.waitFor()
      assert.equal(await editor.inputValue(), '冲突时保留我的草稿')
      assert.equal(await page.getByRole('button', { name: '保存修改', exact: true }).isDisabled(), true)
      await acknowledge.click()
      await page.getByRole('button', { name: '保存修改', exact: true }).click()
      await page.getByRole('button', { name: '已编辑 · 2 次', exact: true }).waitFor()
      assert.equal(patches[patches.length - 1].expected_revision, 2)
      // Read-only members can inspect history but get no edit action.
      const readOnly = { ...message, outgoing: false, revision: 3 }
      if (surface === 'PC') await page.evaluate(seed => window.replaceMessage(seed), readOnly)
      else await page.evaluate(seed => { window.testMessage = seed; window.drawRevision() }, readOnly)
      assert.equal(await page.getByRole('button', { name: '编辑', exact: true }).count(), 0)
      console.log(`PASS: ${surface} save/retry, draft preservation, complete paginated history, XSS-safe text, Escape, conflict acknowledgement, read-only member`)
    }
    assert.deepEqual(errors, [], 'browser runtime errors')
  } finally {
    if (browser) await browser.close()
    await server.close()
    await fs.unlink(fixturePath).catch(() => {})
  }
}
main().catch(error => { console.error(error); process.exitCode = 1 })
