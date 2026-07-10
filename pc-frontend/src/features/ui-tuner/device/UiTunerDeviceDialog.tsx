import { useMemo, useState } from 'react'
import {
  Cable,
  Link2,
  RefreshCw,
  ShieldCheck,
  Smartphone,
  Trash2,
  Wifi,
  X,
} from 'lucide-react'
import type {
  AndroidDeviceProfile,
  AndroidInspectorDevice,
  AndroidWirelessStatus,
} from './deviceInspectorApi'
import styles from './UiTunerDeviceDialog.module.css'

interface UiTunerDeviceDialogProps {
  open: boolean
  busy: boolean
  status: AndroidWirelessStatus | null
  devices: AndroidInspectorDevice[]
  selectedDeviceId: string
  onClose: () => void
  onSelectDevice: (deviceId: string) => void
  onRefresh: () => void
  onRegister: (deviceId: string, displayName?: string) => Promise<AndroidDeviceProfile | null>
  onPair: (input: {
    pairingAddress: string
    pairingCode: string
    profileId?: string
  }) => Promise<boolean>
  onReconnect: (profileId?: string) => void
  onEnableLegacy: (deviceId: string, profileId?: string) => Promise<boolean>
  onConnectAddress: (address: string, profileId?: string) => Promise<boolean>
  onForget: (profileId: string) => void
}

const STATE_LABELS: Record<AndroidDeviceProfile['connectionState'], string> = {
  connected_usb: 'USB 已连接',
  connected_wireless: '无线已连接',
  paired_offline: '已配对 · 当前离线',
  offline: '未配置无线连接',
}

