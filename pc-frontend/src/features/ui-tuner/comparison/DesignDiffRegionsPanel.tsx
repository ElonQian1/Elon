import { ScanSearch, Sparkles } from 'lucide-react'
import { useState } from 'react'
import type { DesignDiffRegion, DesignDiffRegionAnalysis } from './autoPairApi'
import { analyzeDesignDiffRegions } from './autoPairApi'
import styles from './UiTunerComparisonWorkspace.module.css'

interface DesignDiffRegionsPanelProps {
  sessionId?: string
  targetReady: boolean
  onChooseRegion: (region: DesignDiffRegion) => void
  onAnalysisChange?: (analysis: DesignDiffRegionAnalysis | null) => void
}

export function DesignDiffRegionsPanel({
  sessionId,
  targetReady,
  onChooseRegion,
  onAnalysisChange,
}: DesignDiffRegionsPanelProps) {
  const [analysis, setAnalysis] = useState<DesignDiffRegionAnalysis | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const analyze = async () => {
    if (!sessionId) return
    setLoading(true)
    setError('')
    try {
      const next = await analyzeDesignDiffRegions(sessionId)
      setAnalysis(next)
      onAnalysisChange?.(next)
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : '设计稿差异识别失败')
    } finally {
      setLoading(false)
    }
  }

  return (
    <section className={styles.autoPairPanel} aria-label="PS设计稿自动差异配对">
      <header>
        <div>
          <strong><ScanSearch size={13} aria-hidden="true" />PS 改动识别</strong>
          <span>原始真机截图 → 设计稿 → 真实节点</span>
        </div>
        <button
          type="button"
          disabled={!sessionId || !targetReady || loading}
          onClick={() => { void analyze() }}
        >
          <Sparkles size={13} aria-hidden="true" />
          {loading ? '正在分析…' : '自动识别差异'}
        </button>
      </header>
      {error && <p className={styles.autoPairError}>{error}</p>}
      {analysis && (
        <>
          <div className={styles.autoPairSummary}>
            <span>发现 {analysis.regions.length} 个改动区域</span>
            <span>变化像素 {(analysis.changedPixelRatio * 100).toFixed(2)}%</span>
            <span>缩放 {analysis.scaleX.toFixed(3)} × {analysis.scaleY.toFixed(3)}</span>
          </div>
          {analysis.regions.length === 0 ? (
            <p className={styles.autoPairEmpty}>没有发现超过阈值的设计变化。</p>
          ) : (
            <div className={styles.autoPairRegions}>
              {analysis.regions.map((region, index) => {
                const candidate = region.candidates.find((item) => (
                  item.runtimeNodeId === region.recommendedRuntimeNodeId
                )) ?? region.candidates[0]
                return (
                  <button
                    type="button"
                    key={region.id}
                    disabled={!candidate}
                    onClick={() => onChooseRegion(region)}
                    title={candidate?.definitionId ?? '没有可匹配节点'}
                  >
                    <strong>区域 {index + 1}</strong>
                    <span>{rectLabel(region.targetRect)}</span>
                    <span>{candidate ? nodeLabel(candidate.kind, candidate.text) : '未找到真实节点'}</span>
                    <em>{Math.round(region.confidence * 100)}%</em>
                  </button>
                )
              })}
            </div>
          )}
        </>
      )}
    </section>
  )
}

function rectLabel(rect: DesignDiffRegion['targetRect']) {
  return `${rect.right - rect.left} × ${rect.bottom - rect.top} · ${rect.left}, ${rect.top}`
}

function nodeLabel(kind: string, text?: string) {
  return text?.trim() ? `${kind} · ${text.trim()}` : kind
}
