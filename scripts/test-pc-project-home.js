const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const projectsPage = read('pc-frontend/src/features/projects/ProjectsPage.tsx')
const projectPlaza = read('pc-frontend/src/features/plaza/ProjectPlazaView.tsx')
const marketplaceInstall = read('pc-frontend/src/features/plaza/MarketplaceErpInstallDialog.tsx')
const landing = read('pc-frontend/src/features/conversation/ProjectLanding.tsx')
const landingDownloads = read('pc-frontend/src/features/conversation/ProjectLandingDownloads.tsx')
const landingCss = read('pc-frontend/src/features/conversation/ProjectLanding.module.css')
const quantLaunch = read('pc-frontend/src/features/conversation/QuantPaperLaunch.tsx')
const quantLaunchCss = read('pc-frontend/src/features/conversation/QuantPaperLaunch.module.css')
const landingTypes = read('pc-frontend/src/features/conversation/types.ts')
const launchSchema = JSON.parse(read('contracts/quant/paper-launch-v1.schema.json'))
const officialCatalog = JSON.parse(read('server/src/official_project_catalog/catalog.json'))
const conversationPage = read('pc-frontend/src/features/conversation/ConversationPage.tsx')
const conversationDraft = read('pc-frontend/src/features/conversation/NewConversationDraft.tsx')
const conversationDraftCss = read('pc-frontend/src/features/conversation/NewConversationDraft.module.css')

assert.match(
  projectPlaza,
  /async function openProject\(project: PlazaProject\)[\s\S]*selectProject\(project\.id\)[\s\S]*navigate\('\/workspace'\)/,
  'project selection should enter the workspace landing directly',
)
assert.doesNotMatch(projectsPage, /function ProjectHome\(/, 'project center should not keep a duplicate project-home intermediary')
assert.match(projectsPage, /<ProjectPlazaView \/>/, 'project center should delegate to the current project plaza')
assert.match(projectPlaza, /project\.install_action\?\.kind === 'erp_blueprint'/, 'plaza should expose published ERP blueprints')
assert.match(projectPlaza, /<MarketplaceErpInstallDialog/, 'plaza should use the dedicated ERP onboarding dialog')
assert.match(marketplaceInstall, /\/api\/store\/projects\/\$\{encodeURIComponent\(project\.id\)\}\/erp-instances/, 'ERP onboarding should call the marketplace instance endpoint')
assert.match(marketplaceInstall, /平台账号登录和商户经营数据不会写入公开模板项目/, 'ERP onboarding should explain tenant data isolation')

for (const contract of ['workflowGrid', 'landing?.highlights', 'landing?.target_users', 'quickChannels', 'projectResources']) {
  assert.ok(landing.includes(contract), `project landing should include ${contract}`)
}
assert.ok(landing.includes('ProjectLandingDownloads'), 'project landing should render the complete download surface')
assert.ok(landingDownloads.includes('variantList'), 'download variants should stay independently actionable')
assert.match(landingCss, /width:\s*min\(1120px, 100%\)/, 'project landing should restore the wider desktop composition')

assert.match(landing, /project\.id === 'yilong-quant'[\s\S]*yilong\.quant\.paper_launch\.v1/, 'Paper launch must be limited to the official quant project')
assert.ok(landing.includes('<QuantPaperLaunch integration={quantPaperLaunch} />'), 'quant project home should render its controlled launch surface')
assert.ok(landingTypes.includes('ProjectLandingPaperLaunch'), 'landing types should expose the sanitized paper launch contract')
for (const route of ['/api/me/quant/paper-launch', '/api/me/quant/paper-launches']) {
  assert.ok(quantLaunch.includes(route), `quant launch should call ${route}`)
}
assert.match(quantLaunch, /sandbox="allow-scripts allow-same-origin"/, 'quant iframe must keep a minimal sandbox')
assert.match(quantLaunch, /referrerPolicy="strict-origin"/, 'quant iframe must not leak main page paths')
assert.match(quantLaunch, /event\.source !== frameWindow \|\| event\.origin !== expectedOrigin/, 'message receipt must bind exact source and origin')
assert.match(quantLaunch, /frameWindow\.postMessage\([\s\S]*expectedOrigin\)/, 'grant delivery must use the exact quant origin')
for (const forbidden of ['localStorage', 'sessionStorage', 'clipboard', "postMessage('*'", '?grant=']) {
  assert.equal(quantLaunch.includes(forbidden), false, `quant launch must not contain ${forbidden}`)
}
assert.match(quantLaunchCss, /height:\s*min\(720px, 72vh\)/, 'embedded quant surface should remain usable on desktop')
assert.equal(launchSchema.$defs.protocol.const, 'yilong.quant.paper_launch.v1')
const quantCatalog = officialCatalog.projects.find((project) => project.id === 'yilong-quant')
assert.equal(quantCatalog.landing.paper_launch.schema, 'yilong.quant.paper_launch.v1')

assert.match(conversationPage, /onSelectChannel=\{\(id\) => \{ void openDevelopmentDraft\(id\) \}\}/, 'continue development should open a draft conversation')
assert.ok(conversationPage.includes("channels.find((channel) => channel.id === id)?.kind === 'ai_development') openDevelopmentDraft(id)"), 'clicking the AI development channel should open the same draft conversation')
assert.match(conversationPage, /sessionView === 'new'[\s\S]*<NewConversationDraft/, 'new sessions should use the dedicated draft canvas')
assert.match(conversationPage, /sessionView !== 'new' && activeProject && <ProjectContextSidebar/, 'draft mode should hide the project context sidebar')
assert.match(conversationPage, /function startNewSession\(\)[\s\S]*waitingForNewSession\.current = false/, 'opening an empty draft must not wait for or create a real conversation')
for (const label of ['开发新功能', '修复一个问题', '继续未完成任务', '分析当前项目', '发送第一条消息后才会创建真实会话']) {
  assert.ok(conversationDraft.includes(label), `new conversation draft should include ${label}`)
}
assert.match(conversationDraftCss, /grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/, 'draft starters should use a clean two-column desktop layout')

console.log('PC project center and project-home contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
