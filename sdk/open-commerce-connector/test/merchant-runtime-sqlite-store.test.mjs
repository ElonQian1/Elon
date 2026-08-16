import assert from 'node:assert/strict'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { DatabaseSync } from 'node:sqlite'
import test from 'node:test'

import {
  MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA,
  MerchantRuntimeSqliteStoreError,
  createSqliteMerchantRuntimeIdempotencyStore,
} from '../src/merchant-runtime-sqlite-store.js'
import {
  createTestRuntime,
  signedInvocation,
} from './merchant-runtime-fixtures.mjs'

test('replays one immutable runtime result after the SQLite store restarts', async (t) => {
  const directory = testDirectory()
  const databasePath = join(directory, 'runtime.sqlite')
  let calls = 0
  const firstStore = createStore(databasePath)
  let restartedStore
  t.after(() => {
    firstStore.close()
    restartedStore?.close()
    rmSync(directory, { recursive: true, force: true })
  })
  const firstRuntime = createTestRuntime({
    idempotencyStore: firstStore,
    handlers: {
      async 'catalog.search'() {
        calls += 1
        return { items: [{ id: 'coffee-1' }] }
      },
    },
  })
  const first = await firstRuntime.handleInvoke(signedInvocation())
  first.body.result.items[0].id = 'caller-mutated'
  firstStore.close()

  restartedStore = createStore(databasePath)
  const restartedRuntime = createTestRuntime({
    idempotencyStore: restartedStore,
    handlers: {
      async 'catalog.search'() {
        calls += 1
        return { items: [{ id: 'should-not-run' }] }
      },
    },
  })
  const replay = await restartedRuntime.handleInvoke(signedInvocation({
    invocation_id: 'invocation-sqlite-replay',
  }))

  assert.equal(first.status, 200)
  assert.equal(replay.status, 200)
  assert.equal(replay.body.result.items[0].id, 'coffee-1')
  assert.equal(calls, 1)
  assert.equal(restartedStore.schema, MERCHANT_RUNTIME_SQLITE_STORE_SCHEMA)
})

test('persists an idempotency conflict for a different logical input', async (t) => {
  const directory = testDirectory()
  const store = createStore(join(directory, 'runtime.sqlite'))
  t.after(() => {
    store.close()
    rmSync(directory, { recursive: true, force: true })
  })
  const runtime = createTestRuntime({ idempotencyStore: store })

  assert.equal((await runtime.handleInvoke(signedInvocation())).status, 200)
  const conflict = await runtime.handleInvoke(signedInvocation({
    invocation_id: 'invocation-sqlite-conflict',
    input: { query: 'espresso' },
  }))

  assert.equal(conflict.status, 409)
  assert.equal(conflict.body.error_code, 'idempotency_conflict')
})

test('allows timed-out takeover without giving the old invocation ownership', async (t) => {
  const directory = testDirectory()
  const databasePath = join(directory, 'runtime.sqlite')
  let timestamp = 10_000
  const firstStore = createStore(databasePath, { clock: () => timestamp })
  const secondStore = createStore(databasePath, { clock: () => timestamp })
  t.after(() => {
    firstStore.close()
    secondStore.close()
    rmSync(directory, { recursive: true, force: true })
  })
  const first = idempotencyInput()
  const second = idempotencyInput({ invocationId: 'invocation-sqlite-second' })

  assert.deepEqual(await firstStore.claim(first), { status: 'claimed' })
  assert.deepEqual(await secondStore.claim(second), { status: 'busy' })
  timestamp += 1_001
  assert.deepEqual(await secondStore.claim(second), { status: 'claimed' })
  assert.equal(await firstStore.complete(first, { owner: 'old' }), false)
  await firstStore.release(first)
  assert.equal(await secondStore.complete(second, { owner: 'new' }), true)
  assert.deepEqual(await firstStore.claim({
    ...first,
    invocationId: 'invocation-sqlite-third',
  }), {
    status: 'replayed',
    result: { owner: 'new' },
  })
})

test('releases a failed owner so another invocation can claim', async (t) => {
  const directory = testDirectory()
  const store = createStore(join(directory, 'runtime.sqlite'))
  t.after(() => {
    store.close()
    rmSync(directory, { recursive: true, force: true })
  })
  const first = idempotencyInput()
  const retry = idempotencyInput({ invocationId: 'invocation-sqlite-retry' })

  assert.deepEqual(await store.claim(first), { status: 'claimed' })
  await store.release(first)
  assert.deepEqual(await store.claim(retry), { status: 'claimed' })
})

