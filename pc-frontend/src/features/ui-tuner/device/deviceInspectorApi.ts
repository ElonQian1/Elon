import { nodeApi, probeLocalNode } from '../../node/localNodeApi'
import { safeNodeAdminUrl } from '../../../lib/utils'

export interface AndroidInspectorDevice {
  serial: string
  state: string
  hardwareSerial?: string
  connectionType?: 'usb' | 'wireless' | 'emulator'
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
  matchKind: 'resource_id_xml' | 'resource_id_code' | 'compose_semantics' | string
  componentKey: string
  scope: 'component' | 'repeated_component' | string
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
  sourceCandidates: AndroidInspectorSource[]
}

export interface AndroidSnapshotArtifact {
  id: string
  rootDir: string
  manifestPath: string
  screenshotPath: string
  hierarchyPath: string
  rawXmlPath?: string
}

export interface AndroidSelectionArtifact {
  snapshotId: string
  selectionId: string
  cropPath: string
  contextPath: string
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
    error?: string
  }
  nodes: AndroidInspectorNode[]
  sourceRoot?: string
  sourceFingerprint?: string
  sourceBindingsPath?: string
  artifact?: AndroidSnapshotArtifact
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

export interface AndroidDeviceProfile {
  id: string
  displayName: string
  hardwareSerial: string
  manufacturer?: string
  model?: string
  androidSdk?: number
  androidRelease?: string
  wirelessMode: 'unknown' | 'tls' | 'legacy' | 'manual'
  paired: boolean
  lastEndpoint?: string
  createdAt: string
  lastSeenAt: string
  connectionState: 'connected_usb' | 'connected_wireless' | 'paired_offline' | 'offline'
  connectedDeviceId?: string
}

export interface AdbMdnsService {
  name: string
  serviceType: string
  address: string
}

export interface AndroidWirelessStatus {
  ok: boolean
  adb: NonNullable<AndroidInspectorStatus['adb']>
  devices: AndroidInspectorDevice[]
  profiles: AndroidDeviceProfile[]
  mdnsServices: AdbMdnsService[]
}

interface AndroidWirelessActionResponse {
  ok: boolean
  output?: string
  status: AndroidWirelessStatus
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

export async function connectAndroidDevice(address: string, profileId?: string): Promise<string> {
  try {
    const data = await nodeApi<{ output?: string }>(
      inspectorAdminUrl(),
      '/api/android-inspector/connect',
      {
        method: 'POST',
        body: JSON.stringify({ address: address.trim(), profileId }),
      },
      12000,
    )
    return data.output ?? ''
  } catch (error) {
    throw inspectorError(error, '无线 ADB 连接失败')
  }
}

export async function getAndroidWirelessStatus(): Promise<AndroidWirelessStatus> {
  try {
    return await nodeApi<AndroidWirelessStatus>(
      inspectorAdminUrl(),
      '/api/android-inspector/wireless/status',
      {},
      15000,
    )
  } catch (error) {
    throw inspectorError(error, '读取无线 ADB 状态失败')
  }
}

export async function registerAndroidDevice(input: {
  deviceId: string
  displayName?: string
}): Promise<{ profile: AndroidDeviceProfile; status: AndroidWirelessStatus }> {
  try {
    return await nodeApi<{ profile: AndroidDeviceProfile; status: AndroidWirelessStatus }>(
      inspectorAdminUrl(),
      '/api/android-inspector/wireless/register',
      {
        method: 'POST',
        body: JSON.stringify({
          deviceId: input.deviceId,
          displayName: input.displayName?.trim() || undefined,
        }),
      },
      20000,
    )
  } catch (error) {
    throw inspectorError(error, '登记手机失败')
  }
}

export async function pairAndroidDevice(input: {
  pairingAddress: string
  pairingCode: string
  profileId?: string
}): Promise<AndroidWirelessActionResponse> {
  try {
    return await nodeApi<AndroidWirelessActionResponse>(
      inspectorAdminUrl(),
      '/api/android-inspector/wireless/pair',
      {
        method: 'POST',
        body: JSON.stringify({
          pairingAddress: input.pairingAddress.trim(),
          pairingCode: input.pairingCode.trim(),
          profileId: input.profileId,
        }),
      },
      35000,
    )
  } catch (error) {
    throw inspectorError(error, '无线 ADB 配对失败')
  }
}

export async function reconnectAndroidDevices(profileId?: string): Promise<AndroidWirelessStatus> {
  try {
    return await nodeApi<AndroidWirelessStatus>(
      inspectorAdminUrl(),
      '/api/android-inspector/wireless/reconnect',
      {
        method: 'POST',
        body: JSON.stringify({ profileId }),
      },
      22000,
    )
  } catch (error) {
    throw inspectorError(error, '自动重连无线 ADB 失败')
  }
}

export async function enableAndroidTcpIp(input: {
  deviceId: string
  profileId?: string
  port?: number
}): Promise<AndroidWirelessActionResponse> {
  try {
    return await nodeApi<AndroidWirelessActionResponse>(
      inspectorAdminUrl(),
      '/api/android-inspector/wireless/enable-tcpip',
      {
        method: 'POST',
        body: JSON.stringify(input),
      },
      30000,
    )
  } catch (error) {
    throw inspectorError(error, '启用传统无线 ADB 失败')
  }
}

export async function forgetAndroidDevice(profileId: string): Promise<void> {
  try {
    await nodeApi(
      inspectorAdminUrl(),
      '/api/android-inspector/wireless/forget',
      {
        method: 'POST',
        body: JSON.stringify({ profileId }),
      },
      10000,
    )
  } catch (error) {
    throw inspectorError(error, '移除手机档案失败')
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

export async function persistAndroidSelectionArtifact(input: {
  snapshotId: string
  selectionId: string
  cropDataUrl: string
  bounds: AndroidInspectorBounds
  resourceId?: string
  componentKey?: string
}): Promise<AndroidSelectionArtifact> {
  try {
    const response = await nodeApi<{ artifact: AndroidSelectionArtifact }>(
      inspectorAdminUrl(),
      '/api/android-inspector/selection-artifact',
      {
        method: 'POST',
        body: JSON.stringify(input),
      },
      12000,
    )
    return response.artifact
  } catch (error) {
    throw inspectorError(error, '保存选中元素上下文失败')
  }
}
