import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { Clipboard, FileCode2, MessageSquareCode } from 'lucide-react'
import type { UiTunerFilterResult } from './filtering'
import { stringifyStandardPackage, type UiTunerStandardInsight } from './standards'
import type { UiTunerDocument, UiTunerElement } from './types'
import type { UiTunerSelectionScope } from './types'
import type { MetricItem } from './uiTunerGeometry'
import { summarizeClosurePriorities } from './closurePriorities'
import {
  buildUiTunerCodexContextPack,
  buildUiTunerCodexTaskPrompt,
  stringifyUiTunerCodexContextPack,
} from './contextPack'
import { UiTunerProjectSessionPanel } from './UiTunerProjectSessionPanel'
import { buildSelectionVisualContext, type UiTunerSelectionVisualContext } from './runtime/selectionArtifact'
import pageStyles from './UiTunerPage.module.css'
import panelStyles from './UiTunerPanels.module.css'
import contextStyles from './UiTunerContext.module.css'
import type { UiTunerVerificationReport } from './runtime/verification'
import type { useLiveUiSession } from './live/useLiveUiSession'

interface UiTunerCodexPanelProps {
  tunerDoc: UiTunerDocument
  selected: UiTunerElement
  metrics: MetricItem[]
  filterResult: UiTunerFilterResult
  standardInsight: UiTunerStandardInsight | null
  verificationReport: UiTunerVerificationReport | null
  onMutationTaskStarted: (pack: ReturnType<typeof buildUiTunerCodexContextPack>) => Promise<void> | void
  onRequestVerification: () => void
  liveUi: ReturnType<typeof useLiveUiSession>
}

const DEFAULT_INTENT = '把这个节点调整成可复用的 APK UI 美观标准，并说明应该保存到 token、组件标准还是当前页面覆盖。'

