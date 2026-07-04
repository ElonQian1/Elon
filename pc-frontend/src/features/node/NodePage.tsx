import { useState, useEffect, useRef, useCallback } from 'react'
import { useSearchParams } from 'react-router-dom'
import { Settings } from 'lucide-react'
import { nodeApi, probeLocalNode } from './localNodeApi'
import { fetchMyNodes, fetchNodeAgentVersion, nodeId, nodeName, nodeSummaryLine } from './nodeHelpers'
import { launchWinClientProtocol, WIN_CLIENT_DOWNLOAD_URL } from './launchWinClient'
import CodexVaultCard from './CodexVaultCard'
import RuntimeRouteConfigGuide, { isRouteConfigKey } from './RuntimeRouteConfigGuide'
import { useUserProgression } from '../billing/useUserProgression'
import type { UserProgressionSummary } from '../billing/progressionApi'
import { safeNodeAdminUrl } from '../../lib/utils'
import { useAuthStore } from '../../store/auth'
import type {
  AutostartStatus,
  CodexVaultStatusResponse,
  LocalCliToolStatus,
  LocalNodeStatus,
  NodeSummary,
} from './types'
import styles from './NodePage.module.css'

export default function NodePage() {
  const [nodes, setNodes] = useState<NodeSummary[]>([])
  const [selectedNodeId, setSelectedNodeId] = useState('')
  const [searchParams] = useSearchParams()
  const adminUrl = safeNodeAdminUrl()
  const routeConfig = searchParams.get('route')
  const routeConfigKey = isRouteConfigKey(routeConfig) ? routeConfig : null

  useEffect(() => {
    fetchMyNodes().then(setNodes).catch(() => {})
  }, [])

  const selected = nodes.find((n) => nodeId(n) === selectedNodeId)

  return (
    <div className={styles.layout}>
      <aside className={styles.sidebar}>
        <div className={styles.sideSection}>本机</div>
        <button
          className={[styles.sideBtn, !selectedNodeId ? styles.sideActive : ''].join(' ')}
          onClick={() => setSelectedNodeId('')}
        >
          <span className={styles.sideIcon}>🖥️</span>
          <span className={styles.sideMeta}>
            <strong>分享算力</strong>
            <small>下载、启动和注册</small>
          </span>
        </button>

        <div className={styles.sideSection}>我的节点</div>
        {nodes.length === 0 && <p className={styles.sideEmpty}>暂无节点</p>}
        {nodes.map((n) => {
          const id = nodeId(n)
          return (
            <button
              key={id}
              className={[styles.sideBtn, id === selectedNodeId ? styles.sideActive : ''].join(' ')}
              onClick={() => setSelectedNodeId(id)}
            >
              <span className={styles.sideIcon}>{n.online ? '●' : '○'}</span>
              <span className={styles.sideMeta}>
                <strong>{nodeName(n)}</strong>
                <small>{nodeSummaryLine(n)}</small>
              </span>
            </button>
          )
        })}

        <div className={styles.sideSection}>状态</div>
        <div className={styles.sideNote}>
          {nodes.filter((n) => n.online).length}/{nodes.length} 台在线
        </div>
      </aside>

      <main className={styles.main}>
        {routeConfigKey && <RuntimeRouteConfigGuide route={routeConfigKey} />}
        {!selectedNodeId
          ? <LocalNodePanel adminUrl={adminUrl} />
          : selected
            ? <NodeDetailPanel node={selected} onBack={() => setSelectedNodeId('')} adminUrl={adminUrl} />
            : <p className={styles.notFound}>节点不存在</p>
        }
      </main>
    </div>
  )
}

