import assert from 'node:assert/strict'
import test from 'node:test'

import {
  createTestRuntime,
  deferred,
  signedInvocation,
} from './merchant-runtime-fixtures.mjs'

test('successful invocations replay one immutable business result', async () => {
  let calls = 0
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'() {
        calls += 1
        return { items: [{ id: 'product-1' }] }
      },
    },
  })

  const first = await runtime.handleInvoke(signedInvocation())
  first.body.result.items[0].id = 'mutated-by-caller'
  const replay = await runtime.handleInvoke(signedInvocation({ invocation_id: 'invocation-demo-2' }))

  assert.equal(first.status, 200)
  assert.equal(replay.status, 200)
  assert.equal(replay.body.result.items[0].id, 'product-1')
  assert.equal(calls, 1)
})

test('the same idempotency key with different logical input is rejected', async () => {
  const runtime = createTestRuntime()
  const first = await runtime.handleInvoke(signedInvocation())
  const conflict = await runtime.handleInvoke(signedInvocation({
    invocation_id: 'invocation-demo-2',
    input: { query: 'espresso' },
  }))

  assert.equal(first.status, 200)
  assert.equal(conflict.status, 409)
  assert.equal(conflict.body.error_code, 'idempotency_conflict')
})

test('a busy duplicate cannot release the active invocation claim', async () => {
  const entered = deferred()
  const finish = deferred()
  let calls = 0
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'() {
        calls += 1
        entered.resolve()
        await finish.promise
        return { items: [] }
      },
    },
  })
  const request = signedInvocation()

  const active = runtime.handleInvoke(request)
  await entered.promise
  const busy = await runtime.handleInvoke(request)
  finish.resolve()
  const completed = await active
  const replay = await runtime.handleInvoke(signedInvocation({ invocation_id: 'invocation-demo-2' }))

  assert.equal(busy.status, 409)
  assert.equal(busy.body.error_code, 'idempotency_busy')
  assert.equal(completed.status, 200)
  assert.equal(replay.status, 200)
  assert.equal(calls, 1)
})

test('handler failures release an owned claim so the caller can retry', async () => {
  let calls = 0
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'() {
        calls += 1
        if (calls === 1) throw new Error('sensitive merchant database error')
        return { items: [] }
      },
    },
  })

  const failed = await runtime.handleInvoke(signedInvocation())
  const retry = await runtime.handleInvoke(signedInvocation({ invocation_id: 'invocation-demo-2' }))

  assert.equal(failed.status, 500)
  assert.equal(failed.body.error_code, 'internal_error')
  assert.equal(failed.body.message.includes('sensitive'), false)
  assert.equal(retry.status, 200)
  assert.equal(calls, 2)
})

test('ownership loss returns a conflict and releases only the claim that was acquired', async () => {
  const released = []
  const runtime = createTestRuntime({
    idempotencyStore: {
      async claim() { return { status: 'claimed' } },
      async complete() { return false },
      async release(input) { released.push(input.invocationId) },
    },
  })

  const response = await runtime.handleInvoke(signedInvocation())

  assert.equal(response.status, 409)
  assert.equal(response.body.error_code, 'idempotency_ownership_lost')
  assert.deepEqual(released, ['invocation-demo-1'])
})
