const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const projectsPage = read('pc-frontend/src/features/projects/ProjectsPage.tsx')
const projectsCss = read('pc-frontend/src/features/projects/ProjectsPage.module.css')
const landing = read('pc-frontend/src/features/conversation/ProjectLanding.tsx')
const landingDownloads = read('pc-frontend/src/features/conversation/ProjectLandingDownloads.tsx')
const landingCss = read('pc-frontend/src/features/conversation/ProjectLanding.module.css')

assert.match(
  projectsPage,
  /async function openProject[\s\S]*selectProject\(projectId\)[\s\S]*navigate\('\/workspace'\)/,
  'project selection should enter the workspace landing directly',
)
assert.doesNotMatch(projectsPage, /function ProjectHome\(/, 'project center should not keep a duplicate project-home intermediary')
assert.match(projectsPage, /onManage=.*navigate\(`\/projects\/\$\{projectId\}`\)/, 'project management should remain a secondary action')
assert.match(projectsCss, /grid-template-columns:\s*var\(--sidebar-width\) minmax\(0, 1fr\)/, 'project center should use the reclaimed main width')

for (const contract of ['workflowGrid', 'landing?.highlights', 'landing?.target_users', 'quickChannels', 'projectResources']) {
  assert.ok(landing.includes(contract), `project landing should include ${contract}`)
}
assert.ok(landing.includes('ProjectLandingDownloads'), 'project landing should render the complete download surface')
assert.ok(landingDownloads.includes('variantList'), 'download variants should stay independently actionable')
assert.match(landingCss, /width:\s*min\(1120px, 100%\)/, 'project landing should restore the wider desktop composition')

console.log('PC project center and project-home contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
