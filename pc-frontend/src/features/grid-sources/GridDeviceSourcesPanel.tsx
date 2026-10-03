import { useEffect, useRef, useState } from 'react'
import { getAuthToken } from '../../api/client'
import { deviceSources, setSyncEnabled, syncEnabled } from './gridDeviceApi'
import { fresh, type DeviceSource, type Platform } from './gridDeviceModel'
import styles from './GridDeviceSourcesPanel.module.css'

export default function GridDeviceSourcesPanel({ ownerKey }: { ownerKey: string }) {
  const preferenceKey = 'grid-device-sources.v1.view.' + ownerKey
  const [selected, select] = useState<Platform[]>(() => {
    try {
      const saved = localStorage.getItem(preferenceKey)
      if (saved !== null) return JSON.parse(saved).filter((p: unknown) => p === 'android' || p === 'windows')
    } catch { /* Invalid view preferences do not affect authority or private data. */ }
    return ['android', 'windows']
  })
  const [sources, show] = useState<DeviceSource[]>([])
  const [enabled, enable] = useState(() => syncEnabled(ownerKey))
  const [message, notify] = useState('勾选要查看的来源，再点击刷新。')
  const [busy, wait] = useState(false)
  const [page, setPage] = useState(0)
  const [now, setNow] = useState(Date.now())
  const epoch = useRef(0), flight = useRef(false)
  const owner = useRef(ownerKey); owner.current = ownerKey
  async function read(upload: boolean | null = enabled ? true : null) {
    if (flight.current) return
    flight.current = true; wait(true); show([]); notify('正在读取本人设备…')
    const ticket = ++epoch.current, capturedOwner = ownerKey, token = getAuthToken()
    try {
      const result = await deviceSources(ownerKey, upload)
      if (ticket === epoch.current && capturedOwner === owner.current && token === getAuthToken()) {
        show(result.sources); setPage(0); setNow(Date.now()); notify(result.warning || '快照最多有效 5 分钟；按设备展示，不重复合计收益。')
      }
    } catch (error) {
      if (ticket === epoch.current) notify(error instanceof Error ? error.message : '读取未成功，请重试')
    } finally { flight.current = false; wait(false) }
  }
  useEffect(() => {
    epoch.current++; show([]); enable(syncEnabled(ownerKey)); wait(false)
    const clear = () => { if (document.hidden) { epoch.current++; show([]); wait(false) } }
    document.addEventListener('visibilitychange', clear)
    return () => { epoch.current++; document.removeEventListener('visibilitychange', clear) }
  }, [ownerKey])
  useEffect(() => {
    const deadline = sources.map(s => s.snapshot).filter(s => fresh(s, Date.now())).map(s => s.fresh_until_ms).sort((a, b) => a - b)[0]
    if (!deadline) return
    const timer = window.setTimeout(() => setNow(Date.now()), Math.max(1, deadline - Date.now()))
    return () => window.clearTimeout(timer)
  }, [sources, now])
  const visible = sources.filter(s => selected.includes(s.snapshot.platform))
  const rows = visible.flatMap(source => fresh(source.snapshot, now) ? source.snapshot.rows.map(row => ({ source, row })) : [])
  return <section className={styles.panel} aria-label="多端网格">
    <h3>多端网格</h3>
    <p>APK 和 Win 可以同时勾选。只同步网格数据，币安登录资料留在来源设备。</p>
    <div className={styles.controls}>
      {(['android', 'windows'] as const).map(platform => <label key={platform}>
        <input type="checkbox" checked={selected.includes(platform)} onChange={event => {
          const next = event.target.checked ? [...selected, platform] : selected.filter(p => p !== platform)
          localStorage.setItem(preferenceKey, JSON.stringify(next)); select(next); setPage(0)
          epoch.current++; show(old => old.filter(s => next.includes(s.snapshot.platform)))
        }} />{platform === 'android' ? 'APK 端' : 'Win 端'}
      </label>)}
      <button type="button" disabled={busy || !selected.length} onClick={() => void read()}>{busy ? '读取中…' : '刷新所选来源'}</button>
      {!enabled && <button type="button" disabled={busy} onClick={() => void read(false)}>重试清除远端 Win 副本</button>}
    </div>
    <label className={styles.sync}><input type="checkbox" checked={enabled} disabled={busy} onChange={event => {
      const value = event.target.checked; setSyncEnabled(ownerKey, value); enable(value); void read(value)
    }} />允许将本机 Win 网格同步给本人设备</label>
    <small>打开币安“运行中”列表后，在此点击刷新同步。关闭同步会请求清除远端副本；失败时最多 5 分钟后过期。</small>
    <p role="status">{message}</p>
    {!selected.length && <p>请选择至少一个来源。</p>}
    {selected.map(platform => !visible.some(s => s.snapshot.platform === platform) &&
      <p key={platform}>{platform === 'android' ? 'APK 端' : 'Win 端'}：尚无同步数据，请在来源端登录同一一龙账号并开启同步。</p>)}
    {visible.map(source => <p key={source.source_id} className={styles.source}>
      {source.snapshot.platform === 'android' ? 'APK' : 'Win'} · 设备 {source.snapshot.device_id.slice(0, 8)} · 账号 {source.snapshot.account?.slice(0, 8) ?? '未确认'} ·{' '}
      {fresh(source.snapshot, now) ? (source.snapshot.rows.length ? '可读取' : '当前无运行网格') : '未连接／已过期'} · {new Date(source.snapshot.observed_at_ms).toLocaleTimeString()}
    </p>)}
    <ul className={styles.rows}>{rows.slice(page * 30, page * 30 + 30).map(({ source, row }) => <li key={source.source_id + ':' + source.snapshot.account + ':' + row.id}>
      <strong>{String(row.symbol)} · {source.snapshot.platform === 'android' ? 'APK' : 'Win'} · #{String(row.id)}</strong>
      <span>{String(row.direction ?? '方向未读取')} · {String(row.status)} · {String(row.count ?? '—')} 格 · {String(row.leverage ?? '—')}×</span>
      <span>区间 {String(row.lower ?? '—')}–{String(row.upper ?? '—')} · 收益 {String(row.profit ?? '未读取')} USDT</span>
    </li>)}</ul>
    {rows.length > 30 && <nav className={styles.controls} aria-label="网格分页">
      <button type="button" disabled={page === 0} onClick={() => setPage(page - 1)}>上一页</button>
      <span>{page + 1} / {Math.ceil(rows.length / 30)}</span>
      <button type="button" disabled={(page + 1) * 30 >= rows.length} onClick={() => setPage(page + 1)}>下一页</button>
    </nav>}
  </section>
}
