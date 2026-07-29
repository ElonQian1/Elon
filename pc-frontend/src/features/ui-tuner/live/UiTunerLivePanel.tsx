import { useEffect, useRef, useState } from 'react'
import type {
  DebugIntegrationStatus,
  LivePatchOperation,
  LivePreviewRequest,
  LiveBuildVerifyResult,
  LiveUiNode,
  LiveUiScope,
  LiveUiSession,
} from './liveUiApi'
import type { LiveUiConnectionState } from './useLiveUiSession'
import type { LiveSourceCommitPlan, LiveSourceCommitResult } from './liveUiCommitApi'
import type {
  LiveTargetDesign,
  LiveUiIrDocument,
} from './liveUiIrApi'
import type { LiveMcpDescriptor } from './liveUiApi'
import styles from './UiTunerLivePanel.module.css'
import { UiTunerPreviewPanel } from './UiTunerPreviewPanel'
import type { RuntimeDraftStatus } from './runtimeDraftModel'
import { UiCapabilityGapPanel } from '../capability-gap/UiCapabilityGapPanel'
import { lkgStatusLabel } from './debugPackage'

interface UiTunerLivePanelProps {
  state: LiveUiConnectionState
  error: string
  busy: boolean
  session: LiveUiSession | null
  node: LiveUiNode | null
  mcp: LiveMcpDescriptor | null
  uiIr: LiveUiIrDocument | null
  targetDesign: LiveTargetDesign | null
  draftStatus: RuntimeDraftStatus
  onApply: (operation: LivePatchOperation, scope: LiveUiScope) => Promise<unknown>
  onApplyGesture: (operations: LivePatchOperation[], gestureId: string) => Promise<unknown>
  onGestureActive: (active: boolean) => void
  onUndo: () => Promise<void>
  onRedo: () => Promise<void>
  onReconnect: () => void
  commitPlan: LiveSourceCommitPlan | null
  commitResult: LiveSourceCommitResult | null
  onPreviewCommit: () => Promise<LiveSourceCommitPlan>
  onCommit: (plan: LiveSourceCommitPlan) => Promise<LiveSourceCommitResult>
  onOpenPreview: (request: LivePreviewRequest) => Promise<void>
  buildVerifyResult: LiveBuildVerifyResult | null
  onBuildVerify: () => Promise<LiveBuildVerifyResult>
  prepareBusy: boolean
  prepareError: string
  debugIntegration: DebugIntegrationStatus | null
  lkgEnabled: boolean
  onLkgEnabledChange: (enabled: boolean) => void
  prepareReady: boolean
  debugPackage: string
  projectRoot: string
  onProjectRootChange: (value: string) => void
  onPrepareRuntime: () => void
}

const NUMBER_FIELDS = [
  ['width', '宽度', 'dp', 1, 720],
  ['height', '高度', 'dp', 1, 1_200],
  ['padding.start', '左内距', 'dp', 0, 128],
  ['padding.top', '上内距', 'dp', 0, 128],
  ['padding.end', '右内距', 'dp', 0, 128],
  ['padding.bottom', '下内距', 'dp', 0, 128],
  ['margin.start', '左外距', 'dp', -96, 192],
  ['margin.top', '上外距', 'dp', -96, 192],
  ['margin.end', '右外距', 'dp', -96, 192],
  ['margin.bottom', '下外距', 'dp', -96, 192],
  ['cornerRadius.all', '圆角', 'dp', 0, 96],
  ['textSize', '字号', 'sp', 8, 96],
  ['fontWeight', '字重', 'float', 100, 900],
  ['lineHeight', '行高', 'sp', 8, 160],
  ['letterSpacing', '字距', 'float', -0.1, 0.3],
  ['borderWidth', '边框', 'dp', 0, 32],
  ['opacity', '透明度', 'float', 0, 1],
] as const

const COLOR_FIELDS = [
  ['backgroundColor', '背景色'],
  ['contentColor', '文字色'],
  ['borderColor', '边框色'],
] as const

