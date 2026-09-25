import { test } from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, rm, writeFile, readFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { prepareRegistration } from './registration.mjs'

test('registration persists complete immutable assets outside disposable worktrees', async t => {
  const root = await mkdtemp(path.join(tmpdir(), 'yilong-reader-registration-'))
  t.after(async () => { assert.equal(path.dirname(root), path.resolve(tmpdir())); await rm(root, { recursive: true }) })
  const id = '00000000-0000-4000-8000-000000000001'
  const input = { projectRoot: process.cwd(), references: [`chatgpt-conversation://${id}`], storageRoot: root }
  const first = await prepareRegistration(input), second = await prepareRegistration(input)
  assert.equal(first.entrypoint, second.entrypoint)
  assert.equal(first.ids, id)
  assert.equal(path.dirname(path.dirname(first.entrypoint)), root)
  assert.match(await readFile(path.join(path.dirname(first.entrypoint), 'assets.mjs'), 'utf8'), /export function assetContent/)
  await writeFile(path.join(path.dirname(first.entrypoint), 'win-runtime.mjs'), 'changed')
  await assert.rejects(prepareRegistration(input), /reader_asset_mismatch/)
  await assert.rejects(prepareRegistration({ ...input, references: ['https://example.com/private'] }), /invalid_conversation_reference/)
})
