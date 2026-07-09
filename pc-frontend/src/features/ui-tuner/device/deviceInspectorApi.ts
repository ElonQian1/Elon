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

export function inspectorAdminUrl(): string {
  return safeNodeAdminUrl()
}

export async function probeAndroidInspector(): Promise<AndroidInspectorStatus> {
  const adminUrl = inspectorAdminUrl()
  await probeLocalNode(adminUrl)
  return nodeApi<AndroidInspectorStatus>(adminUrl, '/api/android-inspector/status', {}, 5000)
}

export async function listAndroidDevices(): Promise<AndroidInspectorDevice[]> {
  const data = await nodeApi<{ devices?: AndroidInspectorDevice[] }>(
    inspectorAdminUrl(),
    '/api/android-inspector/devices',
    {},
    8000,
  )
  return data.devices ?? []
}

export async function connectAndroidDevice(address: string): Promise<string> {
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
}

export async function captureAndroidSnapshot(input: {
  deviceId: string
  packageName?: string
  launchApp?: boolean
  projectRoot?: string
}): Promise<AndroidInspectorSnapshot> {
  return nodeApi<AndroidInspectorSnapshot>(
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
}
