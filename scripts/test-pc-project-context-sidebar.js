const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const sidebar = read('pc-frontend/src/features/conversation/ProjectContextSidebar.tsx')
const sidebarCss = read('pc-frontend/src/features/conversation/ProjectContextSidebar.module.css')
const conversation = read('pc-frontend/src/features/conversation/ConversationPage.tsx')
const topbar = read('pc-frontend/src/features/conversation/ConversationTopbarActions.tsx')
const accountMenu = read('pc-frontend/src/features/shell/UserAccountMenu.tsx')
const detailPage = read('pc-frontend/src/features/projects/ProjectDetailPage.tsx')

for (const contract of ['role="tablist"', "setTab('project')", "setTab('members')", '开发位置', '工作目录', '完整项目设置', '邀请成员']) {
  assert.ok(sidebar.includes(contract), `project context sidebar should include ${contract}`)
}

for (const removedButton of ['>状态</button>', '>选择</button>', '>目录</button>', '>成员页</button>', '>管理</button>', '>日志</button>']) {
  assert.ok(!sidebar.includes(removedButton), `project context sidebar should remove the old ${removedButton} button`)
}

assert.match(sidebar, /icon_data_url[\s\S]*updateLogo/, 'project logo should be editable from the project tab')
assert.match(sidebar, /copyText\(workspacePath, '工作目录已复制'\)/, 'workspace path should be directly copyable')
assert.match(sidebar, /canInviteMembers[\s\S]*onShowInvites/, 'invitation should be the member tab primary action')
assert.match(sidebarCss, /grid-template-rows:\s*58px 42px minmax\(0, 1fr\)/, 'sidebar should keep a stable header, tabs, and scrollable content')
assert.match(sidebarCss, /@media \(max-width: 1440px\)[\s\S]*position:\s*absolute/, 'compact desktop should use an overlay sidebar')

assert.ok(!topbar.includes('分享算力'), 'project topbar should not contain the user-level compute action')
assert.ok(!topbar.includes('移动端'), 'project topbar should not contain the global mobile action')
assert.ok(!topbar.includes('旧版'), 'project topbar should not contain the global legacy action')
for (const accountAction of ['我的状态', '电脑与算力', '移动端', '切换旧版']) {
  assert.ok(accountMenu.includes(accountAction), `account menu should own ${accountAction}`)
}

assert.ok(!conversation.includes('ConversationMemberSidebar'), 'the old member-button-wall component should be removed')
assert.ok(!conversation.includes('onShowPresence'), 'personal presence should not be wired through the project sidebar')
assert.match(detailPage, /tabFromLocation\(location\.pathname, location\.search\)/, 'full project settings should support sidebar deep links')

console.log('PC project context sidebar contracts passed')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}
