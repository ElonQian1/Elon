import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright')
const root = path.resolve(import.meta.dirname, '..')
const output = path.join(root, '.ai-tmp', 'project-introduction')
fs.mkdirSync(output, { recursive: true })
const browser = await chromium.launch({ headless: true, channel: process.env.PLAYWRIGHT_CHANNEL || undefined })
const page = await browser.newPage()
const errors = []
page.on('pageerror', error => errors.push(error.message))
try {
  await page.goto(`${process.env.INTRO_PREVIEW_ORIGIN || 'http://127.0.0.1:5177'}/pc/project-introduction-preview.html`)
  await page.getByRole('heading', { name: '核心能力', exact: true }).waitFor()
  assert.equal(await page.locator('#project-capabilities > ul > li').count(), 6)
  assert.ok((await page.locator('body').innerText()).includes('和团队一起，把想法做成应用'))
  assert.equal((await page.locator('body').innerText()).includes('旧节点摘要'), false)
  await page.getByRole('button', { name: '查看团队成员' }).click()
  assert.match(await page.locator('output').innerText(), /members/)
  await page.getByRole('button', { name: '参与项目讨论' }).click()
  assert.match(await page.locator('output').innerText(), /preview-discussion/)
  await page.getByText('最近更新', { exact: true }).click()
  assert.equal(await page.locator('details').filter({ has: page.getByText('最近更新', { exact: true }) }).getAttribute('open'), '')
  for (const [mode, message] of [['invite', '接受项目邀请'], ['open', '项目允许开放加入'], ['approval', '申请通过后'], ['readonly', '只读体验']]) {
    await page.getByLabel('加入方式').selectOption(mode)
    assert.ok((await page.locator('#project-collaboration').innerText()).includes(message))
  }
  await page.getByLabel('加入方式').selectOption('invite')
  await page.getByLabel('成员身份').selectOption('visitor')
  assert.equal(await page.getByRole('button', { name: /继续开发/ }).count(), 0)
  await page.getByRole('button', { name: /了解加入方式/ }).click()
  assert.ok((await page.locator('#project-collaboration').innerText()).includes('你正在浏览项目'))
  await page.getByLabel('成员身份').selectOption('viewer')
  assert.equal(await page.getByRole('button', { name: /查看开发进度/ }).count(), 1)
  await page.getByLabel('成员身份').selectOption('member')
  for (const width of [1280, 768, 390, 320]) {
    await page.setViewportSize({ width, height: 900 })
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth > innerWidth + 1), false, `horizontal overflow at ${width}`)
    await page.screenshot({ path: path.join(output, `desktop-framework-${width}.png`), fullPage: true })
  }
  await page.getByLabel('预览项目').selectOption('child')
  await page.getByRole('heading', { name: '子项目介绍示例', exact: true }).waitFor()
  assert.ok(await page.locator('#project-capabilities > ul > li').count() > 4, 'child must retain the complete capability list')
  assert.ok((await page.locator('body').innerText()).includes('不接收或移动真实用户资金'))
  await page.getByLabel('预览项目').selectOption('empty')
  await page.getByRole('heading', { name: '尚未配置介绍的项目', exact: true }).waitFor()
  assert.ok((await page.locator('#project-capabilities').innerText()).includes('尚未提供能力清单'))
  assert.equal(await page.getByRole('button', { name: '进入 AI 开发', exact: true }).count(), 0)

  // Exercise the actual PWA renderer with hostile text and rerendering.
  await page.goto('about:blank')
  await page.setContent('<main><div id="projectSpaceSummary"><button id="projectSpaceDownloadBtn">安装</button></div><div id="projectSpacePreviewGrid"></div><button id="projectIntroCard">简介</button><div id="projectSpaceFeed"></div></main>')
  await page.addStyleTag({ content: fs.readFileSync(path.join(root, 'server/src/assets/project_home.css'), 'utf8') })
  await page.addScriptTag({ content: fs.readFileSync(path.join(root, 'server/src/assets/project_home.js'), 'utf8') })
  await page.evaluate(() => {
    window.ElonProjectIntroduction.render({ join_mode: 'invite' }, { channels: [{ kind: 'ai_development' }], landing: {
      tagline: '<img src=x onerror=window.__injected=true>', highlights: ['真实能力', '第二项'], recent_updates: ['布局示例'],
      resources: [{ label: '真实资料', url: 'https://example.com/docs' }, { label: '无效入口', url: 'javascript:alert(1)' }]
    } })
  })
  assert.equal(await page.locator('#projectIntroductionSections img').count(), 0)
  assert.ok((await page.locator('#projectIntroductionSections').innerText()).includes('<img src=x'))
  assert.equal(await page.locator('#projectIntroductionSections a').count(), 1)
  assert.equal(await page.locator('#projectIntroductionSections + #projectSpaceDownloadBtn').count(), 1)
  await page.getByText('最近更新', { exact: true }).click()
  assert.ok(await page.getByText('布局示例', { exact: true }).isVisible())
  await page.evaluate(() => window.ElonProjectIntroduction.render({ join_mode: 'readonly' }, { channels: [], landing: {} }))
  assert.equal(await page.locator('#projectIntroductionSections').count(), 1)
  assert.equal(await page.locator('#projectSpaceDownloadBtn').count(), 1)
  assert.equal(await page.locator('#projectSpaceFeedHeading').count(), 1)
  assert.equal(await page.locator('#projectIntroductionSections a').count(), 0)
  assert.ok((await page.locator('#projectIntroductionSections').innerText()).includes('只读体验'))
  await page.evaluate(() => window.ElonProjectIntroduction.clear())
  assert.equal(await page.locator('#projectIntroductionSections').count(), 0)
  assert.deepEqual(errors, [])
  console.log('Project introduction browser checks passed: full child content, 4 join modes, channel action, details, 4 viewport widths, empty fallback, PWA escaping/rerender/clear')
} finally {
  await browser.close()
}
