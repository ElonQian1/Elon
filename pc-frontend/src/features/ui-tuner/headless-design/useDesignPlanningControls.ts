import { useCallback, useEffect, useState } from 'react'
import {
  checkDesignSourceBinding,
  decideDesignWritebackPlan,
  planDesignIntent,
  planDesignWriteback,
} from './designPlanningApi'
import type { DesignBindingHealth, DesignIntentPlan, DesignWritebackPlan } from './designPlanningTypes'
import type { DesignDraft, DesignPlatform, DesignSessionIdentity } from './types'

interface Input {
  projectRoot: string
  platform: DesignPlatform
  route: string
  session: DesignSessionIdentity | null
  draft: DesignDraft | null
  onPlan?: (plan: DesignIntentPlan) => Promise<void> | void
}

export function useDesignPlanningControls(input: Input) {
  const [intentPlan, setIntentPlan] = useState<DesignIntentPlan | null>(null)
  const [bindingHealth, setBindingHealth] = useState<DesignBindingHealth | null>(null)
  const [writebackPlan, setWritebackPlan] = useState<DesignWritebackPlan | null>(null)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  useEffect(() => {
    setBindingHealth(null)
    setWritebackPlan(null)
  }, [input.draft?.draftId, input.draft?.revision])

  useEffect(() => {
    setIntentPlan(null)
  }, [input.projectRoot])

  const run = useCallback(async <T,>(action: string, work: () => Promise<T>) => {
    setBusyAction(action)
    setError('')
    try {
      return await work()
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计规划请求失败')
      throw reason
    } finally {
      setBusyAction('')
    }
  }, [])

  const planIntent = useCallback(async (intent: string) => run('intent', async () => {
    const result = await planDesignIntent({
      projectRoot: input.projectRoot,
      intent,
      ...(input.session?.designSessionId
        ? { designSessionId: input.session.designSessionId }
        : { platform: input.platform, route: input.route }),
    })
    setIntentPlan(result.plan)
    try {
      await input.onPlan?.(result.plan)
    } catch (reason) {
      setError(reason instanceof Error ? `计划已保存，但画布切换失败：${reason.message}` : '计划已保存，但画布切换失败')
    }
    setMessage(`已规划 ${result.plan.primaryPlatform?.toUpperCase() ?? '待确认平台'} ${result.plan.route}`)
    const actions = result.plan.actions.map((item) => item.tool).join(' → ')
    const clarification = result.plan.needsClarification
      ? `；待确认：${result.plan.clarifications.join('；')}`
      : ''
    return [
      `后台 DesignIntentPlan：${result.plan.planId}`,
      `目标：${result.plan.primaryPlatform ?? '未确定'} ${result.plan.route}；会话策略：${result.plan.sessionAction}${clarification}`,
      `按计划工具链执行：${actions}。`,
    ].join('\n')
  }), [input.onPlan, input.platform, input.projectRoot, input.route, input.session?.designSessionId, run])

  const checkBinding = useCallback(async () => {
    if (!input.draft) return null
    return run('binding', async () => {
      const result = await checkDesignSourceBinding({
        projectRoot: input.projectRoot,
        draftId: input.draft!.draftId,
        includeRecoveryCandidates: true,
        limit: 8,
      })
      setBindingHealth(result.health)
      setMessage(result.health.reason)
      return result.health
    })
  }, [input.draft, input.projectRoot, run])

  const compileWritebackPlan = useCallback(async () => {
    if (!input.draft) return null
    return run('writeback', async () => {
      const [healthResult, planResult] = await Promise.all([
        checkDesignSourceBinding({
          projectRoot: input.projectRoot,
          draftId: input.draft!.draftId,
          includeRecoveryCandidates: false,
        }),
        planDesignWriteback(input.projectRoot, input.draft!.draftId),
      ])
      setBindingHealth(healthResult.health)
      setWritebackPlan(planResult.plan)
      setMessage(planResult.plan.impact.blockedItemCount
        ? `写回计划仍有 ${planResult.plan.impact.blockedItemCount} 个阻塞项`
        : '写回影响已整理，等待显式批准')
      return planResult.plan
    })
  }, [input.draft, input.projectRoot, run])

  const decideWriteback = useCallback(async (decision: 'APPROVE' | 'REJECT', reason?: string) => {
    if (!writebackPlan) return null
    return run('decision', async () => {
      const result = await decideDesignWritebackPlan({
        projectRoot: input.projectRoot,
        planId: writebackPlan.planId,
        expectedPlanRevision: writebackPlan.planRevision,
        decision,
        reason,
      })
      setWritebackPlan(result.plan)
      setMessage(decision === 'APPROVE' ? '写回计划已批准，可以固定源码基线' : '写回计划已拒绝')
      return result.plan
    })
  }, [input.projectRoot, run, writebackPlan])

  return {
    intentPlan,
    bindingHealth,
    writebackPlan,
    busyAction,
    message,
    error,
    planIntent,
    checkBinding,
    compileWritebackPlan,
    decideWriteback,
  }
}

export type DesignPlanningControlsModel = ReturnType<typeof useDesignPlanningControls>
