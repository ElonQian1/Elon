const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { pathToFileURL } = require('node:url')

async function main() {
  const protocol = await import(pathToFileURL(path.resolve(
    __dirname,
    '../src/features/open-commerce/merchantMobileCaptureProtocol.js',
  )).href)
  const contract = JSON.parse(fs.readFileSync(path.resolve(
    __dirname,
    '../../contracts/open-commerce/mobile-capture-capability-v1.json',
  ), 'utf8'))
  assert.equal(contract.capability_key, protocol.MOBILE_CAPTURE_CAPABILITY)
  assert.equal(contract.input_schema.properties.target.const, protocol.MOBILE_CAPTURE_TARGET)
  assert.equal(contract.result.android_package, protocol.MOBILE_CAPTURE_ANDROID_PACKAGE)
  assert.equal(contract.result.browser_credentials_transport, 'never')
  const nowSeconds = 1_800_000_000
  const ticket = 'a'.repeat(64)
  const baseUrl = 'https://182.254.168.75'
  const launchUrl = `cofficethinking://mobile-capture/enroll?ticket=${ticket}&base_url=${encodeURIComponent(baseUrl)}`
  const intentUrl = `intent://mobile-capture/enroll?ticket=${ticket}&base_url=${encodeURIComponent(baseUrl)}#Intent;scheme=cofficethinking;package=com.cofficethinking.manager;end`
  const invocation = {
    result: {
      schema: 'merchant_mobile_capture.launch.v1',
      launch_url: launchUrl,
      android_intent_url: intentUrl,
      android_package: 'com.cofficethinking.manager',
      exchange_url: `${baseUrl}/api/v1/mobile-capture/exchange`,
      expires_at_unix: nowSeconds + 120,
      ticket_single_use: true,
    },
  }

  assert.deepEqual(protocol.parseMobileCaptureInvocation(invocation, nowSeconds), {
    launchUrl,
    androidIntentUrl: intentUrl,
    apiBaseUrl: baseUrl,
    expiresAtUnix: nowSeconds + 120,
  })
  assert.equal(protocol.mobileCaptureCapabilityDefinition().access_level, 'authorized')
  assert.equal(protocol.mobileCaptureCapabilityDefinition().handler_type, 'merchant_runtime')

  const compatible = {
    capability_key: protocol.MOBILE_CAPTURE_CAPABILITY,
    status: 'active',
    kind: 'action',
    access_level: 'authorized',
    handler_type: 'merchant_runtime',
    unit_price_micros: 0,
  }
  assert.equal(protocol.assertCompatibleMobileCaptureCapability(compatible), compatible)
  assert.throws(
    () => protocol.assertCompatibleMobileCaptureCapability({ ...compatible, access_level: 'public' }),
    /不兼容/,
  )

  const grants = [
    grant({ id: 'wrong-app', grantee_app_id: 'other' }),
    grant({ id: 'revoked', revoked_at: '2026-01-01T00:00:00Z' }),
    grant({ id: 'expired', expires_at: new Date((nowSeconds - 1) * 1000).toISOString() }),
    grant({ id: 'exhausted', max_invocations: 1, used_invocations: 1 }),
    grant({ id: 'usable', created_at: '2027-01-02T00:00:00Z' }),
  ]
  assert.equal(
    protocol.selectUsableMobileCaptureGrant(grants, 'merchant-1', 'mobile-app', nowSeconds * 1000).id,
    'usable',
  )

  assertRejects(protocol, invocation, nowSeconds, { android_package: 'com.example.fake' })
  assertRejects(protocol, invocation, nowSeconds, { expires_at_unix: nowSeconds - 1 })
  assertRejects(protocol, invocation, nowSeconds, {
    launch_url: launchUrl.replace('https%3A', 'http%3A'),
  })
  assertRejects(protocol, invocation, nowSeconds, {
    exchange_url: 'https://attacker.example/api/v1/mobile-capture/exchange',
  })
  assertRejects(protocol, invocation, nowSeconds, {
    android_intent_url: intentUrl.replace('com.cofficethinking.manager', 'com.example.fake'),
  })

  process.stdout.write('merchant mobile capture protocol tests passed\n')
}

function grant(overrides) {
  return {
    id: 'base',
    merchant_id: 'merchant-1',
    grantee_app_id: 'mobile-app',
    scopes: ['merchant.mobile_capture.session.launch'],
    used_invocations: 0,
    created_at: '2027-01-01T00:00:00Z',
    ...overrides,
  }
}

function assertRejects(protocol, invocation, nowSeconds, override) {
  assert.throws(
    () => protocol.parseMobileCaptureInvocation({
      result: { ...invocation.result, ...override },
    }, nowSeconds),
  )
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