export function UiTunerLivePanel({
  state,
  error,
  busy,
  session,
  node,
  mcp,
  uiIr,
  targetDesign,
  draftStatus,
  onApply,
  onApplyGesture,
  onGestureActive,
  onUndo,
  onRedo,
  onReconnect,
  commitPlan,
  commitResult,
  onPreviewCommit,
  onCommit,
  onOpenPreview,
  buildVerifyResult,
  onBuildVerify,
  prepareBusy,
  prepareError,
  debugIntegration,
  lkgEnabled,
  onLkgEnabledChange,
  prepareReady,
  debugPackage,
  projectRoot,
  onProjectRootChange,
  onPrepareRuntime,
}: UiTunerLivePanelProps) {
  const [scope, setScope] = useState<LiveUiScope>('INSTANCE')
  const [projectRootDraft, setProjectRootDraft] = useState(projectRoot)
  const connected = state === 'connected'

  useEffect(() => setProjectRootDraft(projectRoot), [projectRoot])

  return (
    <section className={styles.panel} data-state={state}>
      <div className={styles.header}>
        <div>
          <span className={styles.eyebrow}>真机实时样式</span>
          <strong>{statusLabel(state)}</strong>
        </div>
        <span className={styles.statusDot} aria-hidden="true" />
      </div>

      {error && <p className={styles.hint}>{error}</p>}
      {debugIntegration && (
        <div className={styles.integrationStatus} role="status" aria-live="polite">
          <div>
            <strong>共享真机合并调试 · 第 {debugIntegration.desiredGeneration} 代</strong>
            <span>{debugIntegration.status}</span>
          </div>
          <small>固定包：{debugIntegration.packageName}</small>
          <small>最近成功版本：{lkgStatusLabel(debugIntegration.lkgEnabled)}</small>
          <small>基础提交：{debugIntegration.baseSha.slice(0, 12)} · 贡献 {debugIntegration.contributions.length} 个</small>
          {debugIntegration.contributions.slice(-5).map((contribution) => (
            <small key={contribution.commitSha} title={contribution.commitSha}>
              {contribution.commitSha.slice(0, 12)} · {contribution.sourceSessionId || contribution.sourceTaskId || '兼容调用'}
            </small>
          ))}
          {debugIntegration.conflicts.length > 0 && (
            <span>冲突：{debugIntegration.conflicts.map((commit) => commit.slice(0, 12)).join('、')}</span>
          )}
          {Boolean(debugIntegration.legacyPackages?.length) && (
            <span>历史调试包（仅报告，不自动卸载）：{debugIntegration.legacyPackages?.join('、')}</span>
          )}
          {debugIntegration.lastError && <span>{debugIntegration.lastError}</span>}
          {debugIntegration.lkgEnabled && debugIntegration.lastUsable && (
            <small>
              最后可用：第 {debugIntegration.lastUsable.generation} 代 · APK {debugIntegration.lastUsable.sha256.slice(0, 12)}
            </small>
          )}
        </div>
      )}
      {session?.id && <UiCapabilityGapPanel sessionId={session.id} />}
      {connected && (
        <div className={styles.connectedProjectField}>
          <label className={styles.projectField}>
            <span>源码写回项目目录</span>
            <input
              value={projectRootDraft}
              placeholder="例如 D:\\projects\\my-android-app"
              onChange={(event) => setProjectRootDraft(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && projectRootDraft.trim() !== projectRoot.trim()) {
                  onProjectRootChange(projectRootDraft)
                }
              }}
            />
          </label>
          <button
            type="button"
            disabled={!projectRootDraft.trim() || projectRootDraft.trim() === projectRoot.trim()}
            onClick={() => onProjectRootChange(projectRootDraft)}
          >切换源码目录</button>
        </div>
      )}
      {!connected && (
        <div className={styles.prepareCard}>
          <strong>启用真正的 LIVE 修改</strong>
          <p>正式 APK 只支持截图/XML 检查。安装并打开当前 PC 节点唯一的共享 Debug Runtime 包后，颜色、尺寸、间距、圆角和文字才能在真机立即变化。</p>
          <small>调试包：{debugPackage}</small>
          <label className={styles.lkgPolicy}>
            <input
              type="checkbox"
              checked={lkgEnabled}
              disabled={prepareBusy}
              onChange={(event) => onLkgEnabledChange(event.currentTarget.checked)}
            />
            <span>本次调试任务启用最近成功版本</span>
            <small>默认关闭；只有勾选后才记录、推进并校验最近成功版本。</small>
          </label>
          <label className={styles.projectField}>
            <span>本机 Android 项目目录</span>
            <input
              value={projectRoot}
              disabled={prepareBusy}
              placeholder="例如 D:\\projects\\my-android-app"
              onChange={(event) => onProjectRootChange(event.currentTarget.value)}
            />
          </label>
          {prepareError && <span>{prepareError}</span>}
          {prepareBusy && (
            <div className={styles.installAttention} role="status">
              <strong>请留意手机上的安装确认</strong>
              <small>首次安装节点专属 Debug 包时，部分荣耀、小米等系统会要求勾选风险提示并点“继续安装”；这是独立调试包的一次性安全确认，后续同签名更新通常会自动完成。</small>
            </div>
          )}
          <button
            className={styles.prepareButton}
            type="button"
            disabled={prepareBusy || !prepareReady}
            onClick={onPrepareRuntime}
          >
            {prepareBusy ? '正在构建并安装…' : '一键安装并连接实时调试包'}
          </button>
          {!prepareReady && <small>请先连接手机，并选择项目或填写本机 Android 项目目录。</small>}
        </div>
      )}
      {connected && !node && (
        <p className={styles.hint}>当前元素未匹配到 Runtime Node。可继续用右侧 Codex 修改源码，或为 View 添加稳定 uiNode ID。</p>
      )}
      {node && (
        <>
          <div className={styles.nodeInfo}>
            <strong>{node.definitionId}</strong>
            <small>{node.kind} · {node.runtimeNodeId}</small>
          </div>
          <div className={styles.irState}>
            <span>{uiIr ? 'UI IR ' + uiIr.revision.slice(0, 16) : '正在生成 UI IR'}</span>
            <span>{mcp ? 'Codex 按需工具已就绪' : 'Codex 工具待连接'}</span>
            <span>{targetDesign ? '目标图 ' + targetDesign.sha256.slice(0, 12) : '尚未导入目标设计图'}</span>
            <span>{draftStatusLabel(draftStatus)}</span>
          </div>
          <label className={styles.scopeField}>
            <span>作用范围</span>
            <select value={scope} onChange={(event) => setScope(event.currentTarget.value as LiveUiScope)}>
              <option value="INSTANCE">只改当前实例</option>
              <option value="DEFINITION">修改同一组件</option>
            </select>
          </label>

          <div className={styles.grid}>
            {NUMBER_FIELDS.map(([property, label, valueType, minimum, maximum]) => node.properties[property] && (
              <NumberLiveField
                key={property}
                label={label}
                value={numberValue(node, property)}
                disabled={busy}
                step={numericStep(property)}
                minimum={node.properties[property]?.constraints?.minimum ?? minimum}
                maximum={node.properties[property]?.constraints?.maximum ?? maximum}
                onGestureActive={onGestureActive}
                onPreview={async (value, gestureId) => {
                  await onApplyGesture([{
                    property,
                    value: { type: valueType, value },
                  }], gestureId)
                }}
                onCommit={async (value) => {
                  await onApply({
                    property,
                    value: { type: valueType, value },
                  }, scope)
                }}
              />
            ))}
          </div>

          {COLOR_FIELDS.map(([property, label]) => node.properties[property] && (
            <ColorLiveField
              key={property}
              label={label}
              value={stringValue(node, property)}
              disabled={busy}
              onCommit={async (value) => {
                await onApply({ property, value: { type: 'argb', value } }, scope)
              }}
            />
          ))}
          {node.properties.text && (
            <TextLiveField
              label="文案"
              value={stringValue(node, 'text')}
              disabled={busy}
              onCommit={async (value) => {
                await onApply({ property: 'text', value: { type: 'text', value } }, scope)
              }}
            />
          )}

          <div className={styles.actions}>
            <button type="button" disabled={busy || (draftStatus !== 'rejected' && (draftStatus !== 'confirmed' || !session?.historyCount))} onClick={() => { void onUndo() }}>
              {draftStatus === 'rejected' ? '放弃本地草稿' : '撤销 LIVE'}
            </button>
            <button type="button" disabled={busy || draftStatus !== 'confirmed' || !session?.redoCount} onClick={() => { void onRedo() }}>
              重做
            </button>
          </div>
          <UiTunerPreviewPanel busy={busy} onOpen={onOpenPreview} />
          <button
            className={styles.commitButton}
            type="button"
            disabled={busy || !session?.historyCount || draftStatus !== 'confirmed'}
            onClick={() => { void onPreviewCommit() }}
          >
            生成源码写回计划
          </button>
          {commitPlan && (
            <CommitPlanView
              plan={commitPlan}
              busy={busy}
              onCommit={onCommit}
            />
          )}
          {commitResult ? (
            <div className={styles.savedState}>
              <strong>SOURCE SAVED</strong>
              <span>已写入 {commitResult.changedFiles.length} 个文件；需重新构建并清空 Patch 后验收。</span>
              {commitResult.changedFiles.map((file) => <small key={file}>{file}</small>)}
              <button
                className={styles.commitButton}
                type="button"
                disabled={busy}
                onClick={() => { void onBuildVerify() }}
              >
                构建、安装并真机验收
              </button>
            </div>
          ) : (
            <>
              <p className={styles.previewWarning}>{draftStatusLabel(draftStatus)} · 源码尚未写入或尚未完成构建验收</p>
              <button
                className={styles.commitButton}
                type="button"
                disabled={busy}
                onClick={() => { void onBuildVerify() }}
              >
                构建当前源码并真机验收
              </button>
            </>
          )}
          {buildVerifyResult && (
            <div className={styles.savedState}>
              <strong>{buildVerifyResult.status.replace(/_/g, ' ')}</strong>
              <span>{buildVerifyResult.message}</span>
              <small>
                {buildVerifyResult.screenshotWidth} × {buildVerifyResult.screenshotHeight}
                {' · '}{buildVerifyResult.nodeCount} 节点
                {buildVerifyResult.sourceParityDiff
                  ? ` · 源码一致性损失 ${buildVerifyResult.sourceParityDiff.visualLoss.toFixed(4)}`
                  : ' · 本机节点未返回源码一致性结果'}
                {buildVerifyResult.visualDiff ? ` · 设计图损失 ${buildVerifyResult.visualDiff.visualLoss.toFixed(4)}` : ''}
                {buildVerifyResult.verificationGate?.failedMetrics.length
                  ? ` · 未通过：${buildVerifyResult.verificationGate.failedMetrics.join(', ')}`
                  : ''}
              </small>
            </div>
          )}
        </>
      )}

      {!connected && !prepareBusy && (
        <button className={styles.reconnect} type="button" disabled={state === 'connecting'} onClick={onReconnect}>
          {state === 'connecting' ? '正在自动连接…' : '重新连接 Debug Runtime'}
        </button>
      )}
    </section>
  )
}

