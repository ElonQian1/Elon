import type { PwaDesignDraft, PwaStyleProperty } from './pwaDesignDraft'
import type { AiWritebackReceipt } from './aiWritebackReceipt'

export type PwaVerificationPhase =
  | 'LIVE_PREVIEW'
  | 'AI_WRITING'
  | 'SOURCE_SAVED'
  | 'BUILD_VERIFYING'
  | 'BUILD_VERIFIED'
  | 'VERIFY_FAILED'

export interface PwaVerificationCheck {
  elementKey: string
  selector: string
  styles: Partial<Record<PwaStyleProperty, string>>
}

export interface PwaSourceSavedEvidence {
  requestId: string
  projectRoot: string
  draftRevision: number
  route: PwaDesignDraft['route']
  viewport: PwaDesignDraft['viewport']
  changedFiles: string[]
  sourceRevisions: Record<string, string>
  expectedValues: string[]
  checks: PwaVerificationCheck[]
}

export interface PwaRuntimeCaptureArtifact {
  path: string
  manifestPath: string
  sha256: string
  width: number
  height: number
  bytes: number
  mediaType: 'image/png'
  capturedAt: string
}

export interface PwaRuntimeCaptureDiagnostic {
  code: string
  message: string
  retryable: boolean
  nextStep: string
}

export interface PwaRuntimeCaptureResult {
  ok: boolean
  status: 'CAPTURED' | 'CAPTURE_FAILED'
  artifact?: PwaRuntimeCaptureArtifact
  revision?: { sourceRevision: string; routeRevision: string }
  diagnostic?: PwaRuntimeCaptureDiagnostic
  base64Embedded: false
}

export interface PwaBuildVerificationResult {
  ok: boolean
  status: 'BUILD_VERIFIED' | 'VERIFY_FAILED'
  message: string
  sourceRevisions: Record<string, string>
  changedFiles: string[]
  buildCommand?: string
  buildDurationMs: number
  resourceFiles: string[]
  resourceValuesVerified: number
  outputTail?: string
}

export interface PwaBridgeVerificationNode {
  elementKey: string
  selector: string
  found: boolean
  computed: Partial<Record<PwaStyleProperty, string>>
  authored: Partial<Record<PwaStyleProperty, string>>
}

export interface PwaBridgeVerificationSnapshot {
  requestId: string
  route: {
    path: string
    search: string
    hash: string
    screenKey?: string
    screenTitle?: string
  }
  sourceRevision?: string
  sourceRevisions?: Record<string, string>
  changedFiles?: string[]
  nodes: PwaBridgeVerificationNode[]
}

export interface PwaVerificationState {
  phase: PwaVerificationPhase
  message: string
  evidence?: PwaSourceSavedEvidence
  build?: PwaBuildVerificationResult
  snapshot?: PwaBridgeVerificationSnapshot
  runtimeCapture?: PwaRuntimeCaptureArtifact
  runtimeCaptureDiagnostic?: PwaRuntimeCaptureDiagnostic
  runtimeCapturePending?: boolean
  mismatches: string[]
  taskId?: string
}

export function livePwaVerificationState(message = '真实 PWA 可正常使用；样式修改仍是临时预览'): PwaVerificationState {
  return { phase: 'LIVE_PREVIEW', message, mismatches: [] }
}

export function pwaAiWritingState(
  previous: PwaVerificationState,
  taskId: string,
  message = 'AI 正在补充未绑定属性或结构修改；临时草稿会一直保留',
): PwaVerificationState {
  return {
    ...previous,
    phase: 'AI_WRITING',
    message,
    taskId,
    evidence: undefined,
    mismatches: [],
    build: undefined,
    snapshot: undefined,
  }
}

export function pwaSourceSavedState(
  previous: PwaVerificationState,
  evidence: PwaSourceSavedEvidence | undefined,
  message = '源码已保存，尚未执行真实构建验证',
  taskId?: string,
): PwaVerificationState {
  return {
    ...previous, phase: 'SOURCE_SAVED', message, evidence, taskId, mismatches: [],
    build: undefined, snapshot: undefined, runtimeCapture: undefined,
    runtimeCaptureDiagnostic: undefined, runtimeCapturePending: false,
  }
}

