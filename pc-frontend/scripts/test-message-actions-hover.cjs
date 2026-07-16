const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')

const projectRoot = path.resolve(__dirname, '..')
const read = (relativePath) => fs.readFileSync(path.join(projectRoot, relativePath), 'utf8')

const actionsSource = read('src/features/message-actions/MessageActions.tsx')
const actionsCss = read('src/features/message-actions/MessageActions.module.css')

assert.match(
  actionsSource,
  /export const messageActionsHostClassName = styles\.actionHost/,
  '共享操作栏必须导出消息悬停宿主类',
)
assert.match(
  actionsCss,
  /@media \(hover: hover\) and \(pointer: fine\)\s*\{[\s\S]*?\.actions\s*\{[\s\S]*?opacity:\s*0;[\s\S]*?pointer-events:\s*none;/,
  '只有支持精细悬停的设备才应默认隐藏操作栏',
)
assert.match(
  actionsCss,
  /\.actionHost:hover \.actions,\s*\.actionHost:focus-within \.actions\s*\{[\s\S]*?opacity:\s*\.64;[\s\S]*?pointer-events:\s*auto;/,
  '整条消息悬停或键盘聚焦时必须恢复操作栏',
)

for (const consumer of [
  'src/features/conversation/ConversationMessage.tsx',
  'src/features/ai/AiChatMessageRow.tsx',
  'src/features/friends/FriendsPage.tsx',
]) {
  const source = read(consumer)
  assert.match(source, /messageActionsHostClassName/, `${consumer} 必须接入共享悬停宿主类`)
  assert.match(source, /className=\{\[[^\]]*messageActionsHostClassName/, `${consumer} 必须把悬停宿主类放在消息行上`)
}

console.log('message actions hover: all assertions passed')
