const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..', 'src', 'features')
const read = (...parts) => fs.readFileSync(path.join(root, ...parts), 'utf8')
const localTasks = read('local-tasks', 'LocalTasksPage.tsx')
const projects = read('plaza', 'ProjectPlazaView.tsx')
const ai = read('ai', 'AiChatPage.tsx')

const checks = [
  ['local task list is sliced before render', localTasks.includes('tasks.slice(0, visibleTaskCount)')],
  ['local task rows have stable test ids', localTasks.includes('data-testid="local-task-row"') && localTasks.includes('data-task-id={task.id}')],
  ['local task list has progressive control', localTasks.includes('data-testid="local-task-list-more"')],
  ['project list uses bounded cursor pages', projects.includes("limit: String(PAGE_SIZE), page_mode: 'cursor'") && projects.includes('next_cursor')],
  ['project rows have stable test ids', projects.includes('data-testid="project-row"') && projects.includes('data-project-id={project.id}')],
  ['project list has progressive control', projects.includes('data-testid="project-list-more"')],
  ['conversation list starts with five rows per group', ai.includes('group.conversations.slice(0, 5)')],
  ['conversation rows have stable test ids', ai.includes('data-testid="ai-conversation-row"') && ai.includes('data-conversation-id={conversation.id}')],
  ['conversation expansion is addressable', ai.includes('data-testid="ai-conversation-list-more"')],
]

const failed = checks.filter(([, passed]) => !passed)
if (failed.length) {
  for (const [name] of failed) console.error(`FAIL: ${name}`)
  process.exit(1)
}
console.log(`progressive list source checks passed: ${checks.length}`)