function CommitPlanView({
  plan,
  busy,
  onCommit,
}: {
  plan: LiveSourceCommitPlan
  busy: boolean
  onCommit: (plan: LiveSourceCommitPlan) => Promise<LiveSourceCommitResult>
}) {
  const sharedImpact = plan.entries.filter((entry) => entry.impactCount > 1)
  return (
    <div className={styles.commitPlan}>
      <div>
        <strong>{plan.deterministicCount} 项可直接写入</strong>
        <span>{plan.codexCount} 项需 Codex</span>
      </div>
      {plan.entries.slice(0, 6).map((entry) => (
        <div className={styles.commitEntry} key={`${entry.definitionId}:${entry.property}`}>
          <span>{entry.property}</span>
          <strong>{entry.sourceKey ?? entry.commitMode}</strong>
          <small>{entry.reason}</small>
        </div>
      ))}
      {sharedImpact.length > 0 && (
        <small>其中 {sharedImpact.length} 项会修改共享资源，请先核对上方影响范围。</small>
      )}
      <button
        className={styles.commitButton}
        type="button"
        disabled={busy || plan.deterministicCount === 0}
        onClick={() => { void onCommit(plan) }}
      >
        确认写入源码
      </button>
    </div>
  )
}

function NumberLiveField({
  label,
  value,
  step,
  minimum,
  maximum,
  disabled,
  onCommit,
  onPreview,
  onGestureActive,
}: {
  label: string
  value: number
  step: number
  minimum: number
  maximum: number
  disabled: boolean
  onCommit: (value: number) => Promise<unknown>
  onPreview: (value: number, gestureId: string) => Promise<unknown>
  onGestureActive: (active: boolean) => void
}) {
  const [draft, setDraft] = useState(String(value))
  const [committing, setCommitting] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const gestureIdRef = useRef('')
  const pendingRef = useRef<number | null>(null)
  const inFlightRef = useRef<Promise<void> | null>(null)
  const idleEndTimerRef = useRef<number | null>(null)
  useEffect(() => setDraft(String(value)), [value])
  const isSameValue = (next: number) => Math.abs(next - value) < Math.max(0.0001, step / 2)
  const commit = async () => {
    // Read the element at commit time. This keeps rapid edit + Apply actions
    // reliable even when a fresh Runtime tree arrives between input events.
    const next = Number(inputRef.current?.value ?? draft)
    if (!Number.isFinite(next) || isSameValue(next)) return
    setCommitting(true)
    try {
      await onCommit(next)
    } catch {
      setDraft(String(value))
    } finally {
      setCommitting(false)
    }
  }
  const flushPreview = () => {
    if (inFlightRef.current || pendingRef.current == null || !gestureIdRef.current) return
    const next = pendingRef.current
    pendingRef.current = null
    const task = onPreview(next, gestureIdRef.current)
      .then(() => undefined)
      .catch(() => { setDraft(String(value)) })
      .finally(() => {
        inFlightRef.current = null
        flushPreview()
      })
    inFlightRef.current = task
  }
  const beginPreview = () => {
    if (disabled || committing || gestureIdRef.current) return
    gestureIdRef.current = `slider_${Date.now()}_${Math.random().toString(16).slice(2)}`
    onGestureActive(true)
  }
  const updatePreview = (next: number) => {
    if (!gestureIdRef.current) beginPreview()
    setDraft(String(next))
    pendingRef.current = next
    flushPreview()
    if (idleEndTimerRef.current != null) window.clearTimeout(idleEndTimerRef.current)
    idleEndTimerRef.current = window.setTimeout(() => { void endPreview() }, 220)
  }
  const endPreview = async () => {
    if (!gestureIdRef.current) return
    if (idleEndTimerRef.current != null) {
      window.clearTimeout(idleEndTimerRef.current)
      idleEndTimerRef.current = null
    }
    flushPreview()
    while (inFlightRef.current || pendingRef.current != null) {
      if (!inFlightRef.current) flushPreview()
      await (inFlightRef.current ?? Promise.resolve())
    }
    gestureIdRef.current = ''
    onGestureActive(false)
  }
  const nudgePreview = async (direction: -1 | 1) => {
    const current = Number(draft)
    const next = Math.min(maximum, Math.max(minimum, (Number.isFinite(current) ? current : value) + (step * direction)))
    if (isSameValue(next)) return
    setDraft(String(next))
    setCommitting(true)
    try {
      await onCommit(next)
    } catch {
      setDraft(String(value))
    } finally {
      setCommitting(false)
    }
  }
  return (
    <div className={styles.liveFieldRow}>
      <div className={styles.liveNumberField}>
        <span>{label}</span>
        <span className={styles.liveNumberInputs}>
          <button
            type="button"
            aria-label={`减小${label}`}
            disabled={disabled || committing || Number(draft) <= minimum}
            onClick={() => { void nudgePreview(-1) }}
          >−</button>
          <input
            type="range"
            aria-label={`实时调整${label}`}
            value={Math.min(maximum, Math.max(minimum, Number(draft) || 0))}
            min={minimum}
            max={maximum}
            step={step}
            disabled={disabled || committing}
            onPointerDown={beginPreview}
            onPointerUp={() => { void endPreview() }}
            onPointerCancel={() => { void endPreview() }}
            onKeyDown={(event) => {
              if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown'].includes(event.key)) {
                beginPreview()
              }
            }}
            onKeyUp={() => { void endPreview() }}
            onChange={(event) => updatePreview(Number(event.currentTarget.value))}
          />
          <button
            type="button"
            aria-label={`增大${label}`}
            disabled={disabled || committing || Number(draft) >= maximum}
            onClick={() => { void nudgePreview(1) }}
          >+</button>
          <input
            ref={inputRef}
            type="number"
            aria-label={label}
            value={draft}
            step={step}
            disabled={disabled || committing}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={(event) => { if (event.key === 'Enter') void commit() }}
          />
        </span>
      </div>
      <button
        type="button"
        disabled={disabled || committing || isSameValue(Number(draft))}
        aria-label={`应用${label}`}
        onClick={() => { void commit() }}
      >
        {committing ? '…' : '应用'}
      </button>
    </div>
  )
}

