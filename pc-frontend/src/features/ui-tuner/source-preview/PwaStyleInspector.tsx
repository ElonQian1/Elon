import { Bot, Copy, Download, MousePointer2, Redo2, RotateCcw, Save, Smartphone, Trash2, Undo2 } from 'lucide-react'
import { buildPwaDraftCliCompactHandoff, type PwaStyleProperty } from './pwaDesignDraft'
import type { PwaDesignSession, PwaSelection } from './usePwaDesignSession'
import { CrossPlatformWritebackReceiptPanel } from './CrossPlatformWritebackReceiptPanel'
import { PwaDesignerWorkflowGuide } from './PwaDesignerWorkflowGuide'
import styles from './SourcePreview.module.css'

interface Props {
  session: PwaDesignSession
}

interface FieldSpec {
  property: PwaStyleProperty
  label: string
  placeholder?: string
  quickStep?: number
  quickUnit?: string
  min?: number
  max?: number
}

const SIZE_FIELDS: FieldSpec[] = [
  { property: 'width', label: '宽度', placeholder: 'auto / 100% / 320px', quickStep: 4, quickUnit: 'px', min: 1 },
  { property: 'height', label: '高度', placeholder: 'auto / 100% / 48px', quickStep: 4, quickUnit: 'px', min: 1 },
]

const PADDING_FIELDS: FieldSpec[] = [
  { property: 'paddingTop', label: '上', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingRight', label: '右', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingBottom', label: '下', quickStep: 2, quickUnit: 'px', min: 0 },
  { property: 'paddingLeft', label: '左', quickStep: 2, quickUnit: 'px', min: 0 },
]

const MARGIN_FIELDS: FieldSpec[] = [
  { property: 'marginTop', label: '上', quickStep: 2, quickUnit: 'px' },
  { property: 'marginRight', label: '右', quickStep: 2, quickUnit: 'px' },
  { property: 'marginBottom', label: '下', quickStep: 2, quickUnit: 'px' },
  { property: 'marginLeft', label: '左', quickStep: 2, quickUnit: 'px' },
]

const TYPE_FIELDS: FieldSpec[] = [
  { property: 'fontSize', label: '字号', placeholder: '16px / 1rem', quickStep: 1, quickUnit: 'px', min: 8 },
  { property: 'fontWeight', label: '字重', placeholder: '400 / 700', quickStep: 100, min: 100, max: 900 },
  { property: 'lineHeight', label: '行高', placeholder: '1.5 / 24px', quickStep: 1, quickUnit: 'px', min: 8 },
  { property: 'borderRadius', label: '圆角', placeholder: '8px / 50%', quickStep: 2, quickUnit: 'px', min: 0 },
]

const STYLE_PRESETS = [
  {
    label: '紧凑',
    hint: '小按钮/列表项',
    styles: { paddingTop: '6px', paddingRight: '10px', paddingBottom: '6px', paddingLeft: '10px', borderRadius: '10px', fontSize: '13px' },
  },
  {
    label: '标准',
    hint: '常规移动端',
    styles: { paddingTop: '10px', paddingRight: '14px', paddingBottom: '10px', paddingLeft: '14px', borderRadius: '14px', fontSize: '14px' },
  },
  {
    label: '舒展',
    hint: '主操作/卡片',
    styles: { paddingTop: '14px', paddingRight: '18px', paddingBottom: '14px', paddingLeft: '18px', borderRadius: '18px', fontSize: '15px' },
  },
  {
    label: '胶囊',
    hint: '圆润按钮',
    styles: { borderRadius: '999px', paddingLeft: '18px', paddingRight: '18px' },
  },
] satisfies Array<{ label: string; hint: string; styles: Partial<Record<PwaStyleProperty, string>> }>

function originalValue(selection: PwaSelection, property: PwaStyleProperty): string {
  return selection.originalStyle.authored[property]
    || selection.originalStyle.computed[property]
    || ''
}

function fieldValue(session: PwaDesignSession, property: PwaStyleProperty): string {
  const selection = session.selection
  if (!selection) return ''
  return Object.values(session.draft?.elements ?? {}).find((element) => (
    element.identity.key === selection.identity.key || element.identity.selector === selection.identity.selector
  ))?.styleDiff[property]
    ?? originalValue(selection, property)
}

