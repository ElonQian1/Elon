import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { request as httpRequest } from 'node:http'
import { request as httpsRequest } from 'node:https'
import test from 'node:test'

import {
  MERCHANT_RUNTIME_HTTP_ERROR_SCHEMA,
  MERCHANT_RUNTIME_HTTP_HEALTH_SCHEMA,
  createMerchantRuntimeHttpHost,
} from '../src/merchant-runtime-http-host.js'
import {
  createTestRuntime,
  deferred,
  signedInvocation,
} from './merchant-runtime-fixtures.mjs'

const fixtureKey = new URL('./fixtures/localhost-key.pem', import.meta.url)
const fixtureCertificate = new URL('./fixtures/localhost-cert.pem', import.meta.url)

test('real HTTP requests preserve signed bytes and idempotent runtime replay', async () => {
  let executions = 0
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'(input) {
        executions += 1
        return { items: [{ id: 'product-1', query: input.query }] }
      },
    },
  })
  const host = createMerchantRuntimeHttpHost({ runtime })
  const listening = await host.listen()
  try {
    assert.equal(listening.address, '127.0.0.1')
    const health = await requestJson(listening.origin, '/healthz')
    assert.equal(health.status, 200)
    assert.deepEqual(health.body, {
      schema: MERCHANT_RUNTIME_HTTP_HEALTH_SCHEMA,
      status: 'ready',
    })
    assert.equal(executions, 0)

    const invocation = currentSignedInvocation()
    const first = await invoke(listening.origin, invocation)
    const replay = await invoke(listening.origin, invocation)
    assert.equal(first.status, 200)
    assert.deepEqual(replay.body, first.body)
    assert.equal(executions, 1)
    assert.equal(first.headers['cache-control'], 'no-store')
    assert.equal(first.headers['x-content-type-options'], 'nosniff')
  } finally {
    await host.close()
  }
})

test('host rejects route, method, media type, encoding, size, and expectations before runtime', async () => {
  let invocations = 0
  const host = createMerchantRuntimeHttpHost({
    maxBodyBytes: 1_024,
    runtime: {
      async handleInvoke() {
        invocations += 1
        return { status: 200, body: {} }
      },
    },
  })
  const listening = await host.listen()
  try {
    const route = await requestJson(listening.origin, '/unknown')
    const method = await requestJson(listening.origin, '/commerce/v1/invoke')
    const media = await requestJson(listening.origin, '/commerce/v1/invoke', {
      method: 'POST',
      body: '{}',
      headers: { 'content-type': 'text/plain' },
    })
    const encoding = await requestJson(listening.origin, '/commerce/v1/invoke', {
      method: 'POST',
      body: '{}',
      headers: { 'content-encoding': 'gzip', 'content-type': 'application/json' },
    })
    const declaredLarge = await requestJson(listening.origin, '/commerce/v1/invoke', {
      method: 'POST',
      body: 'x'.repeat(1_025),
      headers: { 'content-type': 'application/json' },
    })
    const streamedLarge = await requestJson(listening.origin, '/commerce/v1/invoke', {
      method: 'POST',
      chunks: ['x'.repeat(700), 'y'.repeat(700)],
      headers: { 'content-type': 'application/json' },
    })
    const expectation = await requestJson(listening.origin, '/commerce/v1/invoke', {
      method: 'POST',
      headers: { 'content-type': 'application/json', expect: '100-continue' },
    })

    assert.equal(route.status, 404)
    assert.equal(method.status, 405)
    assert.equal(method.headers.allow, 'POST')
    assert.equal(media.status, 415)
    assert.equal(encoding.status, 415)
    assert.equal(declaredLarge.status, 413)
    assert.equal(streamedLarge.status, 413)
    assert.equal(expectation.status, 417)
    assert.equal(invocations, 0)
  } finally {
    await host.close()
  }
})

test('runtime authentication errors pass through while host defects stay generic', async () => {
  const signedHost = createMerchantRuntimeHttpHost({ runtime: createTestRuntime() })
  const signedListening = await signedHost.listen()
  try {
    const now = Math.floor(Date.now() / 1_000)
    const rejected = signedInvocation(
      { issued_at_unix: now },
      { timestamp: now, secret: 'wrong-test-secret-that-is-at-least-32-bytes' },
    )
    const response = await invoke(signedListening.origin, rejected)
    assert.equal(response.status, 401)
    assert.equal(response.body.schema, 'merchant_runtime.error.v1')
    assert.equal(response.body.error_code, 'signature_rejected')
  } finally {
    await signedHost.close()
  }

  for (const runtime of [
    { async handleInvoke() { throw new Error('private runtime detail') } },
    { async handleInvoke() { return { status: 200, body: 'invalid' } } },
    { async handleInvoke() { return { status: 302, body: { redirect: true } } } },
  ]) {
    const host = createMerchantRuntimeHttpHost({ runtime })
    const listening = await host.listen()
    try {
      const response = await requestJson(listening.origin, '/commerce/v1/invoke', {
        method: 'POST',
        body: '{}',
        headers: { 'content-type': 'application/json' },
      })
      assert.equal(response.status, 500)
      assert.equal(response.body.schema, MERCHANT_RUNTIME_HTTP_ERROR_SCHEMA)
      assert.equal(JSON.stringify(response.body).includes('private runtime detail'), false)
    } finally {
      await host.close()
    }
  }
})

