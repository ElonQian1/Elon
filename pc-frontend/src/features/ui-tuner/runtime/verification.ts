import type { UiTunerCodexContextPack } from '../contextPack'
import type { AndroidInspectorSnapshot } from '../device/deviceInspectorApi'
import type { UiTunerDocument, UiTunerElement } from '../types'
import { snapshotToTunerDocument } from './snapshotToTunerDocument'
import { analyzeRepeatComponents } from './repeatComponents'
import { cropUiTunerSelectionPreview } from './selectionArtifact'

export type UiTunerVerificationPhase =
  | 'idle'
  | 'waiting_codex'
  | 'capturing'
  | 'passed'
  | 'review'
  | 'failed'

export interface UiTunerVerificationBaseline {
  document: UiTunerDocument
  selected: UiTunerElement
  pack: UiTunerCodexContextPack
  beforePreviewDataUrl?: string
  startedAt: string
}

export interface UiTunerVerificationReport {
  phase: UiTunerVerificationPhase
  message: string
  finishedAt?: string
  matchedElementId?: string
  beforePreviewDataUrl?: string
  afterPreviewDataUrl?: string
  visualChangePercent?: number
  requestedAdjustmentCount: number
  verifiedAdjustmentCount: number
  beforeRepeatCount?: number
  afterRepeatCount?: number
  retryable: boolean
}

export async function createVerificationBaseline(
  document: UiTunerDocument,
  selected: UiTunerElement,
  pack: UiTunerCodexContextPack,
): Promise<UiTunerVerificationBaseline> {
  let beforePreviewDataUrl: string | undefined
  try {
    beforePreviewDataUrl = await cropUiTunerSelectionPreview(document, selected)
  } catch {
    beforePreviewDataUrl = undefined
  }
  return {
    document,
    selected,
    pack,
    beforePreviewDataUrl,
    startedAt: new Date().toISOString(),
  }
}

export async function verifyPostChangeSnapshot(
  baseline: UiTunerVerificationBaseline,
  snapshot: AndroidInspectorSnapshot,
): Promise<{ report: UiTunerVerificationReport; document: UiTunerDocument }> {
  const document = snapshotToTunerDocument(snapshot)
  const matched = findMatchingElement(baseline, document)
  if (!matched) {
    return {
      document,
      report: {
        phase: 'failed',
        message: '重新采集成功，但没有找到原组件。请回到原页面后重试验收。',
        finishedAt: new Date().toISOString(),
        beforePreviewDataUrl: baseline.beforePreviewDataUrl,
        requestedAdjustmentCount: baseline.pack.requestedAdjustments.length,
        verifiedAdjustmentCount: 0,
        retryable: true,
      },
    }
  }

  let afterPreviewDataUrl: string | undefined
  try {
    afterPreviewDataUrl = await cropUiTunerSelectionPreview(document, matched)
  } catch {
    afterPreviewDataUrl = undefined
  }
  const visualChangePercent = baseline.beforePreviewDataUrl && afterPreviewDataUrl
    ? await imageDifferencePercent(baseline.beforePreviewDataUrl, afterPreviewDataUrl)
    : undefined
  const verifiedAdjustmentCount = verifyRequestedGeometry(baseline.pack, matched)
  const repeatBefore = baseline.pack.repeatedComponent?.count
  const repeatAfter = analyzeRepeatComponents(document.elements).groupByElementId[matched.id]?.count
  const requestedCount = baseline.pack.requestedAdjustments.length
  const verifiableCount = baseline.pack.requestedAdjustments.filter((adjustment) => (
    ['x', 'y', 'width', 'height'].includes(adjustment.property)
  )).length
  const geometrySatisfied = verifiableCount > 0 && verifiedAdjustmentCount === verifiableCount
  const phase: UiTunerVerificationPhase = geometrySatisfied ? 'passed' : 'review'
  const message = phase === 'passed'
    ? `已重新捕获并找到同一组件；视觉变化 ${formatPercent(visualChangePercent)}。`
    : `已重新捕获并找到同一组件；视觉变化 ${formatPercent(visualChangePercent)}，样式变化请人工对照确认。`
  return {
    document,
    report: {
      phase,
      message,
      finishedAt: new Date().toISOString(),
      matchedElementId: matched.id,
      beforePreviewDataUrl: baseline.beforePreviewDataUrl,
      afterPreviewDataUrl,
      visualChangePercent,
      requestedAdjustmentCount: requestedCount,
      verifiedAdjustmentCount,
      beforeRepeatCount: repeatBefore,
      afterRepeatCount: repeatAfter,
      retryable: true,
    },
  }
}

