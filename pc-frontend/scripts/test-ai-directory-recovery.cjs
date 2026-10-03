const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const vm = require('node:vm')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/ai/AiChatPage.tsx')
const source = fs.readFileSync(filename, 'utf8')
const file = ts.createSourceFile(filename, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX)
const page = file.statements.find((node) => ts.isFunctionDeclaration(node) && node.name?.text === 'AiChatPage')
assert.ok(page?.body, 'AI page is available for isolated directory lifecycle verification')
const statements = page.body.statements
const declaration = (name) => statements.find((node) => ts.isVariableStatement(node)
  && node.declarationList.declarations.some((item) => item.name.getText(file).includes(name)))
const recovery = declaration('recoveryEpoch')
const userPanel = declaration('userPanelCollapsed')
const directoryEffect = statements.find((node) => ts.isExpressionStatement(node)
  && ts.isCallExpression(node.expression) && node.expression.expression.getText(file) === 'useEffect'
  && node.expression.arguments[0].getText(file).includes('loadConversations()'))
const loadConversations = statements.find((node) => ts.isFunctionDeclaration(node) && node.name?.text === 'loadConversations')
assert.ok(recovery && userPanel && directoryEffect && loadConversations)

// Execute the production effect and loader with their real dependency array.
// Keeping the test at this boundary avoids mounting unrelated provider sessions or starting work.
const extracted = [
  'function renderDirectoryLifecycle() {',
  recovery.getText(file), directoryEffect.getText(file), loadConversations.getText(file),
  '}',
  'function initialUserPanelCollapsed() {', userPanel.getText(file), 'return userPanelCollapsed', '}',
].join('\n')
const compiled = ts.transpileModule(extracted, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText

async function main() {
  const connection = { recoveryEpoch: 0 }
  const user = { id: 'consumer-1', nickname: '消费者' }
  let offline = true
  let dependencies
  let cleanup
  const requests = []
  const state = {
    input: '尚未发送的草稿', messages: [{ id: 'reply-1', content: '已有回答' }],
    activeConvId: 'current-conversation', sending: true, userPanelCollapsed: false,
    conversations: [], friends: [], totalUserCount: 0, usersError: '',
  }
  const protectedFields = ['input', 'messages', 'activeConvId', 'sending', 'userPanelCollapsed']
  const expectedProtected = Object.fromEntries(protectedFields.map((key) => [key, state[key]]))
  const bindings = {
    user, console,
    window: { localStorage: { getItem: () => null } },
    useState: (initial) => [typeof initial === 'function' ? initial() : initial, () => {}],
    useWorkbenchConnection: (selector) => selector(connection),
    useEffect(effect, nextDependencies) {
      const unchanged = dependencies && dependencies.length === nextDependencies.length
        && dependencies.every((value, index) => Object.is(value, nextDependencies[index]))
      if (unchanged) return
      cleanup?.()
      dependencies = nextDependencies
      cleanup = effect()
    },
    displayProjectName: (name) => name || '一龙 AI',
    api: {
      async get(url) {
        requests.push(url)
        if (offline) throw new Error('网络暂时不可用')
        if (url === '/api/me/ai/conversations?limit=50') {
          return { conversations: [{ id: 'history-1', title: '已有会话' }], project_id: 'project-1', project_name: '个人工作区' }
        }
        assert.equal(url, '/api/me/friends/recommendations?limit=50')
        return { recommendations: [{ id: 'user-2', nickname: '协作者' }], total_count: 12 }
      },
    },
  }
  for (const key of [...protectedFields, 'conversations', 'conversationPreviews', 'historyProjectName',
    'conversationsLoaded', 'friends', 'totalUserCount', 'usersLoading', 'usersError']) {
    bindings[`set${key[0].toUpperCase()}${key.slice(1)}`] = (value) => {
      state[key] = typeof value === 'function' ? value(state[key]) : value
    }
  }
  const context = vm.createContext(bindings)
  vm.runInContext(compiled, context, { filename })
  const settle = () => new Promise((resolve) => setImmediate(resolve))
  const assertProtected = () => {
    for (const key of protectedFields) assert.equal(state[key], expectedProtected[key], `${key} must survive directory recovery`)
  }

  assert.equal(context.initialUserPanelCollapsed(), false, 'all-users panel remains expanded by default')
  context.renderDirectoryLifecycle()
  await settle()
  assert.equal(requests.length, 2, 'signed-in offline start attempts both directories')
  assert.match(state.usersError, /网络/)
  assertProtected()

  context.renderDirectoryLifecycle()
  await settle()
  assert.equal(requests.length, 2, 'an unchanged recovery epoch does not refetch')

  offline = false
  connection.recoveryEpoch = 1
  context.renderDirectoryLifecycle()
  await settle()
  assert.equal(requests.length, 4, 'recovery retries both directories with the same signed-in user')
  assert.equal(state.conversations[0].id, 'history-1')
  assert.equal(state.conversations[0].project_name, '个人工作区')
  assert.equal(state.friends[0].id, 'user-2')
  assert.equal(state.totalUserCount, 12)
  assert.equal(state.usersError, '')
  assert.equal(state.usersLoading, false)
  assertProtected()

  context.renderDirectoryLifecycle()
  await settle()
  assert.equal(requests.length, 4, 'routine successful probes do not repeat directory loading')
  connection.recoveryEpoch = 2
  context.renderDirectoryLifecycle()
  await settle()
  assert.equal(requests.length, 6, 'a later real recovery can refresh again')
  assertProtected()
  console.log('PASS AI directory recovery: same account retries both lists; draft, messages, current conversation, active send, and expanded users survive')
}

main().catch((error) => { console.error(error); process.exitCode = 1 })