test('graceful close waits for an in-flight request and is idempotent', async () => {
  const entered = deferred()
  const release = deferred()
  const host = createMerchantRuntimeHttpHost({
    runtime: {
      async handleInvoke() {
        entered.resolve()
        await release.promise
        return { status: 200, body: { completed: true } }
      },
    },
  })
  const listening = await host.listen()
  const responsePromise = requestJson(listening.origin, '/commerce/v1/invoke', {
    method: 'POST',
    body: '{}',
    headers: { 'content-type': 'application/json' },
  })
  await entered.promise
  const closePromise = host.close({ graceMs: 2_000 })
  assert.equal(host.state().status, 'draining')
  assert.equal(await settledWithin(closePromise, 25), false)

  release.resolve()
  const response = await responsePromise
  const receipt = await closePromise
  assert.equal(response.status, 200)
  assert.equal(receipt.forced, false)
  assert.equal(receipt.in_flight_at_start, 1)
  assert.deepEqual(await host.close({ graceMs: 0 }), receipt)
  assert.equal(host.state().status, 'closed')
  await assert.rejects(host.listen(), /cannot listen from closed/)
})

test('grace deadline closes hanging connections without claiming task completion', async () => {
  const entered = deferred()
  const never = deferred()
  const host = createMerchantRuntimeHttpHost({
    runtime: {
      async handleInvoke() {
        entered.resolve()
        await never.promise
        return { status: 200, body: {} }
      },
    },
  })
  const listening = await host.listen()
  const responsePromise = requestJson(listening.origin, '/commerce/v1/invoke', {
    method: 'POST',
    body: '{}',
    headers: { 'content-type': 'application/json' },
  }).catch((error) => error)
  await entered.promise
  const receipt = await host.close({ graceMs: 20 })
  assert.equal(receipt.forced, true)
  assert.equal(receipt.in_flight_at_start, 1)
  assert.equal(receipt.remaining_in_flight, 1)
  assert.ok(await responsePromise instanceof Error)
})

test('caller-provided TLS uses the same bounded request handler', async () => {
  const [key, cert] = await Promise.all([
    readFile(fixtureKey),
    readFile(fixtureCertificate),
  ])
  const host = createMerchantRuntimeHttpHost({
    protocol: 'https',
    tls: { key, cert },
    runtime: createTestRuntime(),
  })
  const listening = await host.listen()
  try {
    const health = await requestJson(listening.origin, '/healthz', {
      rejectUnauthorized: false,
    })
    const response = await invoke(listening.origin, currentSignedInvocation(), {
      rejectUnauthorized: false,
    })
    assert.equal(health.status, 200)
    assert.equal(response.status, 200)
    assert.equal(listening.protocol, 'https')
  } finally {
    await host.close()
  }
})

test('host configuration fails closed before binding a port', () => {
  const runtime = { async handleInvoke() { return { status: 200, body: {} } } }
  assert.throws(() => createMerchantRuntimeHttpHost({ runtime, protocol: 'ftp' }), /http or https/)
  assert.throws(() => createMerchantRuntimeHttpHost({ runtime, protocol: 'https' }), /options.tls/)
  assert.throws(() => createMerchantRuntimeHttpHost({ runtime, protocol: 'http', tls: {} }), /only valid/)
  assert.throws(() => createMerchantRuntimeHttpHost({ runtime, invokePath: '/invoke?unsafe=1' }), /exact/)
  assert.throws(() => createMerchantRuntimeHttpHost({ runtime, headersTimeoutMs: 20_000 }), /between/)
})

function currentSignedInvocation(overrides = {}) {
  const now = Math.floor(Date.now() / 1_000)
  return signedInvocation({ issued_at_unix: now, ...overrides }, { timestamp: now })
}

function invoke(origin, invocation, options = {}) {
  return requestJson(origin, '/commerce/v1/invoke', {
    method: 'POST',
    body: invocation.body,
    headers: {
      ...invocation.headers,
      'content-type': 'application/json',
    },
    ...options,
  })
}

function requestJson(origin, path, options = {}) {
  const target = new URL(path, origin)
  const transport = target.protocol === 'https:' ? httpsRequest : httpRequest
  const body = options.body === undefined ? undefined : Buffer.from(options.body)
  const headers = { ...(options.headers ?? {}) }
  if (body && options.chunks === undefined && headers['content-length'] === undefined) {
    headers['content-length'] = String(body.length)
  }
  return new Promise((resolve, reject) => {
    const request = transport({
      protocol: target.protocol,
      hostname: target.hostname,
      port: target.port,
      path: `${target.pathname}${target.search}`,
      method: options.method ?? 'GET',
      headers,
      rejectUnauthorized: options.rejectUnauthorized,
    }, (response) => {
      const chunks = []
      response.on('data', (chunk) => chunks.push(chunk))
      response.on('end', () => {
        const text = Buffer.concat(chunks).toString('utf8')
        let parsed = null
        if (text) parsed = JSON.parse(text)
        resolve({ status: response.statusCode, headers: response.headers, body: parsed })
      })
    })
    request.on('error', reject)
    if (options.chunks) {
      for (const chunk of options.chunks) request.write(chunk)
    } else if (body) {
      request.write(body)
    }
    request.end()
  })
}

async function settledWithin(promise, milliseconds) {
  const marker = Symbol('pending')
  return await Promise.race([
    promise.then(() => true),
    new Promise((resolve) => setTimeout(() => resolve(marker), milliseconds)),
  ]) !== marker
}
