import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import { safeNodeAdminUrl } from '../../../lib/utils'

export interface AndroidInspectorDevice {
  serial: string
  state: string
  product?: string
  model?: string
  device?: string
  transportId?: string
}

export interface AndroidInspectorBounds {
  left: number
  top: number
  right: number
  bottom: number
  width: number
  height: number
}

export interface AndroidInspectorSource {
  file: string
  line?: number
  token: string
  confidence: number
  reason: string
}

export interface AndroidInspectorNode {
  id: string
  depth: number
  indexPath: number[]
  xpath: string
  text: string
  contentDesc: string
  resourceId?: string
  packageName?: string
  className?: string
  bounds: AndroidInspectorBounds
  clickable: boolean
  enabled: boolean
  focusable: boolean
  focused: boolean
  scrollable: boolean
  checkable: boolean
  checked: boolean
  selected: boolean
  password: boolean
  visible: boolean
  source?: AndroidInspectorSource
}

export interface AndroidInspectorSnapshot {
  ok: boolean
  deviceId: string
  packageName?: string
  activityName?: string
  capturedAt: string
  screenshot?: {
    dataUrl?: string
    mimeType: string
    width: number
    height: number
    bytes: number
  }
  xml: {
    nodeCount: number
    length: number
    rawXml?: string
  }
  nodes: AndroidInspectorNode[]
  sourceRoot?: string
}

export interface AndroidInspectorStatus {
  ok: boolean
  adb?: {
    available: boolean
    adbPath: string
    version?: string
    error?: string
  }
}

const DEFAULT_PACKAGE = 'com.elon.app'

function inspectorError(error: unknown, fallback: string): Error {
  const raw = error instanceof Error ? error.message : String(error ?? '')
  const message = raw.trim()
  const lower = message.toLowerCase()
  if (
    lower.includes('failed to fetch')
    || lower.includes('networkerror')
    || lower.includes('load failed')
    || lower.includes('abort')
    || lower.includes('timeout')
    || lower.includes('timed out')
  ) {
    return new Error(`${fallback}：本机节点没有响应，请先启动或更新 Windows PC 节点客户端`)
  }
  if (message.includes('HTTP 404')) {
    return new Error(`${fallback}：本机节点版本过旧，请更新 Windows PC 节点客户端后重试`)
  }
  return new Error(message ? `${fallback}：${message}` : fallback)
}

export function inspectorAdminUrl(): string {
  return safeNodeAdminUrl()
}

export async function probeAndroidInspector(): Promise<AndroidInspectorStatus> {
  const adminUrl = inspectorAdminUrl()
  await probeLocalNode(adminUrl)
  return nodeApi<AndroidInspectorStatus>(adminUrl, '/api/android-inspector/status', {}, 5000)
}

export async function listAndroidDevices(): Promise<AndroidInspectorDevice[]> {
  try {
    const data = await nodeApi<{ devices?: AndroidInspectorDevice[] }>(
      inspectorAdminUrl(),
      '/api/android-inspector/devices',
      {},
      8000,
    )
    return data.devices ?? []
  } catch (error) {
    throw inspectorError(error, '读取 ADB 设备失败')
  }
}

export async function connectAndroidDevice(address: string): Promise<string> {
  try {
    const data = await nodeApi<{ output?: string }>(
      inspectorAdminUrl(),
      '/api/android-inspector/connect',
      {
        method: 'POST',
        body: JSON.stringify({ address: address.trim() }),
      },
      12000,
    )
    return data.output ?? ''
  } catch (error) {
    throw inspectorError(error, '无线 ADB 连接失败')
  }
}

export async function captureAndroidSnapshot(input: {
  deviceId: string
  packageName?: string
  launchApp?: boolean
  projectRoot?: string
}): Promise<AndroidInspectorSnapshot> {
  try {
    return await nodeApi<AndroidInspectorSnapshot>(
      inspectorAdminUrl(),
      '/api/android-inspector/capture',
      {
        method: 'POST',
        body: JSON.stringify({
          deviceId: input.deviceId,
          packageName: input.packageName?.trim() || DEFAULT_PACKAGE,
          launchApp: input.launchApp ?? false,
          includeRawXml: false,
          includeScreenshotDataUrl: true,
          projectRoot: input.projectRoot?.trim() || undefined,
        }),
      },
      18000,
    )
  } catch (error) {
    throw inspectorError(error, '真机捕获失败')
  }
}
