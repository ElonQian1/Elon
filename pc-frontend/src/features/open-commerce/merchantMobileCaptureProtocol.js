export const MOBILE_CAPTURE_CAPABILITY = 'merchant.mobile_capture.session.launch'
export const MOBILE_CAPTURE_TARGET = 'merchant_platforms'
export const MOBILE_CAPTURE_ANDROID_PACKAGE = 'com.cofficethinking.manager'

const CAPABILITY_INPUT_SCHEMA = Object.freeze({
  type: 'object',
  required: ['target'],
  properties: { target: { const: MOBILE_CAPTURE_TARGET } },
  additionalProperties: false,
})

const CAPABILITY_OUTPUT_SCHEMA = Object.freeze({
  type: 'object',
  required: [
    'schema',
    'launch_url',
    'android_intent_url',
    'android_package',
    'exchange_url',
    'expires_at_unix',
    'ticket_single_use',
  ],
  properties: {
    schema: { const: 'merchant_mobile_capture.launch.v1' },
    launch_url: { type: 'string' },
    android_intent_url: { type: 'string' },
    android_package: { const: MOBILE_CAPTURE_ANDROID_PACKAGE },
    exchange_url: { type: 'string' },
    expires_at_unix: { type: 'integer' },
    ticket_single_use: { const: true },
  },
  additionalProperties: false,
})

export function mobileCaptureCapabilityDefinition() {
  return {
    capability_key: MOBILE_CAPTURE_CAPABILITY,
    display_name: '手机商户平台账号绑定',
    description: '为商户本人签发一次性手机设备绑定入口；网页登录凭据不离开手机。',
    kind: 'action',
    access_level: 'authorized',
    input_schema: CAPABILITY_INPUT_SCHEMA,
    output_schema: CAPABILITY_OUTPUT_SCHEMA,
    handler_type: 'merchant_runtime',
    unit_price_micros: 0,
    currency: 'CNY',
    freshness_seconds: 0,
  }
}

export function assertCompatibleMobileCaptureCapability(capability) {
  if (!capability || typeof capability !== 'object') {
    throw new Error('手机平台账号能力尚未登记。')
  }
  if (capability.capability_key !== MOBILE_CAPTURE_CAPABILITY
    || capability.status !== 'active'
    || capability.kind !== 'action'
    || capability.access_level !== 'authorized'
    || capability.handler_type !== 'merchant_runtime'
    || capability.unit_price_micros !== 0) {
    throw new Error('现有手机平台账号能力与受控绑定协议不兼容，请先修正能力配置。')
  }
  return capability
}

export function selectUsableMobileCaptureGrant(grants, merchantId, appId, now = Date.now()) {
  return [...grants]
    .filter((grant) => grant.merchant_id === merchantId
      && grant.grantee_app_id === appId
      && !grant.revoked_at
      && grant.scopes.includes(MOBILE_CAPTURE_CAPABILITY)
      && (!grant.expires_at || Date.parse(grant.expires_at) > now)
      && (grant.max_invocations == null || grant.used_invocations < grant.max_invocations))
    .sort((left, right) => Date.parse(right.created_at) - Date.parse(left.created_at))[0]
}

export function parseMobileCaptureInvocation(invocation, nowSeconds = Math.floor(Date.now() / 1000)) {
  const root = requireRecord(invocation, '商户运行时返回无效。')
  const result = requireRecord(root.result, '商户运行时没有返回手机绑定结果。')
  if (result.schema !== 'merchant_mobile_capture.launch.v1'
    || result.android_package !== MOBILE_CAPTURE_ANDROID_PACKAGE
    || result.ticket_single_use !== true) {
    throw new Error('商户运行时返回了不受支持的手机绑定协议。')
  }

  const expiresAtUnix = requireInteger(result.expires_at_unix, '手机绑定入口缺少有效期。')
  if (expiresAtUnix <= nowSeconds || expiresAtUnix > nowSeconds + 300) {
    throw new Error('手机绑定入口已过期或有效期异常。')
  }

  const launchUrl = requireText(result.launch_url, '手机绑定入口缺失。')
  const parsedLaunch = new URL(launchUrl)
  if (parsedLaunch.protocol !== 'cofficethinking:'
    || parsedLaunch.hostname !== 'mobile-capture'
    || parsedLaunch.pathname !== '/enroll'
    || parsedLaunch.username
    || parsedLaunch.password
    || parsedLaunch.hash
    || [...parsedLaunch.searchParams.keys()].some((key) => !['ticket', 'base_url'].includes(key))
    || [...parsedLaunch.searchParams.keys()].length !== 2) {
    throw new Error('手机绑定入口地址不受信任。')
  }

  const ticket = parsedLaunch.searchParams.get('ticket') || ''
  if (!/^[a-f0-9]{64}$/i.test(ticket)) throw new Error('手机绑定票据格式无效。')
  const apiBaseUrl = parseHttpsOrigin(parsedLaunch.searchParams.get('base_url'))
  const exchangeUrl = requireText(result.exchange_url, '手机绑定交换地址缺失。')
  if (exchangeUrl !== `${apiBaseUrl}/api/v1/mobile-capture/exchange`) {
    throw new Error('手机绑定交换地址与受信任服务不一致。')
  }

  const androidIntentUrl = requireText(result.android_intent_url, '安卓应用入口缺失。')
  const expectedIntentUrl = `intent://mobile-capture/enroll?ticket=${ticket}&base_url=${encodeURIComponent(apiBaseUrl)}#Intent;scheme=cofficethinking;package=${MOBILE_CAPTURE_ANDROID_PACKAGE};end`
  if (androidIntentUrl !== expectedIntentUrl) {
    throw new Error('安卓应用入口包名或参数不受信任。')
  }

  return {
    launchUrl,
    androidIntentUrl,
    apiBaseUrl,
    expiresAtUnix,
  }
}

function parseHttpsOrigin(value) {
  const text = requireText(value, '手机绑定服务地址缺失。')
  const url = new URL(text)
  if (url.protocol !== 'https:'
    || url.username
    || url.password
    || url.pathname !== '/'
    || url.search
    || url.hash
    || url.origin !== text) {
    throw new Error('手机绑定服务必须使用纯 HTTPS origin。')
  }
  return url.origin
}

function requireRecord(value, message) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(message)
  return value
}

function requireText(value, message) {
  if (typeof value !== 'string' || !value.trim()) throw new Error(message)
  return value.trim()
}

function requireInteger(value, message) {
  if (!Number.isSafeInteger(value)) throw new Error(message)
  return value
}
