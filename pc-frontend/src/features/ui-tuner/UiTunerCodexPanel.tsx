import { useMemo, useState } from 'react'
import { Clipboard, FileCode2, MessageSquareCode } from 'lucide-react'
import type { UiTunerFilterResult } from './filtering'
import { stringifyStandardPackage, type UiTunerStandardInsight } from './standards'
import type { UiTunerDocument, UiTunerElement } from './types'
import type { MetricItem } from './uiTunerGeometry'
import { summarizeClosurePriorities } from './closurePriorities'
import {
  buildUiTunerCodexContextPack,
  buildUiTunerCodexTaskPrompt,
  stringifyUiTunerCodexContextPack,
} from './contextPack'
import { UiTunerProjectSessionPanel } from './UiTunerProjectSessionPanel'
import pageStyles from './UiTunerPage.module.css'
import panelStyles from './UiTunerPanels.module.css'

interface UiTunerCodexPanelProps {
  tunerDoc: UiTunerDocument
  selected: UiTunerElement
  metrics: MetricItem[]
  filterResult: UiTunerFilterResult
  standardInsight: UiTunerStandardInsight | null
}

const DEFAULT_INTENT = '把这个节点调整成可复用的 APK UI 美观标准，并说明应该保存到 token、组件标准还是当前页面覆盖。'

export function UiTunerCodexPanel({
  tunerDoc,
  selected,
  metrics,
  filterResult,
  standardInsight,
}: UiTunerCodexPanelProps) {
  const [intent, setIntent] = useState(DEFAULT_INTENT)
  const [copyState, setCopyState] = useState('')
  const [fallbackText, setFallbackText] = useState('')
  const pack = useMemo(() => buildUiTunerCodexContextPack({
    document: tunerDoc,
    selected,
    metrics,
    filterResult,
    standardInsight,
  }), [filterResult, metrics, selected, standardInsight, tunerDoc])
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

      <UiTunerProjectSessionPanel pack={pack} intent={intent} />

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