function normalizeNumber(value: number, min?: number, max?: number): number {
  const clampedMin = typeof min === 'number' ? Math.max(min, value) : value
  return typeof max === 'number' ? Math.min(max, clampedMin) : clampedMin
}

function formatNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.?0+$/, '')
}

function adjustedCssValue(current: string, delta: number, fallbackUnit = '', min?: number, max?: number): string {
  const text = current.trim()
  const match = text.match(/^(-?\d+(?:\.\d+)?)([a-z%]*)$/i)
  if (match) {
    const unit = match[2] || fallbackUnit
    return `${formatNumber(normalizeNumber(Number(match[1]) + delta, min, max))}${unit}`
  }
  const fallback = fallbackUnit || 'px'
  return `${formatNumber(normalizeNumber(delta > 0 ? delta : 0, min, max))}${fallback}`
}

function StyleField({ session, spec }: { session: PwaDesignSession; spec: FieldSpec }) {
  const value = fieldValue(session, spec.property)
  const canQuickAdjust = typeof spec.quickStep === 'number'
  const quickAdjust = (direction: -1 | 1) => {
    if (!canQuickAdjust) return
    session.updateStyle(
      spec.property,
      adjustedCssValue(value, spec.quickStep! * direction, spec.quickUnit, spec.min, spec.max),
    )
  }
  return (
    <label className={styles.pwaStyleField}>
      <span>{spec.label}</span>
      <div className={styles.pwaQuickAdjust}>
        {canQuickAdjust && <button type="button" onClick={() => quickAdjust(-1)} aria-label={`${spec.label}减小`}>−</button>}
        <input
          value={value}
          placeholder={spec.placeholder}
          onChange={(event) => session.updateStyle(spec.property, event.currentTarget.value)}
        />
        {canQuickAdjust && <button type="button" onClick={() => quickAdjust(1)} aria-label={`${spec.label}增大`}>+</button>}
      </div>
    </label>
  )
}

