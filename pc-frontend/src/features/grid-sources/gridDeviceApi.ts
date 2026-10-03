import { v4 as uuid } from 'uuid'
import { getDesktopInvoke } from '../shell/desktopShell'
import { getAuthToken } from '../../api/client'
import { getExchangeWebObservation } from '../exchange-webview/exchangeWebviewApi'
import { accountDigest, deviceSnapshot, parseDeviceSources, type DeviceSource } from './gridDeviceModel'

const key = 'grid-device-sources.v1'
export function syncEnabled(owner: string) { return localStorage.getItem(key + '.enabled.' + owner) === 'true' }
export function setSyncEnabled(owner: string, enabled: boolean) { localStorage.setItem(key + '.enabled.' + owner, String(enabled)) }
export async function deviceSources(owner: string, upload: boolean | null) {
  const invoke = getDesktopInvoke(), token = getAuthToken()
  if (!invoke || !token) throw Error('请在登录本人一龙账号的 Win 客户端使用')
  const active = () => getAuthToken() === token
  const warnings: string[] = []
  const stored = localStorage.getItem(key + '.device')
  const device = stored || uuid()
  if (!stored) localStorage.setItem(key + '.device', device)
  const observation = await getExchangeWebObservation('binance', owner).catch(() => {
    warnings.push('本机 Win 来源未就绪'); return null
  })
  if (!active()) throw Error('一龙账号已变化')
  let local: DeviceSource | null = null
  try {
    const snapshot = deviceSnapshot(observation, device, 1)
    if (snapshot.status === 'fresh') local = { source_id: accountDigest('local:' + device), snapshot }
  } catch { warnings.push('本机 Win 网格尚未确认') }
  if (upload !== null) {
    try {
    const prior = Number(localStorage.getItem(key + '.sequence') || 0)
    if (!Number.isSafeInteger(prior) || prior < 0 || prior >= Number.MAX_SAFE_INTEGER) throw Error('设备同步版本无效')
    const sequence = prior + 1
    localStorage.setItem(key + '.sequence', String(sequence))
    if (!active()) throw Error('一龙账号已变化')
    const body = deviceSnapshot(upload ? observation : null, device, sequence)
    const raw = await invoke<string>('grid_device_sources_request', { token, snapshot: JSON.stringify(body) })
    const reply = JSON.parse(raw)
    if (!active() || reply.schema !== 'yilong.grid_device_sources.ack.v1' || reply.sequence !== sequence
      || !['accepted', 'unchanged'].includes(reply.status)) throw Error('同步未确认')
    } catch { warnings.push('本机上传／清除未确认，可重试；不影响其他来源读取') }
  }
  let remote: DeviceSource[] = []
  try {
    const raw = await invoke<string>('grid_device_sources_request', { token, snapshot: null })
    remote = parseDeviceSources(raw)
  } catch { warnings.push('远端读取失败，本机已有网格仍可查看') }
  if (!active()) throw Error('一龙账号已变化')
  return { sources: [...remote.filter(s => !(local && s.snapshot.platform === 'windows' && s.snapshot.device_id === device)),
    ...(local ? [local] : [])], warning: warnings.join('；') }
}
