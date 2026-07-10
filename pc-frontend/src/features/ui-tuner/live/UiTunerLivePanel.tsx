import { useEffect, useState } from 'react'
import type { UiTunerElement } from '../types'
import type {
  LivePatchOperation,
  LiveUiNode,
  LiveUiScope,
  LiveUiSession,
} from './liveUiApi'
import type { LiveUiConnectionState } from './useLiveUiSession'
import type { LiveSourceCommitPlan, LiveSourceCommitResult } from './liveUiCommitApi'
import type {
  LiveTargetDesign,
  LiveUiIrDocument,
  PixelRect,
  VisualSolverResult,
} from './liveUiIrApi'
import type { LiveMcpDescriptor } from './liveUiApi'
import styles from './UiTunerLivePanel.module.css'

interface UiTunerLivePanelProps {
  state: LiveUiConnectionState
  error: string
  busy: boolean
  session: LiveUiSession | null
  node: LiveUiNode | null
  selected: UiTunerElement
  mcp: LiveMcpDescriptor | null
  uiIr: LiveUiIrDocument | null
  targetDesign: LiveTargetDesign | null
  solverResult: VisualSolverResult | null
  onApply: (operation: LivePatchOperation, scope: LiveUiScope) => Promise<unknown>
  onUndo: () => Promise<void>
  onRedo: () => Promise<void>
  onReconnect: () => void
  onOptimisticUpdate: (patch: Partial<UiTunerElement>) => void
  commitPlan: LiveSourceCommitPlan | null
  commitResult: LiveSourceCommitResult | null
  onPreviewCommit: () => Promise<LiveSourceCommitPlan>
  onCommit: (plan: LiveSourceCommitPlan) => Promise<LiveSourceCommitResult>
  onSolve: (targetRect: PixelRect) => Promise<VisualSolverResult>
}

const NUMBER_FIELDS = [
  ['width', '宽度', 'dp'],
  ['height', '高度', 'dp'],
  ['padding.start', '左内距', 'dp'],
  ['padding.top', '上内距', 'dp'],
  ['padding.end', '右内距', 'dp'],
  ['padding.bottom', '下内距', 'dp'],
  ['cornerRadius.all', '圆角', 'dp'],
  ['textSize', '字号', 'sp'],
  ['borderWidth', '边框', 'dp'],
  ['opacity', '透明度', 'float'],
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
  selected,
  mcp,
  uiIr,
  targetDesign,
  solverResult,
  onApply,
  onUndo,
  onRedo,
  onReconnect,
  onOptimisticUpdate,
  commitPlan,
  commitResult,
  onPreviewCommit,
  onCommit,
  onSolve,
}: UiTunerLivePanelProps) {
  const [scope, setScope] = useState<LiveUiScope>('INSTANCE')
  const connected = state === 'connected'

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
          </div>
          <label className={styles.scopeField}>
            <span>作用范围</span>
            <select value={scope} onChange={(event) => setScope(event.currentTarget.value as LiveUiScope)}>
              <option value="INSTANCE">只改当前实例</option>
              <option value="DEFINITION">修改同一组件</option>
            </select>
          </label>

          <div className={styles.grid}>
            {NUMBER_FIELDS.map(([property, label, valueType]) => node.properties[property] && (
              <NumberLiveField
                key={property}
                label={label}
                value={numberValue(node, property)}
                disabled={busy}
                step={property === 'opacity' ? 0.05 : 1}
                onCommit={async (value) => {
                  await onApply({
                    property,
                    value: { type: valueType, value },
                  }, scope)
                  onOptimisticUpdate(optimisticPatch(property, value))
                }}
              />
            ))}
          </div>

          {COLOR_FIELDS.map(([property, label]) => node.properties[property] && (
            <TextLiveField
              key={property}
              label={label}
              value={stringValue(node, property)}
              disabled={busy}
              onCommit={async (value) => {
                await onApply({ property, value: { type: 'argb', value } }, scope)
                onOptimisticUpdate(optimisticPatch(property, value))
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
                onOptimisticUpdate({ text: value })
              }}
            />
          )}

          <div className={styles.actions}>
            <button type="button" disabled={busy || !session?.historyCount} onClick={() => { void onUndo() }}>
              撤销 LIVE
            </button>
            <button type="button" disabled={busy || !session?.redoCount} onClick={() => { void onRedo() }}>
              重做
            </button>
          </div>
          <VisualSolverPanel
            key={node.runtimeNodeId}
            node={node}
            selected={selected}
            targetReady={Boolean(targetDesign)}
            busy={busy}
            result={solverResult}
            onSolve={onSolve}
          />
          <button
            className={styles.commitButton}
            type="button"
            disabled={busy || !session?.historyCount}
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
            </div>
          ) : (
            <p className={styles.previewWarning}>LIVE PREVIEW · 真机已变化，源码尚未写入</p>
          )}
        </>
      )}

      {!connected && (
        <button className={styles.reconnect} type="button" disabled={state === 'connecting'} onClick={onReconnect}>
          {state === 'connecting' ? '正在自动连接…' : '重新连接 Debug Runtime'}
        </button>
      )}
    </section>
  )
}