export function pwaBuildVerifyingState(state: PwaVerificationState): PwaVerificationState {
  if (!state.evidence) return pwaVerifyFailedState(state, '缺少源码 revision 与 changed files，不能开始构建验证')
  return {
    ...state,
    phase: 'BUILD_VERIFYING',
    message: '正在核对源码哈希、运行前端构建并验证生成资源…',
    taskId: undefined,
    mismatches: [],
  }
}

export function pwaVerifyFailedState(
  state: PwaVerificationState,
  message: string,
  mismatches: string[] = [],
  build?: PwaBuildVerificationResult,
): PwaVerificationState {
  return { ...state, phase: 'VERIFY_FAILED', message, mismatches, build: build ?? state.build, taskId: undefined }
}

export function completePwaVerification(
  state: PwaVerificationState,
  build: PwaBuildVerificationResult,
  snapshot: PwaBridgeVerificationSnapshot,
): PwaVerificationState {
  const evidence = state.evidence
  if (state.phase !== 'BUILD_VERIFYING' || !evidence) {
    return pwaVerifyFailedState(state, '验证响应没有对应的 BUILD_VERIFYING 状态')
  }
  const mismatches: string[] = []
  if (!build.ok || build.status !== 'BUILD_VERIFIED') mismatches.push(`构建/资源验证失败：${build.message}`)
  if (snapshot.requestId !== evidence.requestId) mismatches.push('iframe 验证请求 ID 不一致')
  compareSourceEvidence(evidence, build, mismatches)
  compareBridgeSourceEvidence(evidence, snapshot, mismatches)
  compareRoute(evidence, snapshot, mismatches)
  compareNodes(evidence, snapshot, mismatches)
  if (mismatches.length) {
    return pwaVerifyFailedState(state, '真实源码重载结果与目标不一致，临时草稿已恢复', mismatches, build)
  }
  return {
    ...state,
    phase: 'BUILD_VERIFIED',
    message: `真实源码、前端构建、${build.resourceFiles.length} 个资源文件和 iframe 样式均验证通过`,
    build,
    snapshot,
    mismatches: [],
    taskId: undefined,
  }
}

export function pwaRuntimeCapturePendingState(state: PwaVerificationState): PwaVerificationState {
  return {
    ...state,
    runtimeCapturePending: true,
    runtimeCaptureDiagnostic: undefined,
    message: `${state.message}；正在由 PC 节点无头浏览器保存 PNG 像素证据…`,
  }
}

export function completePwaRuntimeCapture(
  state: PwaVerificationState,
  capture: PwaRuntimeCaptureResult,
): PwaVerificationState {
  if (capture.ok && capture.status === 'CAPTURED' && capture.artifact) {
    return {
      ...state,
      runtimeCapture: capture.artifact,
      runtimeCaptureDiagnostic: undefined,
      runtimeCapturePending: false,
      message: `${state.message.replace(/；正在由 PC 节点无头浏览器保存 PNG 像素证据…$/, '')}；PNG ${capture.artifact.width}×${capture.artifact.height} 已保存并关联 revision`,
    }
  }
  const diagnostic = capture.diagnostic ?? {
    code: 'CAPTURE_FAILED',
    message: 'PC 节点未返回 PNG 工件',
    retryable: true,
    nextStep: '检查本机 PWA URL、浏览器与 authProfile 后显式重试',
  }
  return {
    ...state,
    runtimeCapturePending: false,
    runtimeCaptureDiagnostic: diagnostic,
    message: `${state.message.replace(/；正在由 PC 节点无头浏览器保存 PNG 像素证据…$/, '')}；源码/iframe 已验证，PNG 待准备：${diagnostic.message}`,
  }
}