function ColorLiveField({
  label,
  value,
  disabled,
  onCommit,
}: {
  label: string
  value: string
  disabled: boolean
  onCommit: (value: string) => Promise<unknown>
}) {
  const [draft, setDraft] = useState(value)
  const [committing, setCommitting] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  useEffect(() => setDraft(value), [value])
  const commit = async () => {
    const next = normalizeColorDraft(inputRef.current?.value ?? draft)
    if (!next || next === value) return
    setCommitting(true)
    try {
      await onCommit(next)
    } catch {
      setDraft(value)
    } finally {
      setCommitting(false)
    }
  }
  const nativeColor = nativePickerColor(draft)
  return (
    <div className={styles.liveFieldRow}>
      <label className={styles.fieldFull}>
        <span>{label}</span>
        <span style={{ display: 'grid', gridTemplateColumns: '44px minmax(0, 1fr)', alignItems: 'center', gap: 8, minWidth: 0 }}>
          <input
            className={styles.colorSwatchInput}
            type="color"
            aria-label={`${label}取色器`}
            value={nativeColor}
            disabled={disabled || committing}
            onChange={(event) => setDraft(event.currentTarget.value)}
          />
          <input
            ref={inputRef}
            className={styles.colorTextInput}
            aria-label={`${label}颜色值`}
            value={draft}
            placeholder="#222255 / #ff222255 / rgba(34,34,85,.9)"
            disabled={disabled || committing}
            spellCheck={false}
            onFocus={(event) => event.currentTarget.select()}
            onChange={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={(event) => { if (event.key === 'Enter') void commit() }}
          />
        </span>
      </label>
      <button
        type="button"
        disabled={disabled || committing || normalizeColorDraft(draft) === value}
        aria-label={`应用${label}`}
        onClick={() => { void commit() }}
      >
        {committing ? '…' : '应用'}
      </button>
    </div>
  )
}

function TextLiveField({
  label,
  value,
  disabled,
  onCommit,
}: {
  label: string
  value: string
  disabled: boolean
  onCommit: (value: string) => Promise<unknown>
}) {
  const [draft, setDraft] = useState(value)
  const [committing, setCommitting] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  useEffect(() => setDraft(value), [value])
  const commit = async () => {
    const next = (inputRef.current?.value ?? draft).trim()
    if (!next || next === value) return
    setCommitting(true)
    try {
      await onCommit(next)
    } catch {
      setDraft(value)
    } finally {
      setCommitting(false)
    }
  }
  return (
    <div className={styles.liveFieldRow}>
      <label className={styles.fieldFull}>
        <span>{label}</span>
        <input
          ref={inputRef}
          value={draft}
          disabled={disabled || committing}
          onChange={(event) => setDraft(event.currentTarget.value)}
          onKeyDown={(event) => { if (event.key === 'Enter') void commit() }}
        />
      </label>
      <button
        type="button"
        disabled={disabled || committing || draft.trim() === value}
        aria-label={`应用${label}`}
        onClick={() => { void commit() }}
      >
        {committing ? '…' : '应用'}
      </button>
    </div>
  )
}

function numberValue(node: LiveUiNode, property: string) {
  const raw = node.properties[property]?.effective?.value
  return typeof raw === 'number' ? raw : Number(raw) || 0
}

function stringValue(node: LiveUiNode, property: string) {
  return String(node.properties[property]?.effective?.value ?? '')
}

function normalizeColorDraft(value: string) {
  return value.trim()
}

function nativePickerColor(value: string) {
  const trimmed = value.trim()
  if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) return trimmed
  if (/^#[0-9a-fA-F]{8}$/.test(trimmed)) return `#${trimmed.slice(3)}`
  return '#000000'
}

function numericStep(property: string) {
  if (property === 'opacity') return 0.05
  if (property === 'letterSpacing') return 0.01
  if (property === 'fontWeight') return 100
  return 1
}

function statusLabel(state: LiveUiConnectionState) {
  switch (state) {
    case 'connected': return '已连接 · LIVE'
    case 'connecting': return '正在连接'
    case 'attach_only': return 'Attach Mode'
    case 'error': return '连接异常'
    default: return '等待真机捕获'
  }
}

function draftStatusLabel(status: RuntimeDraftStatus) {
  if (status === 'local') return 'PC 即时预览 · 尚未同步真机'
  if (status === 'syncing') return 'PC 已更新 · 真机后台同步中'
  if (status === 'calibrating') return '真机已接收 · 正在校准画面'
  if (status === 'rejected') return 'PC 草稿已保留 · 真机同步失败'
  return '真机画面已校准'
}