function EdgeFields({ session, fields }: { session: PwaDesignSession; fields: FieldSpec[] }) {
  return <div className={styles.pwaEdgeGrid}>{fields.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
}

function confidenceLabel(selection: PwaSelection): string {
  if (selection.identity.confidence === 'high') return '稳定映射'
  if (selection.identity.confidence === 'medium') return '候选映射'
  return 'DOM 路径'
}

function bindingLabel(status?: 'BOUND' | 'CANDIDATE' | 'NEEDS_AI') {
  if (status === 'BOUND') return '双端来源已绑定'
  if (status === 'CANDIDATE') return '已有双端来源候选'
  return '需要 AI 建立绑定'
}

function writebackTargetLabel(status: 'DETERMINISTIC' | 'DETERMINISTIC_PARTIAL' | 'CODEX_REQUIRED') {
  if (status === 'DETERMINISTIC') return '确定性写回'
  if (status === 'DETERMINISTIC_PARTIAL') return '部分确定性 · 部分需要 AI'
  return '需要 AI 建立绑定/结构修改'
}

function syncPhaseLabel(phase: PwaDesignSession['syncState']['phase']): string {
  if (phase === 'LIVE_PREVIEW') return '临时预览'
  if (phase === 'AI_WRITING') return 'AI 写源码'
  if (phase === 'SOURCE_SAVED') return '源码已保存'
  if (phase === 'BUILD_VERIFYING') return '真实构建验证'
  if (phase === 'BUILD_VERIFIED') return '验证通过'
  return '验证失败'
}

function syncStepState(phase: PwaDesignSession['syncState']['phase'], step: 'preview' | 'source' | 'verify' | 'done'): 'done' | 'active' | 'pending' | 'failed' {
  if (phase === 'VERIFY_FAILED') return step === 'verify' ? 'failed' : step === 'done' ? 'pending' : 'done'
  if (step === 'preview') return 'done'
  if (step === 'source') {
    if (phase === 'AI_WRITING' || phase === 'SOURCE_SAVED') return 'active'
    if (phase === 'BUILD_VERIFYING' || phase === 'BUILD_VERIFIED') return 'done'
    return 'pending'
  }
  if (step === 'verify') {
    if (phase === 'BUILD_VERIFYING') return 'active'
    if (phase === 'BUILD_VERIFIED') return 'done'
    return 'pending'
  }
  return phase === 'BUILD_VERIFIED' ? 'done' : 'pending'
}

function SyncProgress({ session }: Props) {
  const steps = [
    { key: 'preview', label: '草稿实时预览', hint: 'PWA 页面已临时变化' },
    { key: 'source', label: '写入源码', hint: session.writebackPlan.requiresCodex ? 'AI 只补绑定/结构缺口' : '确定性写回优先' },
    { key: 'verify', label: '真实构建验证', hint: '重载 PWA/APK 证明源码生效' },
    { key: 'done', label: '完成交付', hint: '无 Runtime Patch 也一致' },
  ] as const
  return (
    <ol className={styles.pwaSyncProgress} aria-label="PWA 到 APK 写回验证进度">
      {steps.map((step) => (
        <li key={step.key} data-step-state={syncStepState(session.syncState.phase, step.key)}>
          <strong>{step.label}</strong>
          <span>{step.hint}</span>
        </li>
      ))}
    </ol>
  )
}

function designModeTitle(session: PwaDesignSession): string {
  if (!session.ready) return '正在连接真实 PWA'
  if (session.mode === 'select') return '选择模式：下一次点击会选中组件'
  if (session.selection) return '设计模式：正在修改选中组件'
  return '操作模式：正常使用'
}

function designModeHint(session: PwaDesignSession): string {
  if (!session.ready) return '连接后先操作到目标页面。'
  if (session.mode === 'select') return '点击要修改的真实组件。'
  if (session.selection) return '改尺寸、间距、圆角、颜色；确认后写回源码。'
  return '到目标页后点击“选择组件”。'
}

function BridgeHealthCard({ session }: Props) {
  const health = session.bridgeHealth
  return (
    <section className={styles.pwaBridgeHealthCard} data-ready={health?.ready ? 'true' : 'false'} aria-label="PWA 草稿链路自检">
      <div>
        <strong>{health?.ready ? 'PWA 草稿链路已连接' : '等待 PWA 草稿链路'}</strong>
        <span>{health
          ? `${health.mode === 'select' ? '选择组件' : '正常操作'} · 可编辑 ${health.editablePropertyCount} 类样式`
          : '连接后自动检测草稿、回写和样式命令。'}</span>
      </div>
      {health && <dl>
        <div><dt>草稿命令</dt><dd>{health.canApplyDraft ? '可用' : '不可用'}</dd></div>
        <div><dt>源码验证</dt><dd>{health.canVerifySource ? '可用' : '待后端注入'}</dd></div>
        <div><dt>选中组件</dt><dd>{health.selected ? '有' : '无'}</dd></div>
        <div><dt>草稿应用</dt><dd>{health.draft ? `${health.draft.appliedCount}/${health.draft.requestedCount}` : '无草稿'}</dd></div>
      </dl>}
    </section>
  )
}

function WritebackPlanSummary({ session }: Props) {
  const plan = session.writebackPlan
  const deterministicCount = plan.deterministic.pwa.length + plan.deterministic.android.length
  const codexCount = plan.codexChanges.length
  if (!deterministicCount && !codexCount) return null
  return (
    <div className={styles.pwaWritebackPlanSummary}>
      <span>可直接写回 {deterministicCount} 组</span>
      <span>AI 只补 {codexCount} 项</span>
      {plan.codexReasons.slice(0, 2).map((reason) => <small key={reason}>{reason}</small>)}
      {plan.codexReasons.length > 2 && <small>还有 {plan.codexReasons.length - 2} 个缺口进 CLI 包。</small>}
    </div>
  )
}

function HandoffSummary({ session }: Props) {
  if (!session.draft) return null
  const handoff = buildPwaDraftCliCompactHandoff(session.draft)
  const changedPropertyCount = handoff.elements.reduce((total, element) => total + element.changedProperties.length, 0)
  const bindingGapCount = handoff.elements.filter((element) => element.binding.needsBinding).length
  return (
    <div className={styles.pwaHandoffSummary} data-testid="pwa-low-token-handoff">
      <strong>低 Token 交接</strong>
      <span>改动元素 {handoff.elements.length} 个</span>
      <span>样式属性 {changedPropertyCount} 项</span>
      <span>候选源码 {handoff.sourceFilesToInspect.length} 个</span>
      <small>{bindingGapCount > 0
        ? `AI 只补 ${bindingGapCount} 个绑定缺口，优先读取 compactHandoff。`
        : 'AI 按 compactHandoff 写回，不读整仓库。'}</small>
    </div>
  )
}

function StylePresetBar({ session }: Props) {
  return (
    <section className={styles.pwaPresetSection}>
      <h3>快速草稿</h3>
      <div className={styles.pwaPresetGrid}>
        {STYLE_PRESETS.map((preset) => (
          <button
            key={preset.label}
            type="button"
            title={preset.hint}
            onClick={() => session.updateStyles(`preset:${preset.label}`, preset.styles)}
          >
            <strong>{preset.label}</strong>
            <span>{preset.hint}</span>
          </button>
        ))}
      </div>
      <small>预设只改当前元素草稿；确认后写回验证。</small>
    </section>
  )
}

function quickValue(session: PwaDesignSession, property: PwaStyleProperty, delta: number, fallback: number, unit = 'px', min = 0, max?: number): string {
  return adjustedCssValue(fieldValue(session, property), delta, unit, min, max) || `${fallback}${unit}`
}

function DesignerQuickActions({ session }: Props) {
  const nudgeBox = (label: string, delta: number) => session.updateStyles(`designer:${label}`, {
    width: quickValue(session, 'width', delta * 4, delta > 0 ? 4 : 0, 'px', 1),
    height: quickValue(session, 'height', delta * 4, delta > 0 ? 4 : 0, 'px', 1),
    paddingTop: quickValue(session, 'paddingTop', delta * 2, delta > 0 ? 2 : 0),
    paddingRight: quickValue(session, 'paddingRight', delta * 2, delta > 0 ? 2 : 0),
    paddingBottom: quickValue(session, 'paddingBottom', delta * 2, delta > 0 ? 2 : 0),
    paddingLeft: quickValue(session, 'paddingLeft', delta * 2, delta > 0 ? 2 : 0),
  })
  return (
    <section className={styles.pwaDesignerQuickSection} aria-label="设计师常用微调">
      <h3>设计师常用微调</h3>
      <div className={styles.pwaDesignerQuickGrid}>
        <button type="button" onClick={() => nudgeBox('smaller', -1)}><strong>变小</strong><span>尺寸和内边距一起收紧</span></button>
        <button type="button" onClick={() => nudgeBox('larger', 1)}><strong>变大</strong><span>尺寸和内边距一起放大</span></button>
        <button type="button" onClick={() => session.updateStyle('borderRadius', quickValue(session, 'borderRadius', 2, 12))}><strong>更圆</strong><span>圆角 +2px</span></button>
        <button type="button" onClick={() => session.updateStyle('borderRadius', quickValue(session, 'borderRadius', -2, 0))}><strong>少圆</strong><span>圆角 -2px</span></button>
        <button type="button" onClick={() => session.updateStyle('fontSize', quickValue(session, 'fontSize', 1, 14, 'px', 8))}><strong>字大</strong><span>字号 +1px</span></button>
        <button type="button" onClick={() => session.updateStyle('fontSize', quickValue(session, 'fontSize', -1, 14, 'px', 8))}><strong>字小</strong><span>字号 -1px</span></button>
        <button type="button" onClick={() => session.updateStyles('designer:primary', { backgroundColor: '#2563eb', color: '#ffffff', borderRadius: '14px' })}><strong>主按钮</strong><span>蓝底白字</span></button>
        <button type="button" onClick={() => session.updateStyles('designer:ghost', { backgroundColor: 'transparent', color: '#e5e7eb' })}><strong>透明底</strong><span>保留文字层级</span></button>
      </div>
      <small>直接改真实 PWA DOM，并进入写回计划。</small>
    </section>
  )
}

export function PwaStyleInspector({ session }: Props) {
  const selectedDraft = session.selection
    ? Object.values(session.draft?.elements ?? {}).find((element) => (
        element.identity.key === session.selection?.identity.key
        || element.identity.selector === session.selection?.identity.selector
      ))
    : null
  const elementCount = Object.keys(session.draft?.elements ?? {}).length
  return (
    <aside className={styles.pwaStyleInspector} data-testid="pwa-style-inspector">
      <header>
        <div>
          <strong>PWA 手工样式</strong>
          <small>直接修改真实 DOM，页面立即重绘</small>
        </div>
        <span className={styles.pwaDraftCount}>{elementCount} 个元素</span>
      </header>

      <div className={styles.pwaHistoryBar}>
        <button type="button" disabled={!session.canUndo} onClick={session.undo} title="撤销上一事务"><Undo2 size={15} />撤销</button>
        <button type="button" disabled={!session.canRedo} onClick={session.redo} title="重做上一事务"><Redo2 size={15} />重做</button>
        <button type="button" disabled={!session.draft} onClick={session.saveNow}><Save size={15} />保存草稿</button>
      </div>
      <p className={styles.pwaSaveStatus}>{session.saveLabel}</p>

      <PwaDesignerWorkflowGuide session={session} />

      <BridgeHealthCard session={session} />

      <section className={styles.pwaDesignModeCard} data-mode={session.mode} data-selected={session.selection ? 'true' : 'false'}>
        <div>
          <strong>{designModeTitle(session)}</strong>
          <span>{designModeHint(session)}</span>
        </div>
        <div className={styles.pwaDesignModeActions}>
          <button type="button" disabled={!session.ready || session.mode === 'interact'} onClick={() => session.setMode('interact')}>
            <Smartphone size={14} />操作页面
          </button>
          <button type="button" disabled={!session.ready || session.mode === 'select'} onClick={() => session.setMode('select')}>
            <MousePointer2 size={14} />选择组件
          </button>
        </div>
      </section>

      <section className={styles.pwaSyncCard} data-sync-phase={session.syncState.phase}>
        <strong>{syncPhaseLabel(session.syncState.phase)}</strong>
        <SyncProgress session={session} />
        <div className={styles.pwaSyncTargets}>
          <span>PWA：{writebackTargetLabel(session.writebackPlan.targets.pwa)}</span>
          <span>APK：{writebackTargetLabel(session.writebackPlan.targets.android)}</span>
          <strong>{session.writebackPlan.requiresCodex ? '确定性优先，AI 只补缺口' : '无需 AI 重做'}</strong>
        </div>
        <WritebackPlanSummary session={session} />
        <button
          type="button"
          className={styles.pwaPrimarySync}
          data-testid="pwa-cross-platform-sync"
          disabled={!elementCount || session.syncState.phase === 'BUILD_VERIFYING' || Boolean(session.syncState.taskId)}
          onClick={() => { void session.syncNow() }}
        >
          <Bot size={16} />{session.syncState.phase === 'BUILD_VERIFYING'
            ? '正在构建并核验真实源码…'
            : session.syncState.phase === 'AI_WRITING'
              ? 'AI 正在写回源码…'
            : session.writebackPlan.requiresCodex
              ? '让 AI 建立绑定并验证 APK 与 PWA'
              : '写回源码并验证 APK 与 PWA'}
        </button>
        <p className={styles.pwaSyncStatus}>{session.syncState.message}</p>
        <HandoffSummary session={session} />
        <CrossPlatformWritebackReceiptPanel receipt={session.writebackReceipt} />
        {session.syncState.runtimeCapture && <p className={styles.pwaSyncStatus}>
          PNG 证据：{session.syncState.runtimeCapture.width}×{session.syncState.runtimeCapture.height}
          {' · '}<code>{session.syncState.runtimeCapture.sha256.slice(0, 16)}</code>
          {' · '}{session.syncState.runtimeCapture.path}
        </p>}
        {session.syncState.runtimeCaptureDiagnostic && <p className={styles.pwaSyncStatus}>
          {session.syncState.runtimeCaptureDiagnostic.code}：{session.syncState.runtimeCaptureDiagnostic.nextStep}
        </p>}
        {session.syncState.mismatches.length > 0 && <ul>
          {session.syncState.mismatches.map((mismatch) => <li key={mismatch}>{mismatch}</li>)}
        </ul>}
        {session.syncState.phase === 'VERIFY_FAILED' && session.syncState.evidence && (
          <button type="button" onClick={() => { void session.retryVerification() }}>保留草稿并重试真实验证</button>
        )}
        <div className={styles.pwaArtifactActions}>
          <button type="button" disabled={!elementCount} onClick={() => { void session.copyCliPackage() }}><Copy size={13} />复制 CLI 包</button>
          <button type="button" disabled={!elementCount} onClick={session.downloadCliPackage}><Download size={13} />下载草稿</button>
        </div>
      </section>

      {!session.selection && (
        <div className={styles.pwaInspectorEmpty}>
          <strong>{session.mode === 'interact' ? '先正常使用 PWA 到目标页面' : '点击左侧页面中的真实组件'}</strong>
          <span>{session.mode === 'interact' ? '到目标页后点“选择组件”。' : '选中后右侧可调样式。'}</span>
          <button
            type="button"
            className={styles.pwaEmptyPrimaryAction}
            disabled={!session.ready}
            onClick={() => session.setMode(session.mode === 'interact' ? 'select' : 'interact')}
          >
            {session.mode === 'interact'
              ? <><MousePointer2 size={15} />开始选择组件</>
              : <><Smartphone size={15} />返回正常操作</>}
          </button>
          {!session.ready && <small>真实 PWA 连接完成后会自动启用选择。</small>}
        </div>
      )}

      {session.selection && <>
        <section className={styles.pwaSelectedIdentity}>
          <div><strong>{session.selection.identity.ariaLabel || session.selection.identity.text || session.selection.identity.id || session.selection.identity.tag}</strong><span className={`${styles.pwaConfidence} ${styles[`pwaConfidence_${session.selection.identity.confidence}`]}`}>{confidenceLabel(session.selection)}</span></div>
          <code title={session.selection.identity.selector}>{session.selection.identity.selector}</code>
          <div className={styles.pwaBindingSummary}>
            <span>{bindingLabel(selectedDraft?.binding.status)}</span>
            <span>置信度：{selectedDraft?.binding.bindingConfidence ?? session.selection.identity.confidence}</span>
          </div>
          <p>PWA 候选 {selectedDraft?.binding.pwaCandidates.length ?? 0} 个 · Android 候选 {selectedDraft?.binding.androidCandidates.length ?? 0} 个</p>
          {(selectedDraft?.binding.needsBinding ?? session.selection.identity.needsBinding) && <p>需要 AI 建立源码绑定。</p>}
        </section>

        <DesignerQuickActions session={session} />

        <StylePresetBar session={session} />

        <section className={styles.pwaStyleSection}>
          <h3>尺寸</h3>
          <div className={styles.pwaTwoColumn}>{SIZE_FIELDS.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
      <small>支持 auto/%/rem/px。</small>
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>内边距</h3>
          <EdgeFields session={session} fields={PADDING_FIELDS} />
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>外边距</h3>
          <EdgeFields session={session} fields={MARGIN_FIELDS} />
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>形状与文字</h3>
          <div className={styles.pwaTwoColumn}>{TYPE_FIELDS.map((spec) => <StyleField key={spec.property} session={session} spec={spec} />)}</div>
        </section>

        <section className={styles.pwaStyleSection}>
          <h3>颜色与透明度</h3>
          <StyleField session={session} spec={{ property: 'color', label: '文字色' }} />
          <StyleField session={session} spec={{ property: 'backgroundColor', label: '背景色' }} />
          <label className={styles.pwaOpacityField}>
            <span>透明度</span>
            <input type="range" min="0" max="1" step="0.01" value={Number(fieldValue(session, 'opacity')) || 0} onChange={(event) => session.updateStyle('opacity', event.currentTarget.value)} />
            <input value={fieldValue(session, 'opacity')} onChange={(event) => session.updateStyle('opacity', event.currentTarget.value)} />
          </label>
        </section>

        <div className={styles.pwaResetActions}>
          <button type="button" disabled={!selectedDraft} onClick={session.resetCurrent}><RotateCcw size={15} />重置当前元素</button>
          <button type="button" disabled={!elementCount} onClick={session.clearPage}><Trash2 size={15} />清空本页草稿</button>
        </div>
      </>}
    </aside>
  )
}
