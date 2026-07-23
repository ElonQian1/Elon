import { useCallback, useEffect, useRef, useState } from 'react'
import type { LiveBuildVerifyResult } from '../live/liveUiApi'
import {
  completeCrossPlatformWritebackReceipt,
  type CrossPlatformWritebackReceipt,
  type PlatformReceiptUpdate,
} from './crossPlatformWritebackReceipt'
import type { PwaVerificationState } from './pwaVerificationModel'

export interface AndroidWritebackVerification {
  sessionId: string
  deviceId: string
  verify: () => Promise<LiveBuildVerifyResult>
}

interface Options {
  androidVerification?: AndroidWritebackVerification
  pwaState: PwaVerificationState
  onComplete: () => void
  onError: (message: string) => void
  onStatus: (message: string) => void
}

export function useCrossPlatformWritebackReceipt({
  androidVerification,
  pwaState,
  onComplete,
  onError,
  onStatus,
}: Options) {
  const [receipt, setReceipt] = useState<CrossPlatformWritebackReceipt | null>(null)
  const receiptRef = useRef<CrossPlatformWritebackReceipt | null>(null)
  const updateQueueRef = useRef<Promise<unknown>>(Promise.resolve())
  const pwaVerifiedKeyRef = useRef('')

  const remember = useCallback((next: CrossPlatformWritebackReceipt | null) => {
    receiptRef.current = next
    setReceipt(next)
  }, [])

  const reset = useCallback(() => {
    pwaVerifiedKeyRef.current = ''
    remember(null)
  }, [remember])

  const update = useCallback((
    platformResults: Partial<Record<'pwa' | 'apk', PlatformReceiptUpdate>>,
  ): Promise<CrossPlatformWritebackReceipt> => {
    const run = async () => {
      const current = receiptRef.current
      if (!current) throw new Error('缺少写回 receiptId，不能更新双端证据')
      const next = await completeCrossPlatformWritebackReceipt({
        receiptId: current.receiptId,
        projectRoot: current.projectRoot,
        platformResults,
      })
      remember(next)
      if (next.complete) onComplete()
      return next
    }
    const queued = updateQueueRef.current.then(run, run)
    updateQueueRef.current = queued.then(() => undefined, () => undefined)
    return queued
  }, [onComplete, remember])

  const verifyAndroid = useCallback(async (source: CrossPlatformWritebackReceipt) => {
    const platform = source.platformResults.apk
    if (!platform || !platform.changedFiles.length || platform.status !== 'SAVED') return
    if (!androidVerification) {
      onStatus('PWA 源码可继续验证；APK 已保存，等待连接真实 Android Runtime')
      return
    }
    await update({
      apk: {
        status: 'BUILD_VERIFYING',
        method: platform.method,
        changedFiles: platform.changedFiles,
        sourceRevisions: platform.sourceRevisions,
      },
    })
    try {
      const build = await androidVerification.verify()
      const verified = build.status === 'BUILD_VERIFIED' && build.runtimeConnected
      await update({
        apk: {
          status: verified ? 'BUILD_VERIFIED' : 'FAILED',
          method: platform.method,
          changedFiles: platform.changedFiles,
          sourceRevisions: platform.sourceRevisions,
          buildEvidence: androidBuildEvidence(build, androidVerification, receiptRef.current),
          error: verified ? undefined : build.message,
        },
      })
    } catch (error) {
      await update({
        apk: {
          status: 'FAILED',
          method: platform.method,
          changedFiles: platform.changedFiles,
          sourceRevisions: platform.sourceRevisions,
          error: error instanceof Error ? error.message : 'APK 构建/运行验证失败',
        },
      })
    }
  }, [androidVerification, onStatus, update])

  useEffect(() => {
    const evidence = pwaState.evidence
    const platform = receipt?.platformResults.pwa
    if (
      pwaState.phase !== 'BUILD_VERIFIED'
      || pwaState.runtimeCapturePending
      || !pwaState.build
      || !pwaState.snapshot
      || !evidence
      || !platform
      || !['SAVED', 'BUILD_VERIFYING'].includes(platform.status)
    ) return
    const key = `${receipt.receiptId}:${evidence.requestId}`
    if (pwaVerifiedKeyRef.current === key) return
    pwaVerifiedKeyRef.current = key
    void update({
      pwa: {
        status: 'BUILD_VERIFIED',
        method: platform.method,
        changedFiles: platform.changedFiles,
        sourceRevisions: platform.sourceRevisions,
        buildEvidence: {
          status: 'BUILD_VERIFIED',
          sourceRevision: receipt.sourceRevision,
          runtimeReloaded: true,
          routeRevision: `pwa-draft-r${evidence.draftRevision}`,
          requestId: evidence.requestId,
          buildCommand: pwaState.build.buildCommand,
          buildDurationMs: pwaState.build.buildDurationMs,
          resourceFiles: pwaState.build.resourceFiles,
          runtimeCapture: pwaState.runtimeCapture,
          runtimeCaptureDiagnostic: pwaState.runtimeCaptureDiagnostic,
        },
      },
    }).catch((error) => {
      onError(error instanceof Error ? error.message : 'PWA 机器回执更新失败')
    })
  }, [onError, pwaState, receipt, update])

  return {
    receipt,
    receiptRef,
    remember,
    reset,
    update,
    verifyAndroid,
  }
}

function androidBuildEvidence(
  build: LiveBuildVerifyResult,
  verification: AndroidWritebackVerification,
  receipt: CrossPlatformWritebackReceipt | null,
) {
  return {
    status: build.status,
    sourceRevision: receipt?.sourceRevision,
    runtimeConnected: build.runtimeConnected,
    runtimeBuildId: build.runtimeBuildId,
    deviceId: verification.deviceId,
    sessionId: verification.sessionId,
    apkPath: build.apkPath,
    buildDurationMs: build.buildDurationMs,
    nodeCount: build.nodeCount,
    screenshotWidth: build.screenshotWidth,
    screenshotHeight: build.screenshotHeight,
    verificationGate: build.verificationGate,
  }
}