export function UiTunerCodexPanel({
  tunerDoc,
  selected,
  metrics,
  filterResult,
  standardInsight,
  verificationReport,
  onMutationTaskStarted,
  onRequestVerification,
  liveUi,
}: UiTunerCodexPanelProps) {
  const [intent, setIntent] = useState(DEFAULT_INTENT)
  const [copyState, setCopyState] = useState('')
  const [fallbackText, setFallbackText] = useState('')
  const repeatGroup = useMemo(
    () => filterResult.repeatGroups.find((group) => group.memberIds.includes(selected.id)) ?? null,
    [filterResult.repeatGroups, selected.id],
  )
  const [selectionScope, setSelectionScope] = useState<UiTunerSelectionScope>(repeatGroup ? 'component' : 'instance')
  const [selectionVisual, setSelectionVisual] = useState<UiTunerSelectionVisualContext | null>(null)
  const liveContext = useMemo(() => ({
    sessionId: liveUi.session?.id,
    uiIrRevision: liveUi.uiIr?.revision,
    treeRevision: liveUi.session?.treeRevision,
    runtimeNodeId: liveUi.selectedNode?.runtimeNodeId,
    definitionId: liveUi.selectedNode?.definitionId,
    mcpConfigPath: liveUi.mcp?.configPath,
    targetDesign: liveUi.targetDesign ? {
      path: liveUi.targetDesign.path,
      sha256: liveUi.targetDesign.sha256,
      width: liveUi.targetDesign.width,
      height: liveUi.targetDesign.height,
    } : undefined,
  }), [
    liveUi.mcp?.configPath,
    liveUi.selectedNode?.definitionId,
    liveUi.selectedNode?.runtimeNodeId,
    liveUi.session?.id,
    liveUi.session?.treeRevision,
    liveUi.targetDesign,
    liveUi.uiIr?.revision,
  ])

  useEffect(() => {
    setSelectionScope(repeatGroup ? 'component' : 'instance')
  }, [repeatGroup?.id, selected.id])

  useEffect(() => {
    let canceled = false
    setSelectionVisual(null)
    const timer = window.setTimeout(() => {
      void buildSelectionVisualContext(tunerDoc, selected).then((visual) => {
        if (!canceled) setSelectionVisual(visual)
      })
    }, 240)
    return () => {
      canceled = true
      window.clearTimeout(timer)
    }
  }, [selected, tunerDoc])

  const pack = useMemo(() => buildUiTunerCodexContextPack({
    document: tunerDoc,
    selected,
    metrics,
    filterResult,
    standardInsight,
    selectionScope,
    repeatGroup,
    selectionVisual,
    liveContext,
  }), [filterResult, liveContext, metrics, repeatGroup, selected, selectionScope, selectionVisual, standardInsight, tunerDoc])
  const prompt = useMemo(() => buildUiTunerCodexTaskPrompt(pack, intent), [intent, pack])
  const stages = useMemo(() => summarizeClosurePriorities(), [])
  const binding = pack.runtimeBinding

  async function copy(label: string, value: string) {
    try {
      await navigator.clipboard.writeText(value)
      setFallbackText('')
      setCopyState(`${label}已复制`)
    } catch {
      if (copyViaTextArea(value)) {
        setFallbackText('')
        setCopyState(`${label}已复制`)
        return
      }
      setFallbackText(value)
      setCopyState('复制失败，已展开手动复制文本')
    }
  }

  return (
    <section className={pageStyles.section}>
      <div className={pageStyles.sectionHeader}>
        <h2>Codex 闭环</h2>
        <button
          type="button"
          title="复制 Codex 任务提示"
          onClick={() => void copy('Codex 任务提示', prompt)}
        >
          <MessageSquareCode size={14} aria-hidden="true" />
        </button>
      </div>

      <div className={panelStyles.codexTarget}>
        <span>当前目标</span>
        <strong>{selected.name}</strong>
        <small>
          {binding.resourceId ?? binding.sourceToken ?? selected.id}
          {binding.bindingConfidence ? `\n绑定置信度：${binding.bindingConfidence}` : ''}
          {binding.bindingReason ? `\n${binding.bindingReason}` : ''}
        </small>
      </div>

      <div className={contextStyles.selectionContext}>
        <div className={contextStyles.selectionPreview}>
          {selectionVisual?.previewDataUrl ? (
            <img src={selectionVisual.previewDataUrl} alt={`当前选中：${selected.name}`} />
          ) : (
            <span>{tunerDoc.canvas.referenceImage ? '正在生成选区预览…' : '当前画布没有真机截图'}</span>
          )}
        </div>
        <div className={contextStyles.selectionFacts}>
          <strong>已锁定当前元素</strong>
          <span>{selected.runtime?.resourceId ?? selected.source?.componentKey ?? selected.id}</span>
          <small>
            {selectionVisual?.artifact?.cropPath
              ? '选区截图和节点上下文会自动发送给 Codex'
              : selectionVisual?.error ?? 'XML、源码候选和当前调节值会自动发送'}
          </small>
        </div>
      </div>

      <div className={contextStyles.scopePicker}>
        <span>修改范围</span>
        <div>
          <ScopeButton scope="instance" current={selectionScope} onChange={setSelectionScope}>仅此实例</ScopeButton>
          <ScopeButton
            scope="component"
            current={selectionScope}
            onChange={setSelectionScope}
            disabled={!repeatGroup && !selected.source?.componentKey}
          >
            同类组件{repeatGroup ? ` × ${repeatGroup.count}` : ''}
          </ScopeButton>
          <ScopeButton scope="screen" current={selectionScope} onChange={setSelectionScope}>当前页面</ScopeButton>
          <ScopeButton scope="project" current={selectionScope} onChange={setSelectionScope}>全项目标准</ScopeButton>
        </div>
      </div>

      <label className={panelStyles.codexIntent}>
        <span>给 Codex 的修改意图</span>
        <textarea value={intent} onChange={(event) => setIntent(event.currentTarget.value)} />
      </label>

      <div className={panelStyles.codexActions}>
        <button
          type="button"
          onClick={() => void copy('上下文 JSON', stringifyUiTunerCodexContextPack({
            document: tunerDoc,
            selected,
            metrics,
            filterResult,
            standardInsight,
            selectionScope,
            repeatGroup,
            selectionVisual,
            liveContext,
          }))}
        >
          <Clipboard size={14} aria-hidden="true" />
          上下文
        </button>
        <button type="button" onClick={() => void copy('任务提示', prompt)}>
          <MessageSquareCode size={14} aria-hidden="true" />
          任务提示
        </button>
        <button type="button" onClick={() => void copy('标准配置', stringifyStandardPackage(tunerDoc, selected))}>
          <FileCode2 size={14} aria-hidden="true" />
          标准配置
        </button>
      </div>

      {copyState && <p className={panelStyles.codexCopyState}>{copyState}</p>}
      {fallbackText && (
        <textarea className={panelStyles.codexFallback} value={fallbackText} readOnly />
      )}

      <UiTunerProjectSessionPanel
        pack={pack}
        intent={intent}
        onMutationTaskStarted={onMutationTaskStarted}
        onTaskSettled={onRequestVerification}
      />

      {verificationReport && (
        <div className={contextStyles.verificationPanel} data-verification-phase={verificationReport.phase}>
          <div>
            <strong>{verificationTitle(verificationReport.phase)}</strong>
            <span>{verificationReport.message}</span>
          </div>
          {(verificationReport.beforePreviewDataUrl || verificationReport.afterPreviewDataUrl) && (
            <div className={contextStyles.verificationImages}>
              {verificationReport.beforePreviewDataUrl && (
                <figure><img src={verificationReport.beforePreviewDataUrl} alt="修改前选区" /><figcaption>修改前</figcaption></figure>
              )}
              {verificationReport.afterPreviewDataUrl && (
                <figure><img src={verificationReport.afterPreviewDataUrl} alt="修改后选区" /><figcaption>修改后</figcaption></figure>
              )}
            </div>
          )}
          {verificationReport.visualChangePercent !== undefined && (
            <small>视觉变化 {verificationReport.visualChangePercent.toFixed(2)}%</small>
          )}
          {verificationReport.retryable && (
            <button type="button" onClick={onRequestVerification}>重新采集验收</button>
          )}
        </div>
      )}

      <div className={panelStyles.codexContract}>
        <span>Codex 必须输出</span>
        {pack.codexContract.acceptance.map((item) => <strong key={item}>{item}</strong>)}
      </div>

      <div className={panelStyles.prioritySummary}>
        <span>P0-P65 配置化闭环</span>
        {stages.map((stage) => (
          <div key={stage.stage}>
            <strong>{stage.range}</strong>
            <small>{stage.label} · {stage.automationTarget}</small>
          </div>
        ))}
      </div>
    </section>
  )
}