function compareBridgeSourceEvidence(
  evidence: PwaSourceSavedEvidence,
  snapshot: PwaBridgeVerificationSnapshot,
  mismatches: string[],
) {
  if (snapshot.changedFiles?.length) {
    const expected = evidence.changedFiles.join('|')
    const actual = [...snapshot.changedFiles].sort().join('|')
    if (actual !== expected) mismatches.push('真实页面声明的 changed files 与写回回执不一致')
  }
  if (snapshot.sourceRevisions && Object.keys(snapshot.sourceRevisions).length) {
    for (const file of evidence.changedFiles) {
      if (snapshot.sourceRevisions[file]?.toLowerCase() !== evidence.sourceRevisions[file]?.toLowerCase()) {
        mismatches.push(`真实页面声明的 ${file} source revision 不一致`)
      }
    }
  } else if (snapshot.sourceRevision && evidence.changedFiles.length === 1
    && snapshot.sourceRevision.toLowerCase() !== evidence.sourceRevisions[evidence.changedFiles[0]]?.toLowerCase()) {
    mismatches.push('真实页面声明的 source revision 与写回回执不一致')
  }
}

export function sourceSavedEvidenceFromDraft(
  draft: PwaDesignDraft,
  requestId: string,
): PwaSourceSavedEvidence | null {
  const checks: PwaVerificationCheck[] = []
  const revisions: Record<string, string> = {}
  const expectedValues = new Set<string>()
  for (const [elementKey, element] of Object.entries(draft.elements)) {
    const binding = element.binding.pwaStyle
    if (!binding) continue
    const styles: Partial<Record<PwaStyleProperty, string>> = {}
    for (const [propertyValue, value] of Object.entries(element.styleDiff)) {
      const property = propertyValue as PwaStyleProperty
      const receipt = element.writeback?.pwa?.[property]
      if (!receipt || receipt.value !== value || receipt.sourceFile !== binding.sourceFile) continue
      styles[property] = value
      expectedValues.add(value)
    }
    if (!Object.keys(styles).length) continue
    revisions[binding.sourceFile] = binding.sourceRevision
    checks.push({ elementKey, selector: element.identity.selector, styles })
  }
  const changedFiles = Object.keys(revisions).sort()
  if (!changedFiles.length || !checks.length || !expectedValues.size) return null
  checks.sort((left, right) => left.elementKey.localeCompare(right.elementKey))
  return {
    requestId,
    projectRoot: draft.project.workspaceIdentity,
    draftRevision: draft.revision,
    route: draft.route,
    viewport: draft.viewport,
    changedFiles,
    sourceRevisions: Object.fromEntries(changedFiles.map((file) => [file, revisions[file]])),
    expectedValues: [...expectedValues].sort(),
    checks,
  }
}

export function sourceSavedEvidenceFromAiReceipt(
  draft: PwaDesignDraft,
  receipt: AiWritebackReceipt,
  requestId: string,
): PwaSourceSavedEvidence | null {
  const pwaResult = receipt.platformResults.pwa
  if (!receipt.targetPlatforms.includes('pwa') || !pwaResult || pwaResult.status !== 'SAVED') return null
  const changedFiles = [...pwaResult.changedFiles].sort()
  if (!changedFiles.length || !pwaResult.sourceRevision) return null
  const checks: PwaVerificationCheck[] = []
  const expectedValues = new Set<string>()
  for (const [elementKey, element] of Object.entries(draft.elements)) {
    const styles: Partial<Record<PwaStyleProperty, string>> = {}
    for (const [propertyValue, value] of Object.entries(element.styleDiff)) {
      const property = propertyValue as PwaStyleProperty
      if (!value) continue
      styles[property] = value
      expectedValues.add(value)
    }
    if (!Object.keys(styles).length) continue
    checks.push({ elementKey, selector: element.identity.selector, styles })
  }
  if (!checks.length || !expectedValues.size) return null
  checks.sort((left, right) => left.elementKey.localeCompare(right.elementKey))
  return {
    requestId,
    projectRoot: draft.project.workspaceIdentity,
    draftRevision: draft.revision,
    route: draft.route,
    viewport: draft.viewport,
    changedFiles,
    sourceRevisions: Object.fromEntries(changedFiles.map((file) => [file, pwaResult.sourceRevision])),
    expectedValues: [...expectedValues].sort(),
    checks,
  }
}

function compareSourceEvidence(
  evidence: PwaSourceSavedEvidence,
  build: PwaBuildVerificationResult,
  mismatches: string[],
) {
  const expectedFiles = evidence.changedFiles.join('|')
  const actualFiles = [...build.changedFiles].sort().join('|')
  if (actualFiles !== expectedFiles) mismatches.push('构建验证返回的 changed files 不一致')
  for (const file of evidence.changedFiles) {
    if (build.sourceRevisions[file]?.toLowerCase() !== evidence.sourceRevisions[file]?.toLowerCase()) {
      mismatches.push(`${file} 的 source revision 不一致`)
    }
  }
  if (build.resourceValuesVerified < evidence.expectedValues.length) {
    mismatches.push('构建资源未覆盖全部目标样式值')
  }
}

