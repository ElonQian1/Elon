const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const projectsPage = read('pc-frontend/src/features/projects/ProjectsPage.tsx')
const projectPlaza = read('pc-frontend/src/features/plaza/ProjectPlazaView.tsx')
const landing = read('pc-frontend/src/features/conversation/ProjectLanding.tsx')
const landingDownloads = read('pc-frontend/src/features/conversation/ProjectLandingDownloads.tsx')
const landingCss = read('pc-frontend/src/features/conversation/ProjectLanding.module.css')
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

for (const contract of ['workflowGrid', 'landing?.highlights', 'landing?.target_users', 'quickChannels', 'projectResources']) {
  assert.ok(landing.includes(contract), `project landing should include ${contract}`)
}
assert.ok(landing.includes('ProjectLandingDownloads'), 'project landing should render the complete download surface')
assert.ok(landingDownloads.includes('variantList'), 'download variants should stay independently actionable')
assert.match(landingCss, /width:\s*min\(1120px, 100%\)/, 'project landing should restore the wider desktop composition')

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
