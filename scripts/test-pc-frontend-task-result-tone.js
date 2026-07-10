const assert = require('assert')
const fs = require('fs')
const path = require('path')

const repoRoot = path.resolve(__dirname, '..')
const pcRoot = path.join(repoRoot, 'pc-frontend')

function loadTypescript() {
  const localTypescript = path.join(pcRoot, 'node_modules', 'typescript')
  if (fs.existsSync(localTypescript)) return require(localTypescript)
  return require('typescript')
}

const ts = loadTypescript()
const originalTsLoader = require.extensions['.ts']

require.extensions['.ts'] = function loadTsModule(module, filename) {
  const source = fs.readFileSync(filename, 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.CommonJS,
      target: ts.ScriptTarget.ES2020,
      esModuleInterop: true,
      jsx: ts.JsxEmit.ReactJSX,
    },
    fileName: filename,
  })
  module._compile(output.outputText, filename)
}

try {
  const {
    buildContext,
    taskResultDisplayText,
    taskResultTone,
  } = require(path.join(pcRoot, 'src', 'features', 'dev', 'devTaskUtils.ts'))

  assert.strictEqual(
    taskResultTone(
      'done',
      '已完成并发布。\n\n原来的语音转文字链路仍保留：只有直接录音启动失败时才回退到系统 SpeechRecognizer 转文字。',
    ),
    'done',
    'explicit done task status should not be downgraded by fallback/failure wording in a success summary',
  )
  assert.strictEqual(
    taskResultTone('', '任务失败：PC 节点断线。'),
    'failed',
    'explicit failure text should remain a failed fallback when no terminal status exists',
  )
  assert.strictEqual(
    taskResultTone('failed', '已完成并发布。'),
    'failed',
    'failed task status should remain authoritative over success-looking content',
  )
  assert.strictEqual(
    taskResultDisplayText({
      kind: 'ai_result',
      task_id: 'tsk-codex-limit',
      task_status: 'failed',
      content: '',
      task_error: '当前 Codex 账号额度已用尽或被限流，请切换可用账号后重试。',
    }),
    '当前 Codex 账号额度已用尽或被限流，请切换可用账号后重试。',
    'failed task result with empty content should display the classified Codex quota error',
  )
  assert.strictEqual(
    taskResultDisplayText({
      kind: 'ai_result',
      task_id: 'tsk-auth-invalid',
      task_status: 'failed',
      task_error: '当前 Codex 账号登录已失效，auth.json 无法刷新；账号本人需要重新登录。',
    }),
    '当前 Codex 账号登录已失效，auth.json 无法刷新；账号本人需要重新登录。',
    'failed shared auth switch should surface the auth.json refresh failure when no answer content exists',
  )
  const context = buildContext([
    {
      id: 'result-with-error-only',
      kind: 'ai_result',
      task_id: 'tsk-auth-invalid',
      task_status: 'failed',
      content: '',
      task_error: '当前 Codex 账号登录已失效，auth.json 无法刷新；账号本人需要重新登录。',
    },
  ])
  const task = context.tasks.get('tsk-auth-invalid')
  assert.ok(task, 'failed result should create a task context entry')
  assert.strictEqual(task.failed, true, 'empty-content auth failure should still be marked failed')
  assert.strictEqual(
    task.resultText,
    '当前 Codex 账号登录已失效，auth.json 无法刷新；账号本人需要重新登录。',
    'task cards should use task_error as the visible final result text',
  )
  assert.strictEqual(
    taskResultDisplayText({
      kind: 'ai_result',
      task_id: 'tsk-provider-error',
      task_status: 'failed',
      content: '任务遇到问题：平台 AI runtime 返回 502 Bad Gateway，本轮没有生成有效诊断。',
    }),
    '平台 AI 暂时不可用，本轮没有生成有效回复，结果未确认完成。请重试处理。',
    'provider details should stay in diagnostics instead of the primary failure reply',
  )
  assert.strictEqual(
    taskResultDisplayText({
      kind: 'ai_result',
      task_id: 'tsk-final-missing',
      task_status: 'failed',
      content: 'PC CLI 执行未完成：Codex 在最后一条公开说明之后仍执行了命令，但没有返回收尾回复；本轮结果无法确认完成。',
    }),
    '本机 AI 没有返回完整的最终回复，本轮未标记为完成。请重试处理。',
    'missing final replies should never be phrased as completed work',
  )
  console.log('pc-frontend task-result tone tests passed')
} finally {
  if (originalTsLoader) require.extensions['.ts'] = originalTsLoader
  else delete require.extensions['.ts']
}
