import type { PwaRuntimeCaptureResult } from './pwaVerificationModel'

export interface PreparedPwaAuthProfile {
  profile: string
  expiresAt: string
  remembered: boolean
  accountLabel?: string | null
}

export interface PwaRuntimeAuthTransport {
  prepare: (
    projectRoot: string,
    token: string | null,
    accountLabel: string | null,
  ) => Promise<PreparedPwaAuthProfile>
  capture: (profile: string) => Promise<PwaRuntimeCaptureResult>
  cleanup: (projectRoot: string, profile: string) => Promise<unknown>
}

const AUTO_PROFILE_PATTERN = /^pc_ui_tuner_[0-9a-f]{32}$/

function failed(
  code: string,
  message: string,
  retryable: boolean,
  nextStep: string,
): PwaRuntimeCaptureResult {
  return {
    ok: false,
    status: 'CAPTURE_FAILED',
    diagnostic: { code, message, retryable, nextStep },
    base64Embedded: false,
  }
}

export async function captureWithTemporaryPwaAuthProfile(
  projectRoot: string,
  token: string | null,
  accountLabel: string | null,
  transport: PwaRuntimeAuthTransport,
): Promise<PwaRuntimeCaptureResult> {
  let prepared: PreparedPwaAuthProfile
  try {
    prepared = await transport.prepare(projectRoot, token, accountLabel)
  } catch (error) {
    if (!token) {
      return failed(
        'AUTHENTICATION_REQUIRED',
        '当前浏览器和 Windows 节点都没有可复用的一龙登录态',
        false,
        '在这台 PC 登录一次；后续 PWA 验证会复用 Windows 当前用户保护的登录态',
      )
    }
    throw error
  }
  if (!AUTO_PROFILE_PATTERN.test(prepared.profile)) {
    return failed(
      'AUTH_PROFILE_PREPARE_FAILED',
      'PC 节点没有返回安全的临时 PWA 登录态名称',
      false,
      '更新 PC 节点后重试；不要手工创建或传递登录 secret',
    )
  }

  let capture: PwaRuntimeCaptureResult | undefined
  let captureError: unknown
  try {
    capture = await transport.capture(prepared.profile)
  } catch (error) {
    captureError = error
  }

  try {
    await transport.cleanup(projectRoot, prepared.profile)
  } catch {
    return failed(
      'AUTH_PROFILE_CLEANUP_FAILED',
      '临时 PWA 登录态未能确认清理，已拒绝接受本次 PNG',
      true,
      '确认项目 .elon/ui-tuner/pwa-sessions 未被占用后重试',
    )
  }

  if (captureError) throw captureError
  return capture ?? failed(
    'CAPTURE_FAILED',
    'PC 节点未返回 PWA PNG 捕获结果',
    true,
    '确认本机节点与 PWA Runtime 后重试',
  )
}
