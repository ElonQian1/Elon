import type {
  CreateOpenCommerceCapability,
  OpenCommerceCapability,
  OpenCommerceGrant,
} from './openCommerceTypes'

export const MOBILE_CAPTURE_CAPABILITY: 'merchant.mobile_capture.session.launch'
export const MOBILE_CAPTURE_TARGET: 'merchant_platforms'
export const MOBILE_CAPTURE_ANDROID_PACKAGE: 'com.cofficethinking.manager'

export interface MobileCaptureLaunch {
  launchUrl: string
  androidIntentUrl: string
  apiBaseUrl: string
  expiresAtUnix: number
}

export function mobileCaptureCapabilityDefinition(): CreateOpenCommerceCapability
export function assertCompatibleMobileCaptureCapability(
  capability: OpenCommerceCapability | undefined,
): OpenCommerceCapability
export function selectUsableMobileCaptureGrant(
  grants: OpenCommerceGrant[],
  merchantId: string,
  appId: string,
  now?: number,
): OpenCommerceGrant | undefined
export function parseMobileCaptureInvocation(
  invocation: Record<string, unknown>,
  nowSeconds?: number,
): MobileCaptureLaunch