/* ── 本机面板 ── */
function LocalNodePanel({ adminUrl }: { adminUrl: string }) {
  const [probeStatus, setProbeStatus] = useState<'checking' | 'online' | 'offline'>('checking')
  const [localStatus, setLocalStatus] = useState<LocalNodeStatus | null>(null)
  const [version, setVersion] = useState('')
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null)

  const doProbe = useCallback(async (quiet = false) => {
    if (!quiet) setProbeStatus('checking')
    try {
      const status = await probeLocalNode(adminUrl) as LocalNodeStatus
      setLocalStatus(status)
      setProbeStatus('online')
      if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null }
    } catch {
      setProbeStatus('offline')
    }
  }, [adminUrl])

  useEffect(() => {
    doProbe()
    fetchNodeAgentVersion()
      .then((d) => {
        const size = d.windowsClientFileSize
          ? ` · ${(d.windowsClientFileSize / 1024 / 1024).toFixed(1)} MB` : ''
        setVersion(`最新 Win 端：v${d.version ?? 'latest'}${size}`)
      })
      .catch(() => setVersion('Win 端下载包暂时无法读取版本。'))
    return () => { if (pollRef.current) clearInterval(pollRef.current) }
  }, [doProbe])

  function launchWinClient() {
    launchWinClientProtocol()
    if (!pollRef.current) {
      let attempts = 0
      pollRef.current = setInterval(() => {
        attempts++
        if (attempts > 24) { clearInterval(pollRef.current!); pollRef.current = null; return }
        doProbe(true)
      }, 3500)
    }
  }

  return (
    <div className={styles.localPage}>
      <div className={styles.hero}>
        <div>
          <div className={styles.kicker}>一龙 Win 端</div>
          <h2>分享这台电脑算力</h2>
          <p>首次使用需要下载并安装；安装后点击"启动 Win 端"，浏览器会拉起本机程序。</p>
        </div>
        <span className={[styles.chip, styles[probeStatus]].join(' ')}>
          {{ checking: '检测中', online: '已连接', offline: '未连接' }[probeStatus]}
        </span>
      </div>

      <div className={styles.actions}>
        <a className={[styles.btn, styles.primary].join(' ')} href={WIN_CLIENT_DOWNLOAD_URL} download>下载 Win 端</a>
        <button className={styles.btn} onClick={launchWinClient}>
          {probeStatus === 'online' ? '打开高级本机页' : '启动 Win 端'}
        </button>
        <button className={styles.btn} onClick={() => doProbe()}>重新检测</button>
      </div>

      {version && <p className={styles.versionLine}>{version}</p>}

      {probeStatus === 'online' && localStatus
        ? <NodeAdminPanel adminUrl={adminUrl} initialStatus={localStatus} />
        : probeStatus === 'offline' && (
          <div className={styles.setupCard}>
            <h3>还没有可用的本机节点</h3>
            <div className={styles.stepList}>
              <div><strong>1</strong><span>下载 Win 端压缩包并解压。</span></div>
              <div><strong>2</strong><span>双击「一龙开发平台.exe」，它会自动安装并注册网页一键唤起。</span></div>
              <div><strong>3</strong><span>安装后点击"启动 Win 端"，在本机页面登录并注册 PC 节点。</span></div>
            </div>
          </div>
        )}
    </div>
  )
}