function verificationTitle(phase: UiTunerVerificationReport['phase']) {
  if (phase === 'waiting_codex') return '等待 Codex 完成'
  if (phase === 'capturing') return '正在真机验收'
  if (phase === 'passed') return '真机验收通过'
  if (phase === 'review') return '需要人工确认'
  if (phase === 'failed') return '验收未完成'
  return '真机验收'
}

function ScopeButton({
  scope,
  current,
  onChange,
  disabled,
  children,
}: {
  scope: UiTunerSelectionScope
  current: UiTunerSelectionScope
  onChange: (scope: UiTunerSelectionScope) => void
  disabled?: boolean
  children: ReactNode
}) {
  return (
    <button
      type="button"
      className={current === scope ? contextStyles.activeScope : ''}
      disabled={disabled}
      onClick={() => onChange(scope)}
    >
      {children}
    </button>
  )
}

function copyViaTextArea(value: string) {
  if (typeof document === 'undefined') return false
  const textarea = document.createElement('textarea')
  textarea.value = value
  textarea.setAttribute('readonly', 'true')
  textarea.style.position = 'fixed'
  textarea.style.left = '-9999px'
  textarea.style.top = '0'
  document.body.appendChild(textarea)
  textarea.select()
  let copied = false
  try {
    copied = document.execCommand('copy')
  } catch {
    copied = false
  } finally {
    document.body.removeChild(textarea)
  }
  return copied
}
