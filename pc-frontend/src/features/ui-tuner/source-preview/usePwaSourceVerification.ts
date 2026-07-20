import { useCallback, useRef, useState } from 'react'
import { capturePwaSourceRuntime, verifyPwaSourceBuild } from './sourcePreviewApi'
import {
  completePwaVerification,
  completePwaRuntimeCapture,
  livePwaVerificationState,
  pwaAiWritingState,
  pwaBuildVerifyingState,
  pwaSourceSavedState,
  pwaRuntimeCapturePendingState,
  pwaVerifyFailedState,
  type PwaBridgeVerificationSnapshot,
  type PwaBuildVerificationResult,
  type PwaSourceSavedEvidence,
  type PwaVerificationState,
} from './pwaVerificationModel'

interface UsePwaSourceVerificationOptions {
  post: (type: string, payload: unknown) => void
  reloadSource: () => void
  restoreDraft: () => void
  clearVerifiedDraft: () => void
  runtimeUrl: string
}

interface PendingVerification {
  evidence: PwaSourceSavedEvidence
  build?: PwaBuildVerificationResult
  snapshotRequested: boolean
}

export function usePwaSourceVerification({
  post,
  reloadSource,
  restoreDraft,
  clearVerifiedDraft,
  runtimeUrl,
}: UsePwaSourceVerificationOptions) {
  const [state, setState] = useState<PwaVerificationState>(() => livePwaVerificationState())
  const stateRef = useRef(state)
  const pendingRef = useRef<PendingVerification | null>(null)
  const timeoutRef = useRef(0)

  const update = useCallback((next: PwaVerificationState) => {
    stateRef.current = next
    setState(next)
  }, [])

  const clearTimeout = useCallback(() => {
    if (timeoutRef.current) window.clearTimeout(timeoutRef.current)
    timeoutRef.current = 0
  }, [])

  const fail = useCallback((message: string, mismatches: string[] = [], build?: PwaBuildVerificationResult) => {
    clearTimeout()
    const next = pwaVerifyFailedState(stateRef.current, message, mismatches, build)
    update(next)
    restoreDraft()
  }, [clearTimeout, restoreDraft, update])

  const markLive = useCallback((message?: string) => {
    clearTimeout()
    pendingRef.current = null
    update(livePwaVerificationState(message))
  }, [clearTimeout, update])

  const markSourceSaved = useCallback((
    evidence: PwaSourceSavedEvidence | undefined,
    message?: string,
    taskId?: string,
  ) => {
    clearTimeout()
    pendingRef.current = evidence ? { evidence, snapshotRequested: false } : null
    update(pwaSourceSavedState(stateRef.current, evidence, message, taskId))
  }, [clearTimeout, update])

  const markAiWriting = useCallback((taskId: string, message?: string) => {
    clearTimeout()
    pendingRef.current = null
    update(pwaAiWritingState(stateRef.current, taskId, message))
  }, [clearTimeout, update])

  const start = useCallback(async (evidence?: PwaSourceSavedEvidence) => {
    const selected = evidence ?? pendingRef.current?.evidence ?? stateRef.current.evidence
    if (!selected) {
      fail('缺少可复核的 PWA source revision 与 changed files；已保留草稿')
      return
    }
    const saved = pwaSourceSavedState(stateRef.current, selected)
    pendingRef.current = { evidence: selected, snapshotRequested: false }
    update(saved)
    post('reset-styles', {})
    await new Promise<void>((resolve) => window.setTimeout(resolve, 0))
    const verifying = pwaBuildVerifyingState(saved)
    update(verifying)
    let build: PwaBuildVerificationResult
    try {
      build = await verifyPwaSourceBuild(selected)
    } catch (error) {
      fail(error instanceof Error ? error.message : 'PWA 构建验证请求失败')
      return
    }
    if (!build.ok || build.status !== 'BUILD_VERIFIED') {
      fail(build.message || 'PWA 前端构建或资源验证失败', [], build)
      return
    }
    pendingRef.current = { evidence: selected, build, snapshotRequested: false }
    update({ ...verifying, build, message: '构建与资源已通过；正在清除缓存并重载真实源码画面…' })
    reloadSource()
    clearTimeout()
    timeoutRef.current = window.setTimeout(() => {
      fail('真实源码 iframe 重载或样式回传超时；临时草稿已恢复')
    }, 20_000)
  }, [clearTimeout, fail, post, reloadSource, update])

  const onIframeReady = useCallback(() => {
    const pending = pendingRef.current
    if (!pending?.build) return false
    if (!pending.snapshotRequested) {
      pending.snapshotRequested = true
      post('verify-source', {
        requestId: pending.evidence.requestId,
        checks: pending.evidence.checks.map((check) => ({
          elementKey: check.elementKey,
          selector: check.selector,
          properties: Object.keys(check.styles),
        })),
      })
    }
    return true
  }, [post])

  const handleSnapshot = useCallback((snapshot: PwaBridgeVerificationSnapshot) => {
    const pending = pendingRef.current
    if (!pending?.build || snapshot.requestId !== pending.evidence.requestId) return false
    clearTimeout()
    const next = completePwaVerification(stateRef.current, pending.build, snapshot)
    update(next)
    if (next.phase === 'BUILD_VERIFIED') {
      pendingRef.current = null
      clearVerifiedDraft()
      const capturing = pwaRuntimeCapturePendingState(next)
      update(capturing)
      void capturePwaSourceRuntime(pending.evidence, runtimeUrl).then((capture) => {
        if (stateRef.current.evidence?.requestId !== pending.evidence.requestId) return
        update(completePwaRuntimeCapture(stateRef.current, capture))
      }).catch((error) => {
        if (stateRef.current.evidence?.requestId !== pending.evidence.requestId) return
        update(completePwaRuntimeCapture(stateRef.current, {
          ok: false,
          status: 'CAPTURE_FAILED',
          base64Embedded: false,
          diagnostic: {
            code: 'CAPTURE_REQUEST_FAILED',
            message: error instanceof Error ? error.message : 'PC 节点 PWA PNG 请求失败',
            retryable: true,
            nextStep: '确认本机节点、PWA URL 与浏览器后显式重试',
          },
        }))
      })
    } else {
      restoreDraft()
    }
    return true
  }, [clearTimeout, clearVerifiedDraft, restoreDraft, runtimeUrl, update])

  const retry = useCallback(() => start(), [start])

  return {
    state,
    markLive,
    markAiWriting,
    markSourceSaved,
    fail,
    start,
    retry,
    onIframeReady,
    handleSnapshot,
  }
}