/* ── 本机已连接时的管理面板（精简版） ── */
function NodeAdminPanel({ adminUrl, initialStatus }: { adminUrl: string; initialStatus: LocalNodeStatus }) {
  const token = useAuthStore((s) => s.token)
  const user = useAuthStore((s) => s.user)
  const progression = useUserProgression(user?.id, token)
  const [status, setStatus] = useState(initialStatus)
  const [autostart, setAutostart] = useState<AutostartStatus | null>(null)
  const [autostartBusy, setAutostartBusy] = useState(false)
  const [result, setResult] = useState('')
  const [error, setError] = useState('')
  const [codexBusy, setCodexBusy] = useState(false)
  const [vaultBusy, setVaultBusy] = useState(false)
  const [vaultStatus, setVaultStatus] = useState<CodexVaultStatusResponse | null>(null)
  const [apiKey, setApiKey] = useState('')
  const [apiModel, setApiModel] = useState('gpt-5')
  const cliNames = [
    ...(status.allowed_clis ?? []),
    ...(status.cli_tools ?? [])
      .filter((item) => item.available !== false)
      .map((item) => item.name ?? item.label ?? ''),
  ].filter(Boolean)
  const uniqueCliNames = Array.from(new Set(cliNames.map((item) => String(item).trim()).filter(Boolean)))
  const localModelCount = status.local_ai?.models?.length ?? status.models?.length ?? 0
  const codex = codexStatusFrom(status)
  const codexVault = vaultStatus?.local ?? status.codex_vault ?? null

  const refreshStatus = useCallback(async (quiet = false) => {
    if (!quiet) { setResult('刷新中…'); setError('') }
    try {
      const data = await nodeApi<LocalNodeStatus>(adminUrl, '/api/status')
      setStatus(data)
      if (!quiet) setResult('本机状态已刷新。')
    } catch (err) {
      if (!quiet) setError((err as Error).message)
    }
  }, [adminUrl])

  const loadCodexVaultStatus = useCallback(async (quiet = true) => {
    if (!quiet) { setResult('读取 Codex 保险箱状态…'); setError('') }
    try {
      const data = await nodeApi<CodexVaultStatusResponse>(adminUrl, '/api/codex-vault/status', {}, 12000)
      setVaultStatus(data)
      if (!quiet) setResult('Codex 保险箱状态已刷新。')
    } catch (err) {
      const message = (err as Error).message
      setVaultStatus((prev) => ({
        ...(prev ?? {}),
        ok: false,
        cloud: { ok: false, error: message },
        error: message,
      }))
      if (!quiet) setError(message)
    }
  }, [adminUrl])

  const loadAutostart = useCallback(async () => {
    try {
      const data = await nodeApi<AutostartStatus>(adminUrl, '/api/client-maintenance/autostart')
      setAutostart(data)
    } catch (err) {
      setAutostart({ supported: false, enabled: false, summary: (err as Error).message })
    }
  }, [adminUrl])

  useEffect(() => {
    loadAutostart()
  }, [loadAutostart])

  useEffect(() => {
    loadCodexVaultStatus()
  }, [loadCodexVaultStatus])

  useEffect(() => {
    if (!status.cli_probe?.refreshing && codex?.status !== 'checking') return
    const timer = setTimeout(() => { refreshStatus(true) }, 1600)
    return () => clearTimeout(timer)
  }, [status.cli_probe?.refreshing, codex?.status, refreshStatus])

  async function login() {
    setResult('绑定中…'); setError('')
    const userToken = String(token ?? '').trim()
    if (!userToken) {
      setResult('')
      setError('请先在 PC 工作台登录一龙账号，再绑定本机节点。')
      return
    }
    try {
      await nodeApi(adminUrl, '/api/login', { method: 'POST', body: JSON.stringify({ token: userToken }) })
      setResult('本机节点已绑定当前账号。')
      await refreshStatus(true)
      await loadCodexVaultStatus()
    } catch (err) { setError((err as Error).message) }
  }

  async function logout() {
    setResult('登出中…'); setError('')
    try {
      await nodeApi(adminUrl, '/api/logout', { method: 'POST' })
      setResult('本机节点已登出。')
      await refreshStatus(true)
      setVaultStatus(null)
    } catch (err) { setError((err as Error).message) }
  }

  async function refreshCodex() {
    setCodexBusy(true); setResult('正在检测 Codex…'); setError('')
    try {
      await nodeApi(adminUrl, '/api/codex-cli/refresh', { method: 'POST' }, 5000)
      await refreshStatus(true)
      setResult('Codex 检测已完成。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setCodexBusy(false)
    }
  }

  async function backupCodexVault() {
    setVaultBusy(true); setResult('正在备份本机 Codex Pro 凭据…'); setError('')
    try {
      const data = await nodeApi<CodexVaultStatusResponse>(
        adminUrl,
        '/api/codex-vault/backup',
        { method: 'POST', body: JSON.stringify({}) },
        30000,
      )
      setVaultStatus(data)
      setResult(data.message || 'Codex Pro 凭据已加密备份到云端保险箱。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setVaultBusy(false)
    }
  }

  async function restoreCodexVault() {
    setVaultBusy(true); setCodexBusy(true); setResult('正在恢复临时 Codex Pro 登录态…'); setError('')
    try {
      const data = await nodeApi<CodexVaultStatusResponse>(
        adminUrl,
        '/api/codex-vault/restore',
        { method: 'POST', body: JSON.stringify({ purpose: 'pc_web_temporary_codex_cli' }) },
        30000,
      )
      setVaultStatus(data)
      await refreshStatus(true)
      await loadCodexVaultStatus(true)
      setResult(data.message || '已恢复为本机临时 Codex Pro 会话。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setVaultBusy(false)
      setCodexBusy(false)
    }
  }

  async function clearCodexVault() {
    setVaultBusy(true); setCodexBusy(true); setResult('正在清理本机临时 Codex 登录态…'); setError('')
    try {
      const data = await nodeApi<CodexVaultStatusResponse>(
        adminUrl,
        '/api/codex-vault/clear',
        { method: 'POST', body: JSON.stringify({}) },
        20000,
      )
      setVaultStatus(data)
      await refreshStatus(true)
      await loadCodexVaultStatus(true)
      setResult(data.message || '已清理本机保险箱临时 CODEX_HOME。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setVaultBusy(false)
      setCodexBusy(false)
    }
  }

  async function deleteCloudCodexVault() {
    setVaultBusy(true); setResult('正在删除云端 Codex Pro 保险箱备份…'); setError('')
    try {
      const data = await nodeApi<CodexVaultStatusResponse>(
        adminUrl,
        '/api/codex-vault/delete-cloud',
        { method: 'POST', body: JSON.stringify({}) },
        20000,
      )
      setVaultStatus(data)
      setResult(data.message || '已删除云端 Codex Pro 保险箱备份。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setVaultBusy(false)
    }
  }

  async function installEnv() {
    setCodexBusy(true); setResult('正在启动安装修复向导…'); setError('')
    try {
      const data = await nodeApi<{ msg?: string; message?: string }>(
        adminUrl, '/api/install-env', { method: 'POST' }, 10000,
      )
      setResult(data.msg || data.message || '安装修复向导已启动。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setCodexBusy(false)
    }
  }

  async function saveCodexKey() {
    const key = apiKey.trim()
    if (!key) {
      setError('请先填写 OpenAI API Key。')
      return
    }
    setCodexBusy(true); setResult('正在保存 Codex 鉴权…'); setError('')
    try {
      const data = await nodeApi<{ msg?: string; message?: string }>(
        adminUrl,
        '/api/save-openai-key',
        { method: 'POST', body: JSON.stringify({ api_key: key, model: apiModel.trim() || null }) },
        10000,
      )
      setApiKey('')
      await refreshCodex()
      setResult(data.msg || data.message || 'Codex 鉴权已保存。')
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setCodexBusy(false)
    }
  }

  async function toggleAutostart() {
    const nextEnabled = !(autostart?.enabled)
    setAutostartBusy(true); setResult(''); setError('')
    try {
      const data = await nodeApi<AutostartStatus>(adminUrl, '/api/client-maintenance/autostart', {
        method: 'POST',
        body: JSON.stringify({ enabled: nextEnabled }),
      })
      setAutostart(data)
      setResult(data.message || (nextEnabled ? '已开启开机自启动。' : '已关闭开机自启动。'))
    } catch (err) {
      setError((err as Error).message)
    } finally {
      setAutostartBusy(false)
    }
  }

  return (
    <div className={styles.adminPanel}>
      <div className={styles.adminRow}>
        <div>
          <div className={styles.kicker}>本机节点</div>
          <h3>{status.device_name ?? '这台电脑'}</h3>
        </div>
        <span className={[styles.chip, status.connected ? styles.online : styles.checking].join(' ')}>
          {status.connected ? '云端在线' : '等待云端'}
        </span>
      </div>
      <div className={styles.kvGrid}>
        {[
          ['登录', status.logged_in ? '已登录' : '未登录'],
          ['节点 ID', status.agent_id ?? '登录后自动生成'],
          ['版本', status.version ?? '未知'],
          ['开机自启动', autostart?.enabled ? '已开启' : autostart ? '未开启' : '检测中'],
          ['本机AI', uniqueCliNames.length ? uniqueCliNames.join('、') : '未检测到 Codex/Copilot'],
          ['本机模型', localModelCount ? `${localModelCount} 个` : '未检测到'],
        ].map(([k, v]) => (
          <div key={k}><span>{k}</span><strong>{v}</strong></div>
        ))}
      </div>
      <CodexStatusCard
        status={codex}
        refreshing={!!status.cli_probe?.refreshing}
        busy={codexBusy}
        apiKey={apiKey}
        apiModel={apiModel}
        onApiKeyChange={setApiKey}
        onApiModelChange={setApiModel}
        onRefresh={refreshCodex}
        onInstall={installEnv}
        onSaveKey={saveCodexKey}
      />
      <CodexVaultCard
        status={codexVault}
        cloud={vaultStatus?.cloud}
        busy={vaultBusy}
        onBackup={backupCodexVault}
        onRestore={restoreCodexVault}
        onClear={clearCodexVault}
        onDeleteCloud={deleteCloudCodexVault}
        onRefresh={() => loadCodexVaultStatus(false)}
      />
      <ShareSettlementCard progression={progression} />
      <div className={styles.actions}>
        <button className={[styles.btn, styles.primary].join(' ')} onClick={login}>
          {status.logged_in ? '重新绑定当前账号' : '用当前账号注册节点'}
        </button>
        <button className={styles.btn} disabled={!status.logged_in} onClick={logout}>登出本机节点</button>
        <button
          className={[styles.btn, styles.iconBtn].join(' ')}
          disabled={autostartBusy || autostart?.supported === false}
          onClick={toggleAutostart}
          title="配置开机自启动"
        >
          <Settings size={15} strokeWidth={2.2} aria-hidden="true" />
          {autostart?.enabled ? '关闭开机自启动' : '开启开机自启动'}
        </button>
        <button className={styles.btn} onClick={() => window.open(adminUrl, '_blank', 'noopener')}>
          高级本机页
        </button>
        <button className={styles.btn} onClick={() => refreshStatus()} disabled={codexBusy}>
          刷新状态
        </button>
      </div>
      {autostart?.summary && <p className={styles.hintLine}>{autostart.summary}</p>}
      {result && <p className={styles.resultOk}>{result}</p>}
      {error && <p className={styles.resultErr}>{error}</p>}
    </div>
  )
}

function ShareSettlementCard({ progression }: { progression: UserProgressionSummary | null }) {
  const weeklyPoolCents = 4_615
  const fiveHoursCents = Math.round(weeklyPoolCents * 0.3)
  const hourCents = Math.round(fiveHoursCents / 5)
  const weekTokens = progression?.provider_week_tokens ?? 0
  const weekRuns = progression?.provider_week_run_count ?? 0
  const weekEarnedFen = progression?.provider_week_earned_fen ?? 0
  const lifetimeEarnedFen = progression?.provider_earned_fen ?? 0
  const weekLabel = weekRangeLabel(
    progression?.provider_week_start_at,
    progression?.provider_week_end_at,
  )

  return (
    <section className={styles.shareCard}>
      <div className={styles.shareHead}>
        <div>
          <span className={styles.codexLabel}>共享算力结算</span>
          <h4>别人使用你的节点</h4>
        </div>
        <span className={styles.vaultState}>{weekLabel}</span>
      </div>
      <div className={styles.shareGrid}>
        <div>
          <span>本周分享</span>
          <strong>{formatTokens(weekTokens)}</strong>
          <small>{weekRuns} 次运行</small>
        </div>
        <div>
          <span>本周实际收益</span>
          <strong>{formatFen(weekEarnedFen)}</strong>
          <small>来自已结算账本</small>
        </div>
        <div>
          <span>累计分享收益</span>
          <strong>{formatFen(lifetimeEarnedFen)}</strong>
          <small>可用于提现账本</small>
        </div>
        <div>
          <span>Pro 成本参考</span>
          <strong>{formatUsdCents(weeklyPoolCents)}/周</strong>
          <small>按 $200/月年化</small>
        </div>
      </div>
      <p className={styles.shareHint}>
        若约 5 小时等于每周 30% 容量，参考成本约 {formatUsdCents(fiveHoursCents)}，约 {formatUsdCents(hourCents)}/小时。
        这只是容量参考价；真实结算仍以平台账本和节点分账比例为准。
      </p>
    </section>
  )
}

function formatTokens(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0'
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 0 : 1)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(value >= 10_000 ? 0 : 1)}K`
  return `${Math.round(value)}`
}

function formatFen(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '¥0.00'
  return `¥${(value / 100).toFixed(2)}`
}

function formatUsdCents(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '$0.00'
  return `$${(value / 100).toFixed(2)}`
}

function weekRangeLabel(start?: string | null, end?: string | null) {
  const from = formatShortDate(start)
  const to = formatShortDate(end)
  return from && to ? `${from} - ${to}` : '本周'
}

function formatShortDate(value?: string | null) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return `${date.getMonth() + 1}/${date.getDate()}`
}

function codexStatusFrom(status: LocalNodeStatus): LocalCliToolStatus | null {
  return status.codex_cli
    ?? status.cli_tools?.find((item) => String(item.name ?? '').toLowerCase() === 'codex')
    ?? null
}

function codexStatusCopy(status: LocalCliToolStatus | null, refreshing: boolean) {
  if (refreshing || !status || status.status === 'checking') {
    return {
      tone: 'checking',
      title: 'Codex CLI 检测中',
      body: 'Win 端正在后台检测 Codex，不会阻塞页面或节点启动。',
      action: 'wait',
    }
  }
  if (status.status === 'ready' || status.available) {
    return {
      tone: 'online',
      title: 'Codex CLI 已就绪',
      body: status.detail || '本机 Codex 可由一龙 Win 端托管执行。',
      action: 'none',
    }
  }
  if (status.status === 'not_installed') {
    return {
      tone: 'offline',
      title: 'Codex CLI 未安装',
      body: status.detail || '需要安装 @openai/codex CLI。',
      action: 'install',
    }
  }
  if (status.status === 'not_runnable') {
    return {
      tone: 'offline',
      title: 'Codex CLI 不可运行',
      body: status.detail || '检测到 codex 命令，但 Win 端无法启动它。',
      action: 'repair_path',
    }
  }
  if (status.status === 'not_logged_in') {
    return {
      tone: 'checking',
      title: 'Codex CLI 未登录',
      body: status.detail || '请保存 API Key 或完成 Codex CLI 登录。',
      action: 'login',
    }
  }
  return {
    tone: 'checking',
    title: 'Codex CLI 状态未知',
    body: status.detail || '请重新检测。',
    action: 'refresh',
  }
}

function CodexStatusCard({
  status,
  refreshing,
  busy,
  apiKey,
  apiModel,
  onApiKeyChange,
  onApiModelChange,
  onRefresh,
  onInstall,
  onSaveKey,
}: {
  status: LocalCliToolStatus | null
  refreshing: boolean
  busy: boolean
  apiKey: string
  apiModel: string
  onApiKeyChange: (value: string) => void
  onApiModelChange: (value: string) => void
  onRefresh: () => void
  onInstall: () => void
  onSaveKey: () => void
}) {
  const copy = codexStatusCopy(status, refreshing)
  const showKeyForm = copy.action === 'login'
  const showInstall = copy.action === 'install' || copy.action === 'repair_path'
  return (
    <section className={[styles.codexCard, styles[`codex_${copy.tone}`]].join(' ')}>
      <div className={styles.codexHead}>
        <div>
          <span className={styles.codexLabel}>Codex</span>
          <h4>{copy.title}</h4>
        </div>
        <span className={styles.codexState}>{refreshing ? '刷新中' : status?.status ?? 'checking'}</span>
      </div>
      <p>{copy.body}</p>
      {status?.diagnosis && <p className={styles.codexDiagnosis}>{status.diagnosis}</p>}
      {status?.fix_hint && <p className={styles.codexFixHint}>{status.fix_hint}</p>}
      {status?.path && <code className={styles.codexPath}>{status.path}</code>}
      {(status?.version || status?.reason) && (
        <div className={styles.codexMeta}>
          {status.version && <span>版本：{status.version}</span>}
          {status.reason && <span>原因：{status.reason}</span>}
        </div>
      )}
      {showKeyForm && (
        <div className={styles.codexKeyGrid}>
          <input
            value={apiKey}
            onChange={(event) => onApiKeyChange(event.target.value)}
            placeholder="OpenAI API Key"
            type="password"
            autoComplete="off"
          />
          <input
            value={apiModel}
            onChange={(event) => onApiModelChange(event.target.value)}
            placeholder="模型"
            autoComplete="off"
          />
          <button className={[styles.btn, styles.primary].join(' ')} onClick={onSaveKey} disabled={busy}>
            保存鉴权
          </button>
        </div>
      )}
      <div className={styles.codexActions}>
        {showInstall && (
          <button className={[styles.btn, styles.primary].join(' ')} onClick={onInstall} disabled={busy}>
            安装/修复 Codex
          </button>
        )}
        <button className={styles.btn} onClick={onRefresh} disabled={busy}>
          重新检测
        </button>
      </div>
    </section>
  )
}

/* ── 远程节点详情 ── */
function NodeDetailPanel({ node, onBack, adminUrl: _adminUrl }: { node: NodeSummary; onBack: () => void; adminUrl: string }) {
  const hw = node.hardware ?? {}
  const runtime = node.dev_runtime ?? {}
  const warnings = [...(node.capacity_warnings ?? []), ...(runtime.issues ?? [])].filter(Boolean)
  const models = node.models ?? []

  const rows = [
    ['显示名称', nodeName(node)],
    ['节点 ID', nodeId(node) || '未知'],
    ['短 ID', node.short_id ?? '未知'],
    ['在线', node.online ? '是' : '否'],
    ['项目', `${node.project_count ?? 0}/${node.project_limit ?? '?'}`],
    ['系统', String(hw.os ?? '未知')],
    ['CPU', hw.cpu_brand ? `${hw.cpu_brand} · ${hw.cpu_cores ?? '?'} 核` : '未上报'],
    ['内存', hw.memory_total_bytes ? `${(hw.memory_total_bytes / 1024 ** 3).toFixed(1)} GB` : '未上报'],
    ['显卡', (hw.gpu_names ?? []).join('、') || '未上报'],
    ['工作区', runtime.workspace_root_path ?? '未配置'],
    ['Git', runtime.git_ready ? '可用' : '未就绪'],
    ['本机AI', runtime.route_a_ready ? '就绪' : '未就绪'],
    ['本机API key', runtime.api_runtime_ready ? '就绪' : '未就绪'],
    ['一龙开发环境', node.ai_cli_ready ? '就绪' : '未就绪'],
  ]

  return (
    <div className={styles.detailPage}>
      <div className={styles.hero}>
        <div>
          <div className={styles.kicker}>我的节点</div>
          <h2>{nodeName(node)}</h2>
          <p className={styles.nodeIdText}>{nodeId(node)}</p>
        </div>
        <span className={[styles.chip, node.online ? styles.online : styles.offline].join(' ')}>
          {node.online ? '在线' : '离线'}
        </span>
      </div>

      {warnings.length > 0 && (
        <div className={styles.warnings}>
          {warnings.map((w, i) => <div key={i}>{w}</div>)}
        </div>
      )}

      <div className={styles.kvGrid}>
        {rows.map(([k, v]) => <div key={k}><span>{k}</span><strong>{v}</strong></div>)}
      </div>

      {models.length > 0 && (
        <div className={styles.section}>
          <h4>模型能力</h4>
          <div className={styles.pills}>
            {models.map((m, i) => (
              <span key={i} className={styles.pill}>
                {m.display_name ?? m.model_id} {m.provider ? `· ${m.provider}` : ''}
              </span>
            ))}
          </div>
        </div>
      )}

      <button className={styles.btn} onClick={onBack}>← 回到分享算力</button>
    </div>
  )
}
