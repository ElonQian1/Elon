import fs from 'node:fs'
import path from 'node:path'
import assert from 'node:assert/strict'
import {createRequire} from 'node:module'
const require=createRequire(import.meta.url)
const {chromium}=require(process.env.PLAYWRIGHT_MODULE || 'playwright')
const root=path.resolve(import.meta.dirname, '..')
fs.mkdirSync(path.join(root,'.ai-tmp/project-introduction'),{recursive:true})
const html=fs.readFileSync(path.join(root,'server/src/assets/web_page.html'),'utf8')
const manifest=JSON.parse(fs.readFileSync(path.join(root,'.elon/project-landing.json'),'utf8'))
const browser=await chromium.launch({headless:true,channel:process.env.PLAYWRIGHT_CHANNEL || undefined})
try {
  const page=await browser.newPage({viewport:{width:360,height:900}})
  await page.goto('about:blank')
  await page.evaluate(html=>{
    const doc=new DOMParser().parseFromString(html,'text/html')
    const style=document.createElement('style')
    style.textContent=[...doc.querySelectorAll('style')].map(s=>s.textContent).join('\n')
    document.head.appendChild(style)
    const panel=doc.getElementById('projectProgressPanel')
    panel.classList.remove('hidden')
    panel.setAttribute('aria-hidden','false')
    document.body.appendChild(panel)
    document.getElementById('projectSpaceName').textContent='一龙 AI · 离线布局示例'
    document.getElementById('projectSpaceMemberMetric').textContent='成员 3'
  },html)
  await page.addStyleTag({content:fs.readFileSync(path.join(root,'server/src/assets/project_home.css'),'utf8')})
  await page.addScriptTag({content:fs.readFileSync(path.join(root,'server/src/assets/project_home.js'),'utf8')})
  await page.evaluate(manifest=>window.ElonProjectIntroduction.render({join_mode:'invite'}, {channels:[{kind:'ai_development'}],landing:{...manifest,downloads:Object.entries(manifest.downloads).map(([platform,value])=>({...value,platform}))}}),manifest)
  assert.equal(await page.locator('#projectIntroductionSections').count(),1)
  assert.equal(await page.getByText('4.8',{exact:false}).isVisible(),false)
  assert.equal(await page.getByText('12 岁以上 ⓘ',{exact:true}).isVisible(),false)
  for (const width of [320,360,411]) {
    await page.setViewportSize({width,height:900})
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth+1),false,`overflow at ${width}`)
    assert.ok((await page.locator('#projectSpaceDownloadBtn').boundingBox()).height>=48)
    await page.screenshot({path:path.join(root,'.ai-tmp/project-introduction',`pwa-overview-${width}.png`),fullPage:true})
  }
  console.log('PWA real markup checks passed: 320/360/411 widths, real intro/resources, no fabricated metrics, install touch target >=48px')
} finally {await browser.close()}