function compareRoute(
  evidence: PwaSourceSavedEvidence,
  snapshot: PwaBridgeVerificationSnapshot,
  mismatches: string[],
) {
  const expected = routeSignature(evidence.route)
  const actual = routeSignature(snapshot.route)
  if (actual !== expected) mismatches.push(`真实画面不一致：期望 ${expected}，实际 ${actual}`)
}

function compareNodes(
  evidence: PwaSourceSavedEvidence,
  snapshot: PwaBridgeVerificationSnapshot,
  mismatches: string[],
) {
  for (const expected of evidence.checks) {
    const actual = snapshot.nodes.find((node) => node.elementKey === expected.elementKey && node.selector === expected.selector)
    if (!actual?.found) {
      mismatches.push(`${expected.elementKey} 在真实源码页面中不存在`)
      continue
    }
    for (const [propertyValue, value] of Object.entries(expected.styles)) {
      const property = propertyValue as PwaStyleProperty
      if (!styleValueMatches(value, actual.authored[property]) && !styleValueMatches(value, actual.computed[property])) {
        mismatches.push(`${expected.elementKey}.${property} 期望 ${value}，实际 ${actual.authored[property] || actual.computed[property] || '空'}`)
      }
    }
  }
}

function routeSignature(route: { path: string; search: string; hash: string; screenKey?: string }): string {
  const params = new URLSearchParams(route.search)
  params.delete('ui_tuner_preview')
  params.delete('ui_tuner_reload')
  const search = params.toString()
  return `${route.path || '/web'}${search ? `?${search}` : ''}${route.hash || ''}#${route.screenKey || 'screen:unidentified'}`
}

function styleValueMatches(expectedValue: string, actualValue: string | undefined): boolean {
  if (!actualValue) return false
  const expected = normalizedStyle(expectedValue)
  const actual = normalizedStyle(actualValue)
  if (expected === actual) return true
  const expectedColor = colorTuple(expected)
  const actualColor = colorTuple(actual)
  if (expectedColor && actualColor) return expectedColor.every((value, index) => Math.abs(value - actualColor[index]) < .01)
  const expectedNumber = expected.match(/^(-?\d+(?:\.\d+)?)(px|rem|em|%)?$/)
  const actualNumber = actual.match(/^(-?\d+(?:\.\d+)?)(px|rem|em|%)?$/)
  return Boolean(expectedNumber && actualNumber && expectedNumber[2] === actualNumber[2]
    && Math.abs(Number(expectedNumber[1]) - Number(actualNumber[1])) < .01)
}

function normalizedStyle(value: string): string {
  const normalized = value.trim().toLowerCase().replace(/\s+/g, ' ')
  if (normalized === 'normal') return '400'
  if (normalized === 'bold') return '700'
  return normalized
}

function colorTuple(value: string): number[] | null {
  if (value === 'transparent') return [0, 0, 0, 0]
  const short = value.match(/^#([\da-f])([\da-f])([\da-f])([\da-f])?$/)
  if (short) return [parseInt(short[1] + short[1], 16), parseInt(short[2] + short[2], 16), parseInt(short[3] + short[3], 16), short[4] ? parseInt(short[4] + short[4], 16) / 255 : 1]
  const hex = value.match(/^#([\da-f]{6})([\da-f]{2})?$/)
  if (hex) return [parseInt(hex[1].slice(0, 2), 16), parseInt(hex[1].slice(2, 4), 16), parseInt(hex[1].slice(4, 6), 16), hex[2] ? parseInt(hex[2], 16) / 255 : 1]
  const rgb = value.match(/^rgba?\(([^)]+)\)$/)
  if (!rgb) return null
  const values = rgb[1].split(',').map(Number)
  if (values.length < 3 || values.some(Number.isNaN)) return null
  return [values[0], values[1], values[2], values[3] ?? 1]
}
