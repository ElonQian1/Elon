import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { registerClaude } from './register-claude.mjs'

const id = '00000000-0000-4000-8000-000000000001'
async function fixture(t, config = {}) {
  const root = await mkdtemp(path.join(tmpdir(), 'yilong-claude-reader-'))
  t.after(async () => { assert.equal(path.dirname(root), path.resolve(tmpdir())); await rm(root, { recursive: true }) })
  const input = { configPath: path.join(root, 'claude_desktop_config.json'), projectRoot: process.cwd(),
    references: [id], storageRoot: path.join(root, 'assets') }
  await writeFile(input.configPath, JSON.stringify(config))
  return input
}
test('desktop registration preserves other tools/preferences, backs up, and uses stable complete assets', async t => {
  const original = { preferences: { theme: 'dark' }, mcpServers: { other: { command: 'existing', env: { SECRET: 'private' } } } }
  const input = await fixture(t, original), before = await readFile(input.configPath, 'utf8')
  const receipt = await registerClaude({ ...input, apkMcpUrl: 'http://127.0.0.1:8787' })
  const after = JSON.parse(await readFile(input.configPath, 'utf8'))
  assert.deepEqual(after.preferences, original.preferences); assert.deepEqual(after.mcpServers.other, original.mcpServers.other)
  assert.equal(await readFile(receipt.backup, 'utf8'), before)
  const own = after.mcpServers.yilong_web_conversations
  assert.ok(path.isAbsolute(own.command)); assert.equal(own.env.ELON_WEB_CONVERSATION_IDS, id)
  if (process.env.LOCALAPPDATA) assert.equal(own.env.LOCALAPPDATA, process.env.LOCALAPPDATA)
  assert.equal(own.env.ELON_APK_MCP_URL, 'http://127.0.0.1:8787')
  assert.match(await readFile(path.join(path.dirname(own.args[0]), 'assets.mjs'), 'utf8'), /assetContent/)
  assert.doesNotMatch(JSON.stringify(receipt), /SECRET|private|00000000/)
  assert.equal((await registerClaude(input)).status, 'unchanged')
})
test('explicit scope replaces prior grant while retaining only supported device settings', async t => {
  const input = await fixture(t, { mcpServers: { yilong_web_conversations: { env: {
    ELON_WEB_CONVERSATION_IDS: '00000000-0000-4000-8000-000000000002',
    ELON_WEB_CONVERSATION_WIN_INSTANCE: 'pinned', ELON_APK_MCP_URL: 'http://127.0.0.1:8787', UNKNOWN: 'drop',
  } } } })
  await registerClaude(input)
  const own = JSON.parse(await readFile(input.configPath, 'utf8')).mcpServers.yilong_web_conversations
  assert.equal(own.env.ELON_WEB_CONVERSATION_IDS, id); assert.equal(own.env.ELON_WEB_CONVERSATION_WIN_INSTANCE, 'pinned')
  assert.equal(own.env.UNKNOWN, undefined)
})
test('malformed config, invalid grants, remote endpoints and a competing writer do not overwrite settings', async t => {
  const input = await fixture(t)
  for (const value of ['not-json', '[]', '{"mcpServers":[]}']) {
    await writeFile(input.configPath, value)
    await assert.rejects(registerClaude(input), /claude_config_invalid/)
    assert.equal(await readFile(input.configPath, 'utf8'), value)
  }
  await writeFile(input.configPath, '{}')
  await assert.rejects(registerClaude({ ...input, references: ['all'] }), /invalid_conversation_reference/)
  await assert.rejects(registerClaude({ ...input, apkMcpUrl: 'http://example.com' }), /invalid_local_endpoint/)
  await writeFile(input.configPath + '.yilong.lock', '')
  await assert.rejects(registerClaude(input), /claude_config_locked/)
  assert.equal(await readFile(input.configPath, 'utf8'), '{}')
})
