import { useCallback, useEffect, useState } from 'react'
import {
  captureTauriBehavior,
  getDesignCapabilities,
  getDesignVerificationMatrix,
  interactDesignBrowser,
  prepareDesignBrowser,
  stopDesignBrowser,
} from './designSessionApi'
import type {
  DesignBrowserInteraction,
  DesignBrowserResult,
  DesignCapabilities,
  DesignDraft,
  DesignSessionIdentity,
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

  return {
    capabilities, browserResult, tauriBehavior, verificationMatrix,
    canRefreshMatrix: Boolean(input.draft), activeFixtureProfile,
    fixtureProfile, fixtureKey, busyAction, message, error,
    setFixtureProfile, setFixtureKey, refreshCapabilities, refreshMatrix,
    prepareBrowser, interact, stopBrowser, captureBehavior,
  }
}

function assertBrowserResult(result: DesignBrowserResult) {
  if (result.ok) return
  if (result.diagnostic) throw new Error(`${result.diagnostic.code}：${result.diagnostic.nextStep}`)
  throw new Error(`持久浏览器返回 ${result.status}`)
}

export type DesignRuntimeControlsModel = ReturnType<typeof useDesignRuntimeControls>
