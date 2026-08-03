import { useCallback, useEffect, useState } from 'react'
import {
  captureTauriBehavior,
  getDesignCapabilities,
  getDesignVerificationMatrix,
  interactDesignBrowser,
  prepareDesignBrowser,
  previewDesignDraft,
  restoreDesignDraftPreview,
  stopDesignBrowser,
  suggestDesignSourceBinding,
  updateDesignDraft,
} from './designSessionApi'
import type {
  DesignBrowserInteraction,
  DesignBrowserResult,
  DesignCapabilities,
  DesignDraft,
  DesignDraftPreviewResult,
  DesignSessionIdentity,
  DesignSourceBindingCandidate,
  DesignVerificationMatrix,
  TauriBehaviorEvidence,
} from './types'

interface Input {
  active: boolean
  projectRoot: string
  session: DesignSessionIdentity | null
  draft: DesignDraft | null
  initialTauriBehavior: TauriBehaviorEvidence | null
  onEvidenceChanged: () => Promise<unknown>
}

export function useDesignRuntimeControls(input: Input) {
  const [capabilities, setCapabilities] = useState<DesignCapabilities | null>(null)
  const [browserResult, setBrowserResult] = useState<DesignBrowserResult | null>(null)
  const [tauriBehavior, setTauriBehavior] = useState<TauriBehaviorEvidence | null>(null)
  const [verificationMatrix, setVerificationMatrix] = useState<DesignVerificationMatrix | null>(null)
  const [draftPreview, setDraftPreview] = useState<DesignDraftPreviewResult | null>(null)
  const [bindingCandidates, setBindingCandidates] = useState<DesignSourceBindingCandidate[]>([])
  const [fixtureProfile, setFixtureProfile] = useState('')
  const [activeFixtureProfile, setActiveFixtureProfile] = useState('')
  const [fixtureKey, setFixtureKey] = useState('')
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  const refreshCapabilities = useCallback(async () => {
    if (!input.projectRoot) return
    try {
      const result = await getDesignCapabilities(input.projectRoot)
      setCapabilities(result)
      setError('')
    } catch (reason) {
      setCapabilities(null)
      setError(reason instanceof Error ? reason.message : '节点能力清单读取失败')
    }
  }, [input.projectRoot])

  const refreshMatrix = useCallback(async () => {
    if (!input.projectRoot || !input.draft) {
      setVerificationMatrix(null)
      return
    }
    try {
      setVerificationMatrix(await getDesignVerificationMatrix(input.projectRoot, input.draft.draftId))
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '验证矩阵读取失败')
    }
  }, [input.draft, input.projectRoot])

  useEffect(() => {
    if (input.active) void refreshCapabilities()
  }, [input.active, refreshCapabilities])

  useEffect(() => {
    if (input.active) void refreshMatrix()
  }, [input.active, refreshMatrix])

  useEffect(() => {
    setBrowserResult(null)
    setTauriBehavior(null)
    setActiveFixtureProfile('')
    setDraftPreview(null)
    setBindingCandidates([])
    setMessage('')
  }, [input.session?.designSessionId])

  useEffect(() => {
    if (input.initialTauriBehavior) setTauriBehavior(input.initialTauriBehavior)
  }, [input.initialTauriBehavior])

  const prepareBrowser = useCallback(async (restart = false) => {
    if (!input.session) return
    setBusyAction('browser')
    setError('')
    try {
      const profile = fixtureProfile.trim()
      const result = await prepareDesignBrowser({
        projectRoot: input.projectRoot,
        designSessionId: input.session.designSessionId,
        restart,
        fixtureProfile: profile || undefined,
      })
      assertBrowserResult(result)
      setBrowserResult(result)
      setActiveFixtureProfile(profile)
      setMessage(`持久浏览器已保留页面状态 · 操作 ${result.runtime?.operationCount ?? 0}/${result.runtime?.limits.maxOperations ?? 128}`)
      await input.onEvidenceChanged()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '持久浏览器准备失败')
    } finally {
      setBusyAction('')
    }
  }, [fixtureProfile, input])

  const interact = useCallback(async (step: DesignBrowserInteraction) => {
    if (!input.session) return
    setBusyAction('browser')
    setError('')
    try {
      const result = await interactDesignBrowser({
        projectRoot: input.projectRoot,
        designSessionId: input.session.designSessionId,
        step,
        fixtureProfile: activeFixtureProfile || undefined,
      })
      assertBrowserResult(result)
      setBrowserResult(result)
      setMessage(`已在同一页面状态中执行 ${step.action} · 操作 ${result.runtime?.operationCount ?? 0}/${result.runtime?.limits.maxOperations ?? 128}`)
      await input.onEvidenceChanged()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '持久浏览器交互失败')
    } finally {
      setBusyAction('')
    }
  }, [activeFixtureProfile, input])

  const stopBrowser = useCallback(async () => {
    if (!input.session) return
    setBusyAction('browser')
    setError('')
    try {
      const result = await stopDesignBrowser({ projectRoot: input.projectRoot, designSessionId: input.session.designSessionId })
      setBrowserResult(result)
      setActiveFixtureProfile('')
      setMessage(result.status === 'STOPPED' ? '持久浏览器已停止并回收' : '当前会话没有持久浏览器')
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '持久浏览器停止失败')
    } finally {
      setBusyAction('')
    }
  }, [input])

  const captureBehavior = useCallback(async () => {
    if (!input.session) return
    setBusyAction('tauri')
    setError('')
    try {
      const result = await captureTauriBehavior({ projectRoot: input.projectRoot, designSessionId: input.session.designSessionId })
      setTauriBehavior(result.nativeBehavior)
      setMessage(`Tauri 行为证据：菜单 ${result.nativeBehavior.menuItemCount} · 对话框 ${result.nativeBehavior.dialogCount} · command ${result.nativeBehavior.commandEventCount}`)
      await input.onEvidenceChanged()
      await refreshMatrix()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : 'Tauri 行为证据捕获失败')
    } finally {
      setBusyAction('')
    }
  }, [input, refreshMatrix])

  const previewDraft = useCallback(async () => {
    if (!input.draft) return
    setBusyAction('draft-preview')
    setError('')
    try {
      const result = await previewDesignDraft(input.projectRoot, input.draft.draftId)
      assertBrowserResult(result.capture)
      setDraftPreview(result)
      setBrowserResult(result.capture)
      setMessage(`草稿 r${result.revision} 已临时预览；源码未修改`)
      await input.onEvidenceChanged()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计草稿预览失败')
    } finally {
      setBusyAction('')
    }
  }, [input])

  const restoreDraft = useCallback(async () => {
    if (!input.draft) return
    setBusyAction('draft-preview')
    setError('')
    try {
      const result = await restoreDesignDraftPreview(input.projectRoot, input.draft.draftId)
      assertBrowserResult(result.capture)
      setDraftPreview(result)
      setBrowserResult(result.capture)
      setMessage('已恢复预览前页面样式；草稿和源码均未修改')
      await input.onEvidenceChanged()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计草稿预览恢复失败')
    } finally {
      setBusyAction('')
    }
  }, [input])

  const suggestBinding = useCallback(async () => {
    if (!input.draft) return
    setBusyAction('binding')
    setError('')
    try {
      const result = await suggestDesignSourceBinding(input.projectRoot, input.draft.draftId)
      setBindingCandidates(result.candidates)
      setMessage(result.candidates.length
        ? `已从 ${result.scan.filesInspected} 个项目文件中找到 ${result.candidates.length} 个有界候选`
        : `扫描了 ${result.scan.filesInspected} 个项目文件，未找到可靠候选`)
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '源码绑定候选读取失败')
    } finally {
      setBusyAction('')
    }
  }, [input])

  const applyBinding = useCallback(async (candidate: DesignSourceBindingCandidate, confirmed: boolean) => {
    if (!input.draft) return
    setBusyAction('binding')
    setError('')
    try {
      const result = await updateDesignDraft({
        projectRoot: input.projectRoot,
        draftId: input.draft.draftId,
        expectedRevision: input.draft.revision,
        sourceBinding: {
          ...candidate.suggestedBinding,
          status: confirmed ? 'BOUND' : 'CANDIDATE',
          reason: confirmed
            ? `已显式确认：${candidate.suggestedBinding.reason}`
            : candidate.suggestedBinding.reason,
        },
      })
      setMessage(confirmed
        ? `已将 ${candidate.file}:${candidate.line} 确认为源码绑定 r${result.draft.revision}`
        : `已采用 ${candidate.file}:${candidate.line} 作为待确认候选 r${result.draft.revision}`)
      await input.onEvidenceChanged()
      await refreshMatrix()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '源码绑定更新失败')
    } finally {
      setBusyAction('')
    }
  }, [input, refreshMatrix])

  return {
    capabilities, browserResult, tauriBehavior, verificationMatrix, draftPreview, bindingCandidates,
    draft: input.draft,
    canRefreshMatrix: Boolean(input.draft), activeFixtureProfile,
    fixtureProfile, fixtureKey, busyAction, message, error,
    setFixtureProfile, setFixtureKey, refreshCapabilities, refreshMatrix,
    prepareBrowser, interact, stopBrowser, captureBehavior,
    previewDraft, restoreDraft, suggestBinding, applyBinding,
  }
}

function assertBrowserResult(result: DesignBrowserResult) {
  if (result.ok) return
  if (result.diagnostic) throw new Error(`${result.diagnostic.code}：${result.diagnostic.nextStep}`)
  throw new Error(`持久浏览器返回 ${result.status}`)
}

export type DesignRuntimeControlsModel = ReturnType<typeof useDesignRuntimeControls>
