import { useCallback, useEffect, useRef, useState } from 'react'

import { nodeApi } from '../node/localNodeApi'
import type { DocumentCatalog } from './projectDocumentModel'
import type { DocumentOrganizationTrackingRuntime } from './projectDocumentOrganizationStatus'
import { buildOrganizationPrompt, type DocumentAutomationMode } from './projectDocumentSections'
import type { useProjectDocumentOrganization } from './useProjectDocumentOrganization'
import { projectDocumentErrorMessage as errorMessage } from './projectDocumentWorkspaceHelpers'

interface AutomaticDocumentTrigger {
  trigger_id: string
  operation_id: string
  commit_sha: string
  severity: 'warning' | 'blocking'
  paths: string[]
  reasons: string[]
}

interface AutomaticTriggerResponse {
  ok: boolean
  trigger: AutomaticDocumentTrigger | null
}

interface Inputs {
  projectName: string
  catalog: DocumentCatalog | null
  canStartAi: boolean
  trackingRuntime: DocumentOrganizationTrackingRuntime
  organization: ReturnType<typeof useProjectDocumentOrganization>
  automationMode: DocumentAutomationMode
  onStartAiOrganize: (prompt: string) => Promise<{ task_id?: string } | null>
  onMessage: (message: string) => void
}

export function useProjectDocumentAiOrganizer({
  projectName,
  catalog,
  canStartAi,
  trackingRuntime,
  organization,
  automationMode,
  onStartAiOrganize,
  onMessage,
}: Inputs) {
  const [organizing, setOrganizing] = useState(false)
  const activeRef = useRef(false)
  const automaticTriggerRef = useRef('')
  const retryAfterRef = useRef(0)

  const startAiOrganize = useCallback(async (
    scopeInstruction = '',
    automaticTrigger?: AutomaticDocumentTrigger,
  ) => {
    if (!catalog || !canStartAi || activeRef.current) return false
    activeRef.current = true
    setOrganizing(true)
    onMessage('')
    let operationId: string | undefined
    let dispatched = false
    try {
      operationId = await organization.startRun(automaticTrigger?.operation_id)
      const basePrompt = buildOrganizationPrompt(
        projectName,
        catalog,
        organization.manifest,
        operationId,
        automationMode,
      )
      const response = await onStartAiOrganize(scopeInstruction
        ? `${basePrompt}\n\n本次菜单范围：${scopeInstruction}`
        : basePrompt)
      dispatched = true
      await organization.markDispatched(operationId, response?.task_id)
      if (automaticTrigger) {
        await nodeApi(
          trackingRuntime.adminUrl,
          '/api/project-docs/organization/automatic-trigger/claim',
          {
            method: 'POST',
            body: JSON.stringify({
              project_root: trackingRuntime.projectRoot,
              trigger_id: automaticTrigger.trigger_id,
              operation_id: automaticTrigger.operation_id,
            }),
          },
        )
      }
      onMessage(automaticTrigger
        ? `提交 ${automaticTrigger.commit_sha.slice(0, 8)} 的文档整理任务已自动发起。`
        : operationId
          ? 'AI 整理任务已发起；可在“AI 整理建议”分区观察 MCP 每一步。'
          : 'AI 整理任务已发起；当前运行路线不提供本机 MCP 分阶段观测。')
      return true
    } catch (error) {
      const message = errorMessage(error, '无法发起 AI 整理任务')
      if (!dispatched) await organization.markFailed(operationId, message)
      onMessage(dispatched
        ? `AI 整理任务已发出，但自动触发领取失败：${message}`
        : message)
      return false
    } finally {
      activeRef.current = false
      setOrganizing(false)
    }
  }, [
    automationMode,
    canStartAi,
    catalog,
    onMessage,
    onStartAiOrganize,
    organization,
    projectName,
    trackingRuntime.adminUrl,
    trackingRuntime.projectRoot,
  ])

  const pollAutomaticTrigger = useCallback(async () => {
    if (!trackingRuntime.enabled || !trackingRuntime.projectRoot.trim() || !catalog || !canStartAi) return
    if (activeRef.current || Date.now() < retryAfterRef.current) return
    try {
      const response = await nodeApi<AutomaticTriggerResponse>(
        trackingRuntime.adminUrl,
        '/api/project-docs/organization/automatic-trigger/pending',
        {
          method: 'POST',
          body: JSON.stringify({ project_root: trackingRuntime.projectRoot }),
        },
      )
      const trigger = response.trigger
      if (!trigger || automaticTriggerRef.current === trigger.trigger_id) return
      automaticTriggerRef.current = trigger.trigger_id
      const paths = trigger.paths.slice(0, 40).join('、')
      const reasons = trigger.reasons.slice(0, 8).join('；')
      const instruction = [
        `这是提交 ${trigger.commit_sha} 自动触发的文档治理任务，级别为 ${trigger.severity}。`,
        `只处理本次命中的文档：${paths}。`,
        reasons ? `守门原因：${reasons}。` : '',
        '先调用 project_docs_review_modularity，再按权威性与生命周期拆分当前文档；保留原始讨论材料。',
        '完成后运行文档检索测试，确认已否决或 excluded 的方案不会重新成为默认实现依据。',
        '本任务只整理项目文档及其治理元数据，不修改业务源码。',
      ].filter(Boolean).join('\n')
      const started = await startAiOrganize(instruction, trigger)
      if (!started) retryAfterRef.current = Date.now() + 60_000
    } catch {
      retryAfterRef.current = Date.now() + 30_000
    } finally {
      automaticTriggerRef.current = ''
    }
  }, [
    canStartAi,
    catalog,
    startAiOrganize,
    trackingRuntime.adminUrl,
    trackingRuntime.enabled,
    trackingRuntime.projectRoot,
  ])

  useEffect(() => {
    void pollAutomaticTrigger()
    const timer = window.setInterval(() => { void pollAutomaticTrigger() }, 5_000)
    return () => window.clearInterval(timer)
  }, [pollAutomaticTrigger])

  return { organizing, startAiOrganize }
}