function VisualSolverPanel({
  node,
  selected,
  targetReady,
  busy,
  result,
  onSolve,
}: {
  node: LiveUiNode
  selected: UiTunerElement
  targetReady: boolean
  busy: boolean
  result: VisualSolverResult | null
  onSolve: (targetRect: PixelRect) => Promise<VisualSolverResult>
}) {
  const bounds = node.geometry.boundsInDisplayPx
  const [rect, setRect] = useState({
    x: Math.round(selected.x || bounds.left),
    y: Math.round(selected.y || bounds.top),
    width: Math.round(selected.width || bounds.width),
    height: Math.round(selected.height || bounds.height),
  })
  const setValue = (key: keyof typeof rect, value: number) => {
    setRect((current) => ({ ...current, [key]: Number.isFinite(value) ? value : current[key] }))
  }
  const targetRect: PixelRect = {
    left: rect.x,
    top: rect.y,
    right: rect.x + Math.max(1, rect.width),
    bottom: rect.y + Math.max(1, rect.height),
  }
  return (
    <div className={styles.solver}>
      <div>
        <strong>本地视觉求解</strong>
        <span>目标图区域（px）</span>
      </div>
      <div className={styles.grid}>
        {([
          ['x', 'X'],
          ['y', 'Y'],
          ['width', '宽'],
          ['height', '高'],
        ] as const).map(([key, label]) => (
          <label className={styles.field} key={key}>
            <span>{label}</span>
            <input
              type="number"
              value={rect[key]}
              disabled={busy}
              onChange={(event) => setValue(key, Number(event.currentTarget.value))}
            />
          </label>
        ))}
      </div>
      <button
        className={styles.commitButton}
        type="button"
        disabled={busy || !targetReady}
        onClick={() => { void onSolve(targetRect) }}
      >
        {busy ? '正在真机试探…' : '自动逼近目标图'}
      </button>
      {!targetReady && <small>请先点击顶部“导入设计图/截图”。</small>}
      {result && (
        <small>
          {result.evaluations} 次本地比较 · 损失
          {' '}{result.baseline.visualLoss.toFixed(4)} → {result.finalDiff.visualLoss.toFixed(4)}
          {' '}· 改善 {result.improvementPercent.toFixed(2)}%
        </small>
      )}
    </div>
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
  const confirmCommit = () => {
    const impactText = sharedImpact.length > 0
      ? `\n其中 ${sharedImpact.length} 项会修改共享资源，请核对影响范围。`
      : ''
    if (!window.confirm(
      `将确定性写入 ${plan.deterministicCount} 项源码；${plan.codexCount} 项复杂修改保留给 Codex。${impactText}`,
    )) return
    void onCommit(plan)
  }
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
      <button
        className={styles.commitButton}
        type="button"
        disabled={busy || plan.deterministicCount === 0}
        onClick={confirmCommit}
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
  disabled,
  onCommit,
}: {
  label: string
  value: number
  step: number
  disabled: boolean
  onCommit: (value: number) => Promise<unknown>
}) {
  const [draft, setDraft] = useState(String(value))
  useEffect(() => setDraft(String(value)), [value])
  const commit = () => {
    const next = Number(draft)
    if (!Number.isFinite(next) || next === value) return
    void onCommit(next).catch(() => setDraft(String(value)))
  }
  return (
    <label className={styles.field}>
      <span>{label}</span>
      <input
        type="number"
        value={draft}
        step={step}
        disabled={disabled}
        onChange={(event) => setDraft(event.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur() }}
      />
    </label>
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
  useEffect(() => setDraft(value), [value])
  const commit = () => {
    const next = draft.trim()
    if (!next || next === value) return
    void onCommit(next).catch(() => setDraft(value))
  }
  return (
    <label className={styles.fieldFull}>
      <span>{label}</span>
      <input
        value={draft}
        disabled={disabled}
        onChange={(event) => setDraft(event.currentTarget.value)}
        onBlur={commit}
        onKeyDown={(event) => { if (event.key === 'Enter') event.currentTarget.blur() }}
      />
    </label>
  )
}

function numberValue(node: LiveUiNode, property: string) {
  const raw = node.properties[property]?.effective?.value
  return typeof raw === 'number' ? raw : Number(raw) || 0
}

function stringValue(node: LiveUiNode, property: string) {
  return String(node.properties[property]?.effective?.value ?? '')
}

function optimisticPatch(property: string, value: number | string): Partial<UiTunerElement> {
  switch (property) {
    case 'width': return { width: Number(value) }
    case 'height': return { height: Number(value) }
    case 'padding.start':
    case 'padding.end': return { paddingX: Number(value) }
    case 'padding.top':
    case 'padding.bottom': return { paddingY: Number(value) }
    case 'cornerRadius.all': return { borderRadius: Number(value) }
    case 'textSize': return { fontSize: Number(value) }
    case 'borderWidth': return { borderWidth: Number(value) }
    case 'opacity': return { opacity: Number(value) }
    case 'backgroundColor': return { background: normalizeCssColor(String(value)) }
    case 'contentColor': return { color: normalizeCssColor(String(value)) }
    case 'borderColor': return { borderColor: normalizeCssColor(String(value)) }
    default: return {}
  }
}

function normalizeCssColor(value: string) {
  return /^#[0-9a-f]{8}$/i.test(value) ? `#${value.slice(3)}${value.slice(1, 3)}` : value
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
