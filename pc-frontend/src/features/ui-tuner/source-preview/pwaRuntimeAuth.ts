import type { PwaRuntimeCaptureResult } from './pwaVerificationModel'

export interface PreparedPwaAuthProfile {
  profile: string
  expiresAt: string
}

export interface PwaRuntimeAuthTransport {
  prepare: (projectRoot: string, token: string) => Promise<PreparedPwaAuthProfile>
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
  transport: PwaRuntimeAuthTransport,
): Promise<PwaRuntimeCaptureResult> {
  if (!token) {
    return failed(
      'AUTHENTICATION_REQUIRED',
      '当前一龙登录态缺失，无法捕获需要登录的真实 PWA',
      false,
      '重新登录一龙后再执行真实 PWA PNG 捕获',
    )
  }

  const prepared = await transport.prepare(projectRoot, token)
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