test('fails closed when a persisted result is corrupted', async (t) => {
  const directory = testDirectory()
  const databasePath = join(directory, 'runtime.sqlite')
  const input = idempotencyInput()
  const firstStore = createStore(databasePath)
  let restartedStore
  t.after(() => {
    firstStore.close()
    restartedStore?.close()
    rmSync(directory, { recursive: true, force: true })
  })
  assert.deepEqual(await firstStore.claim(input), { status: 'claimed' })
  assert.equal(await firstStore.complete(input, { safe: true }), true)
  firstStore.close()

  const database = new DatabaseSync(databasePath)
  database.prepare(
    `UPDATE yilong_merchant_runtime_idempotency
        SET result_json = ?
      WHERE merchant_id = ?`,
  ).run('{"private":"do-not-leak"', input.merchantId)
  database.close()

  restartedStore = createStore(databasePath)
  await assert.rejects(
    restartedStore.claim({ ...input, invocationId: 'invocation-after-corruption' }),
    (error) => {
      assert.equal(error instanceof MerchantRuntimeSqliteStoreError, true)
      assert.equal(error.code, 'SQLITE_STORE_CORRUPTED')
      assert.equal(error.message.includes('do-not-leak'), false)
      return true
    },
  )
})

test('rejects an oversized persisted result and permits an explicit retry', async (t) => {
  const directory = testDirectory()
  const store = createStore(join(directory, 'runtime.sqlite'))
  t.after(() => {
    store.close()
    rmSync(directory, { recursive: true, force: true })
  })
  const first = idempotencyInput()
  assert.deepEqual(await store.claim(first), { status: 'claimed' })
  await assert.rejects(
    store.complete(first, { content: 'x'.repeat(1024 * 1024) }),
    (error) => error.code === 'SQLITE_RESULT_INVALID',
  )
  await store.release(first)
  assert.deepEqual(await store.claim({
    ...first,
    invocationId: 'invocation-after-large-result',
  }), { status: 'claimed' })
})

test('rejects an unknown schema version and operations after close', async (t) => {
  const directory = testDirectory()
  const databasePath = join(directory, 'runtime.sqlite')
  let validStore
  t.after(() => {
    validStore?.close()
    rmSync(directory, { recursive: true, force: true })
  })
  const database = new DatabaseSync(databasePath)
  database.exec(`
    CREATE TABLE yilong_merchant_runtime_idempotency_meta (
      key TEXT PRIMARY KEY NOT NULL,
      value TEXT NOT NULL
    ) STRICT;
    INSERT INTO yilong_merchant_runtime_idempotency_meta (key, value)
    VALUES ('schema_version', '99');
  `)
  database.close()

  assert.throws(
    () => createStore(databasePath),
    (error) => error.code === 'SQLITE_SCHEMA_UNSUPPORTED',
  )
  const unchanged = new DatabaseSync(databasePath)
  assert.equal(unchanged.prepare(
    `SELECT name FROM sqlite_master
      WHERE type = 'table' AND name = 'yilong_merchant_runtime_idempotency'`,
  ).get(), undefined)
  unchanged.close()

  validStore = createStore(join(directory, 'closed.sqlite'))
  validStore.close()
  await assert.rejects(
    validStore.claim(idempotencyInput()),
    (error) => error.code === 'SQLITE_STORE_CLOSED',
  )
})

test('maps a competing SQLite write lock to a stable busy error', async (t) => {
  const directory = testDirectory()
  const databasePath = join(directory, 'runtime.sqlite')
  const store = createStore(databasePath, { busyTimeoutMs: 0 })
  const locker = new DatabaseSync(databasePath)
  let locked = false
  t.after(() => {
    if (locked) locker.exec('ROLLBACK')
    locker.close()
    store.close()
    rmSync(directory, { recursive: true, force: true })
  })
  locker.exec('BEGIN IMMEDIATE')
  locked = true

  await assert.rejects(
    store.claim(idempotencyInput()),
    (error) => error.code === 'SQLITE_STORE_BUSY',
  )
})

function createStore(path, overrides = {}) {
  return createSqliteMerchantRuntimeIdempotencyStore({
    path,
    takeoverAfterMs: 1_000,
    busyTimeoutMs: 1_000,
    ...overrides,
  })
}

function idempotencyInput(overrides = {}) {
  return {
    merchantId: 'merchant-sqlite',
    requesterAppId: 'app-sqlite',
    requesterUserId: 'consumer-sqlite',
    credentialEnvironment: 'sandbox',
    credentialId: 'credential-sqlite',
    capabilityKey: 'catalog.search',
    actionConfirmationId: null,
    idempotencyKey: 'idempotency-sqlite',
    invocationId: 'invocation-sqlite-first',
    requestHash: 'a'.repeat(64),
    ...overrides,
  }
}

function testDirectory() {
  return mkdtempSync(join(tmpdir(), 'yilong-runtime-sqlite-'))
}