export function UiTunerDeviceDialog({
  open,
  busy,
  status,
  devices,
  selectedDeviceId,
  onClose,
  onSelectDevice,
  onRefresh,
  onRegister,
  onPair,
  onReconnect,
  onEnableLegacy,
  onConnectAddress,
  onForget,
}: UiTunerDeviceDialogProps) {
  const [displayName, setDisplayName] = useState('')
  const [pairingAddress, setPairingAddress] = useState('')
  const [pairingCode, setPairingCode] = useState('')
  const [manualAddress, setManualAddress] = useState('')
  const [activeProfileId, setActiveProfileId] = useState('')

  const selectedDevice = devices.find((device) => device.serial === selectedDeviceId) ?? devices[0]
  const activeProfile = useMemo(() => {
    const profiles = status?.profiles ?? []
    return profiles.find((profile) => profile.id === activeProfileId)
      ?? profiles.find((profile) => profile.connectedDeviceId === selectedDevice?.serial)
      ?? profiles[0]
      ?? null
  }, [activeProfileId, selectedDevice?.serial, status?.profiles])
  const unauthorizedDevice = devices.find((device) => device.state === 'unauthorized')
  const readyUsbDevice = devices.find((device) => (
    device.state === 'device' && !device.serial.includes(':')
  ))
  const selectedReadyDevice = selectedDevice?.state === 'device' ? selectedDevice : readyUsbDevice

  if (!open) return null

  return (
    <div className={styles.backdrop} role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget) onClose()
    }}>
      <section className={styles.dialog} role="dialog" aria-modal="true" aria-labelledby="wireless-adb-title">
        <header className={styles.header}>
          <div>
            <span className={styles.eyebrow}><Wifi size={14} /> 真机连接中心</span>
            <h2 id="wireless-adb-title">无线 ADB</h2>
            <p>首次登记并配对一次，以后打开微调画布会自动发现并重连同一台手机。</p>
          </div>
          <button className={styles.iconButton} type="button" onClick={onClose} aria-label="关闭">
            <X size={18} />
          </button>
        </header>

        <div className={styles.statusBar}>
          <span className={status?.adb.available ? styles.okDot : styles.errorDot} />
          <strong>{status?.adb.available ? 'ADB 已就绪' : 'ADB 不可用'}</strong>
          <span>{status?.adb.version ?? status?.adb.error ?? '正在读取本机节点状态'}</span>
          <button type="button" onClick={onRefresh} disabled={busy}>
            <RefreshCw size={14} className={busy ? styles.spinning : ''} />
            刷新
          </button>
        </div>

        {unauthorizedDevice && (
          <div className={styles.warning}>
            <ShieldCheck size={18} />
            <div>
              <strong>手机正在等待 USB 调试授权</strong>
              <span>请解锁手机，在弹窗中勾选“始终允许这台电脑”，然后点击刷新。</span>
            </div>
          </div>
        )}

        <div className={styles.content}>
          <section className={styles.section}>
            <div className={styles.sectionTitle}>
              <span><Cable size={16} /> 第一步 · 登记有线手机</span>
              <small>设备身份按硬件序列号保存，不依赖会变化的 IP。</small>
            </div>
            <div className={styles.inlineFields}>
              <select value={selectedDevice?.serial ?? ''} onChange={(event) => onSelectDevice(event.currentTarget.value)}>
                <option value="">选择 ADB 设备</option>
                {devices.map((device) => (
                  <option key={device.serial} value={device.serial}>
                    {device.model ?? device.serial} · {device.state}
                  </option>
                ))}
              </select>
              <input
                value={displayName}
                onChange={(event) => setDisplayName(event.currentTarget.value)}
                placeholder="设备名称（可选）"
              />
              <button
                type="button"
                disabled={busy || !selectedReadyDevice}
                onClick={async () => {
                  const profile = await onRegister(selectedReadyDevice?.serial ?? '', displayName)
                  if (profile) setActiveProfileId(profile.id)
                }}
              >
                <Smartphone size={14} /> 记住这台手机
              </button>
            </div>
          </section>

          <section className={styles.section}>
            <div className={styles.sectionTitle}>
              <span><Wifi size={16} /> 第二步 · Android 11+ 安全配对</span>
              <small>手机：开发者选项 → 无线调试 → 使用配对码配对设备。</small>
            </div>
            <div className={styles.pairGrid}>
              <label>
                <span>手机显示的配对地址</span>
                <input
                  value={pairingAddress}
                  onChange={(event) => setPairingAddress(event.currentTarget.value)}
                  placeholder="例如 192.168.1.8:37123"
                />
              </label>
              <label>
                <span>六位配对码</span>
                <input
                  inputMode="numeric"
                  maxLength={6}
                  value={pairingCode}
                  onChange={(event) => setPairingCode(event.currentTarget.value.replace(/\D/g, '').slice(0, 6))}
                  placeholder="123456"
                />
              </label>
              <button
                type="button"
                className={styles.primaryButton}
                disabled={busy || !activeProfile || !pairingAddress.trim() || pairingCode.length !== 6}
                onClick={async () => {
                  const ok = await onPair({
                    pairingAddress,
                    pairingCode,
                    profileId: activeProfile?.id,
                  })
                  if (ok) setPairingCode('')
                }}
              >
                <ShieldCheck size={15} /> 安全配对并连接
              </button>
            </div>
          </section>

          <section className={styles.section}>
            <div className={styles.sectionTitle}>
              <span><Link2 size={16} /> 兼容与手动连接</span>
              <small>传统 5555 模式适合 Android 10 以下；重启手机后可能失效。</small>
            </div>
            <div className={styles.compatActions}>
              <button
                type="button"
                disabled={busy || !selectedReadyDevice}
                onClick={() => onEnableLegacy(selectedReadyDevice?.serial ?? '', activeProfile?.id)}
              >
                USB 一键转无线 5555
              </button>
              <input
                value={manualAddress}
                onChange={(event) => setManualAddress(event.currentTarget.value)}
                placeholder="连接地址 IP:端口"
              />
              <button
                type="button"
                disabled={busy || !manualAddress.trim()}
                onClick={() => onConnectAddress(manualAddress, activeProfile?.id)}
              >
                手动连接
              </button>
            </div>
          </section>

          <section className={styles.section}>
            <div className={styles.sectionTitle}>
              <span><Smartphone size={16} /> 已记住的手机</span>
              <small>{status?.profiles.length ?? 0} 台</small>
            </div>
            <div className={styles.profileList}>
              {!status?.profiles.length && <div className={styles.empty}>尚未登记手机，请先完成第一步。</div>}
              {status?.profiles.map((profile) => (
                <article
                  key={profile.id}
                  className={profile.id === activeProfile?.id ? styles.activeProfile : styles.profileCard}
                  onClick={() => setActiveProfileId(profile.id)}
                >
                  <div>
                    <strong>{profile.displayName}</strong>
                    <span>{profile.manufacturer} {profile.model} · Android {profile.androidRelease ?? '?'}</span>
                    <small>{profile.lastEndpoint ?? '等待首次无线连接'}</small>
                  </div>
                  <div className={styles.profileActions}>
                    <span data-state={profile.connectionState}>{STATE_LABELS[profile.connectionState]}</span>
                    <button type="button" disabled={busy} onClick={(event) => {
                      event.stopPropagation()
                      onReconnect(profile.id)
                    }}>
                      重连
                    </button>
                    <button className={styles.dangerButton} type="button" disabled={busy} onClick={(event) => {
                      event.stopPropagation()
                      if (window.confirm(`移除 ${profile.displayName} 的本机连接档案？`)) onForget(profile.id)
                    }} aria-label={`移除 ${profile.displayName}`}>
                      <Trash2 size={14} />
                    </button>
                  </div>
                </article>
              ))}
            </div>
          </section>
        </div>
      </section>
    </div>
  )
}
