import { useCallback, useEffect, useState } from 'react'
import {
  applyDesignSourcePatch,
  checkDesignSourceBinding,
  createDesignRegressionBaseline,
  decideDesignSourcePatch,
  decideDesignWritebackPlan,
  getDesignSourcePatch,
  getDesignIntentPlan,
  getDesignRegressionBaseline,
  getDesignRegressionComparison,
  planDesignIntent,
  planDesignRegressionComparison,
  planDesignSourceRollback,
  planDesignWriteback,
  replanDesignIntent,
  runDesignRegressionComparison,
  startDesignIntentPlan,
  transitionDesignIntentPlan,
} from './designPlanningApi'
import type {
  DesignBindingHealth,
  DesignIntentPlan,
  DesignRegressionBaseline,
  DesignRegressionComparison,
  DesignSourcePatchProposal,
  DesignSourceRollbackPlan,
  DesignWritebackPlan,
} from './designPlanningTypes'
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
  const [sourcePatch, setSourcePatch] = useState<DesignSourcePatchProposal | null>(null)
  const [rollbackPlan, setRollbackPlan] = useState<DesignSourceRollbackPlan | null>(null)
  const [regressionBaseline, setRegressionBaseline] = useState<DesignRegressionBaseline | null>(null)
  const [regressionComparison, setRegressionComparison] = useState<DesignRegressionComparison | null>(null)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const [error, setError] = useState('')

  useEffect(() => {
    setBindingHealth(null)
    setWritebackPlan(null)
    setSourcePatch(null)
    setRollbackPlan(null)
  }, [input.draft?.draftId, input.draft?.revision])

  useEffect(() => {
    setIntentPlan(null)
    setRegressionBaseline(null)
    setRegressionComparison(null)
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
    const target = input.session?.designSessionId
      ? { designSessionId: input.session.designSessionId }
      : { platform: input.platform, route: input.route }
    const reusablePlan = intentPlan && !['RUNNING', 'SUPERSEDED'].includes(intentPlan.status)
    const result = reusablePlan
      ? await replanDesignIntent({
        projectRoot: input.projectRoot,
        planId: intentPlan.planId,
        expectedRevision: intentPlan.revision,
        intent,
        ...target,
      })
      : await planDesignIntent({
        projectRoot: input.projectRoot,
        intent,
        ...target,
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
  }), [input.onPlan, input.platform, input.projectRoot, input.route, input.session?.designSessionId, intentPlan, run])

  const refreshIntentPlan = useCallback(async () => {
    if (!intentPlan?.planId) return null
    try {
      const result = await getDesignIntentPlan(input.projectRoot, intentPlan.planId)
      setIntentPlan(result.plan)
      return result.plan
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计计划刷新失败')
      return null
    }
  }, [input.projectRoot, intentPlan?.planId])

  const startIntentPlan = useCallback(async (taskId: string, designSessionId?: string) => {
    if (!intentPlan || intentPlan.status !== 'PLANNED' || !designSessionId) return null
    return run('intent-start', async () => {
      const result = await startDesignIntentPlan({
        projectRoot: input.projectRoot,
        planId: intentPlan.planId,
        expectedRevision: intentPlan.revision,
        taskId,
        designSessionId,
        ...(input.draft?.draftId ? { draftId: input.draft.draftId } : {}),
        leaseSeconds: 900,
      })
      setIntentPlan(result.plan)
      setMessage(`计划已开始执行 · r${result.plan.revision}`)
      return result.plan
    })
  }, [input.draft?.draftId, input.projectRoot, intentPlan, run])

  const transitionIntentPlan = useCallback(async (
    transition: 'PAUSE' | 'RESUME' | 'CANCEL' | 'FAIL' | 'COMPLETE',
    reason?: string,
  ) => {
    if (!intentPlan) return null
    return run(`intent-${transition.toLowerCase()}`, async () => {
      const result = await transitionDesignIntentPlan({
        projectRoot: input.projectRoot,
        planId: intentPlan.planId,
        expectedRevision: intentPlan.revision,
        transition,
        reason,
      })
      setIntentPlan(result.plan)
      setMessage(`计划状态：${result.plan.status} · r${result.plan.revision}`)
      return result.plan
    })
  }, [input.projectRoot, intentPlan, run])

  const settleIntentPlan = useCallback(async (succeeded?: boolean) => {
    if (!intentPlan) return null
    return run('intent-settle', async () => {
      const refreshed = await getDesignIntentPlan(input.projectRoot, intentPlan.planId)
      setIntentPlan(refreshed.plan)
      if (refreshed.plan.status !== 'RUNNING') return refreshed.plan
      const receiptsSettled = refreshed.plan.actionReceipts.every((receipt) => (
        receipt.status === 'SUCCEEDED' || receipt.status === 'SKIPPED'
      ))
      const transition = succeeded === false ? 'FAIL' : receiptsSettled ? 'COMPLETE' : 'PAUSE'
      const reason = succeeded === false
        ? '关联 AI 任务执行失败'
        : receiptsSettled ? undefined : 'AI 任务已结束，但仍有动作回执未结算'
      const result = await transitionDesignIntentPlan({
        projectRoot: input.projectRoot,
        planId: refreshed.plan.planId,
        expectedRevision: refreshed.plan.revision,
        transition,
        reason,
      })
      setIntentPlan(result.plan)
      setMessage(`AI 任务已结算，计划状态：${result.plan.status}`)
      return result.plan
    })
  }, [input.projectRoot, intentPlan, run])

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

  const loadSourcePatch = useCallback(async (proposalId: string) => {
    try {
      const result = await getDesignSourcePatch(input.projectRoot, proposalId)
      setSourcePatch(result.proposal)
      return result.proposal
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '源码补丁提案读取失败')
      return null
    }
  }, [input.projectRoot])

  const decideSourcePatch = useCallback(async (decision: 'APPROVE' | 'REJECT', reason?: string) => {
    if (!sourcePatch) return null
    return run('source-patch-decision', async () => {
      const result = await decideDesignSourcePatch({
        projectRoot: input.projectRoot,
        proposalId: sourcePatch.proposalId,
        expectedRevision: sourcePatch.revision,
        decision,
        reason,
      })
      setSourcePatch(result.proposal)
      setMessage(decision === 'APPROVE' ? '确定性源码补丁已批准，等待用户应用' : '源码补丁已拒绝')
      return result.proposal
    })
  }, [input.projectRoot, run, sourcePatch])

  const applySourcePatch = useCallback(async () => {
    if (!sourcePatch) return null
    return run('source-patch-apply', async () => {
      const result = await applyDesignSourcePatch({
        projectRoot: input.projectRoot,
        proposalId: sourcePatch.proposalId,
        expectedRevision: sourcePatch.revision,
      })
      setSourcePatch(result.proposal)
      setMessage(`源码补丁状态：${result.proposal.status}`)
      return result.proposal
    })
  }, [input.projectRoot, run, sourcePatch])

  const compileRollbackPlan = useCallback(async () => {
    if (!sourcePatch) return null
    return run('source-rollback', async () => {
      const result = await planDesignSourceRollback({
        projectRoot: input.projectRoot,
        proposalId: sourcePatch.proposalId,
        expectedRevision: sourcePatch.revision,
      })
      setRollbackPlan(result.rollback)
      setMessage('已生成可审查回滚计划；尚未修改源码')
      return result.rollback
    })
  }, [input.projectRoot, run, sourcePatch])

  const createRegressionBaseline = useCallback(async () => {
    if (!input.session) return null
    return run('regression-baseline', async () => {
      const result = await createDesignRegressionBaseline({
        projectRoot: input.projectRoot,
        designSessionId: input.session!.designSessionId,
        ...(input.draft?.draftId ? { draftId: input.draft.draftId } : {}),
        label: `${input.platform.toUpperCase()} ${input.route} 修改前`,
      })
      setRegressionBaseline(result.baseline)
      setRegressionComparison(null)
      setMessage('已固化当前 PNG/UI tree 为修改前基线')
      return result.baseline
    })
  }, [input.draft?.draftId, input.platform, input.projectRoot, input.route, input.session, run])

  const planRegressionComparison = useCallback(async () => {
    if (!input.session || !regressionBaseline) return null
    return run('regression-comparison', async () => {
      const result = await planDesignRegressionComparison({
        projectRoot: input.projectRoot,
        baselineId: regressionBaseline.baselineId,
        afterDesignSessionId: input.session!.designSessionId,
        changedSelectors: input.draft?.selector ? [input.draft.selector] : [],
        thresholds: {
          maxPixelDiffRatio: 0.01,
          maxMissingSelectors: 0,
          maxChangedSelectors: 0,
          requireSameViewport: true,
        },
      })
      setRegressionComparison(result.comparison)
      setMessage('已创建视觉/语义比较任务；等待后台比较器提交证据')
      return result.comparison
    })
  }, [input.draft?.selector, input.projectRoot, input.session, regressionBaseline, run])

  const loadRegressionBaseline = useCallback(async (baselineId: string) => {
    try {
      const result = await getDesignRegressionBaseline(input.projectRoot, baselineId)
      setRegressionBaseline(result.baseline)
      return result.baseline
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计回归基线读取失败')
      return null
    }
  }, [input.projectRoot])

  const loadRegressionComparison = useCallback(async (comparisonId: string) => {
    try {
      const result = await getDesignRegressionComparison(input.projectRoot, comparisonId)
      setRegressionComparison(result.comparison)
      return result.comparison
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : '设计回归比较任务读取失败')
      return null
    }
  }, [input.projectRoot])

  const runRegressionComparison = useCallback(async () => {
    if (!regressionComparison || regressionComparison.status !== 'READY_TO_COMPARE') return null
    return run('regression-comparator', async () => {
      const result = await runDesignRegressionComparison({
        projectRoot: input.projectRoot,
        comparisonId: regressionComparison.comparisonId,
        expectedRevision: regressionComparison.revision,
      })
      setRegressionComparison(result.comparison)
      setMessage(result.comparison.status === 'PASSED'
        ? '节点本机视觉/语义比较通过，diff artifact 已按哈希固化'
        : '节点本机视觉/语义比较未通过，请检查差异证据')
      return result.comparison
    })
  }, [input.projectRoot, regressionComparison, run])

  return {
    intentPlan,
    bindingHealth,
    writebackPlan,
    sourcePatch,
    rollbackPlan,
    regressionBaseline,
    regressionComparison,
    busyAction,
    message,
    error,
    planIntent,
    refreshIntentPlan,
    startIntentPlan,
    transitionIntentPlan,
    settleIntentPlan,
    checkBinding,
    compileWritebackPlan,
    decideWriteback,
    loadSourcePatch,
    decideSourcePatch,
    applySourcePatch,
    compileRollbackPlan,
    createRegressionBaseline,
    planRegressionComparison,
    runRegressionComparison,
    loadRegressionBaseline,
    loadRegressionComparison,
  }
}

export type DesignPlanningControlsModel = ReturnType<typeof useDesignPlanningControls>