export function failedVerification(message: string): UiTunerVerificationReport {
  return {
    phase: 'failed',
    message,
    finishedAt: new Date().toISOString(),
    requestedAdjustmentCount: 0,
    verifiedAdjustmentCount: 0,
    retryable: true,
  }
}

function findMatchingElement(
  baseline: UiTunerVerificationBaseline,
  document: UiTunerDocument,
) {
  const sourceComponent = baseline.selected.source?.componentKey
  const resourceId = baseline.selected.runtime?.resourceId
  const normalizedPath = normalizeXpath(baseline.selected.runtime?.xpath)
  const candidates = document.elements.filter((element) => {
    if (sourceComponent && element.source?.componentKey === sourceComponent) return true
    if (resourceId && element.runtime?.resourceId === resourceId) return true
    return normalizedPath && normalizeXpath(element.runtime?.xpath) === normalizedPath
  })
  return candidates.sort((left, right) => matchScore(baseline.selected, right) - matchScore(baseline.selected, left))[0]
}

function matchScore(before: UiTunerElement, after: UiTunerElement) {
  let score = 0
  if (before.source?.componentKey && before.source.componentKey === after.source?.componentKey) score += 50
  if (before.runtime?.resourceId && before.runtime.resourceId === after.runtime?.resourceId) score += 40
  if (normalizeXpath(before.runtime?.xpath) === normalizeXpath(after.runtime?.xpath)) score += 20
  score -= Math.abs(before.x - after.x) / 20
  score -= Math.abs(before.y - after.y) / 20
  return score
}

function verifyRequestedGeometry(pack: UiTunerCodexContextPack, after: UiTunerElement) {
  const actual: Record<string, string | number> = {
    x: after.x,
    y: after.y,
    width: after.width,
    height: after.height,
  }
  return pack.requestedAdjustments.filter((adjustment) => {
    const value = actual[adjustment.property]
    return typeof value === 'number'
      && typeof adjustment.after === 'number'
      && Math.abs(value - adjustment.after) <= 2
  }).length
}

function normalizeXpath(value?: string) {
  return value?.replace(/\[\d+\]/g, '[*]') ?? ''
}

async function imageDifferencePercent(beforeUrl: string, afterUrl: string) {
  const [before, after] = await Promise.all([loadImage(beforeUrl), loadImage(afterUrl)])
  const width = 192
  const height = 192
  const beforePixels = renderPixels(before, width, height)
  const afterPixels = renderPixels(after, width, height)
  let difference = 0
  for (let index = 0; index < beforePixels.length; index += 4) {
    difference += Math.abs(beforePixels[index] - afterPixels[index])
    difference += Math.abs(beforePixels[index + 1] - afterPixels[index + 1])
    difference += Math.abs(beforePixels[index + 2] - afterPixels[index + 2])
  }
  return Math.round((difference / ((beforePixels.length / 4) * 3 * 255)) * 10_000) / 100
}

function renderPixels(image: HTMLImageElement, width: number, height: number) {
  const canvas = window.document.createElement('canvas')
  canvas.width = width
  canvas.height = height
  const context = canvas.getContext('2d', { willReadFrequently: true })
  if (!context) throw new Error('浏览器无法读取验收截图')
  context.drawImage(image, 0, 0, width, height)
  return context.getImageData(0, 0, width, height).data
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image()
    image.onload = () => resolve(image)
    image.onerror = () => reject(new Error('验收截图读取失败'))
    image.src = url
  })
}

function formatPercent(value?: number) {
  return value === undefined ? '无法计算' : `${value.toFixed(2)}%`
}
