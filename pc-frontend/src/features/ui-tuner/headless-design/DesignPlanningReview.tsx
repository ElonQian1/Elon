import { useState } from 'react'
import type { DesignPlanningControlsModel } from './useDesignPlanningControls'
import styles from './DesignPlanningReview.module.css'

interface Props {
  model: DesignPlanningControlsModel
  hasDraft: boolean
  hasSession: boolean
}

export function DesignPlanningReview({ model, hasDraft, hasSession }: Props) {
  const [rejectionReason, setRejectionReason] = useState('')
  const [sourcePatchReason, setSourcePatchReason] = useState('')
  const plan = model.writebackPlan
  const intentPlan = model.intentPlan
  const busy = Boolean(model.busyAction)
  return (
    <section className={styles.review} aria-label="设计意图和写回审查">
      <div className={styles.summary}>
        <strong>AI 设计计划</strong>
        <span>{intentPlan
          ? `${intentPlan.primaryPlatform?.toUpperCase() ?? '待确认'} ${intentPlan.route} · ${intentPlan.status} · r${intentPlan.revision}`
          : '发送聊天后自动生成 DesignIntentPlan'}</span>
        {intentPlan?.needsClarification && <em>{intentPlan.clarifications.join('；')}</em>}
      </div>

      {intentPlan && (
        <div className={styles.lifecycle}>
          <div className={styles.receipts} aria-label="设计计划动作回执">
            {intentPlan.actionReceipts.map((receipt) => (
              <span key={receipt.order} data-status={receipt.status} title={receipt.summary ?? undefined}>
                {receipt.order} · {receipt.status}
              </span>
            ))}
          </div>
          <button type="button" disabled={busy || intentPlan.status !== 'RUNNING'} onClick={() => void model.transitionIntentPlan('PAUSE')}>暂停</button>
          <button type="button" disabled={busy || !['PAUSED', 'FAILED'].includes(intentPlan.status)} onClick={() => void model.transitionIntentPlan('RESUME')}>恢复</button>
          <button type="button" disabled={busy || !['PLANNED', 'RUNNING', 'PAUSED', 'FAILED'].includes(intentPlan.status)} onClick={() => void model.transitionIntentPlan('CANCEL', '用户从 PC 微调画布取消')}>取消</button>
        </div>
      )}

      <div className={styles.actions}>
        <button type="button" disabled={!hasDraft || busy} onClick={() => void model.checkBinding()}>检查绑定漂移</button>
        <button type="button" disabled={!hasDraft || busy} onClick={() => void model.compileWritebackPlan()}>生成写回计划</button>
        <span data-ready={model.bindingHealth?.readyForWriteback || undefined}>
          {model.bindingHealth ? `${model.bindingHealth.status} · ${model.bindingHealth.reason}` : '尚未检查源码 SHA / range'}
        </span>
      </div>

      {plan && (
        <div className={styles.plan}>
          <div>
            <strong>{plan.decision}</strong>
            <span>{plan.impact.riskLevel} 风险 · {plan.operationCount} 个操作 · {plan.items.length} 个平台项</span>
            <code>{plan.planId}</code>
          </div>
          <div className={styles.adapters}>
            {[...new Set(plan.items.map((item) => item.adapter))].map((adapter) => <span key={adapter}>{adapter}</span>)}
          </div>
          <input
            value={rejectionReason}
            onChange={(event) => setRejectionReason(event.currentTarget.value)}
            placeholder="拒绝原因（拒绝时必填）"
            aria-label="写回计划拒绝原因"
          />
          <button type="button" disabled={busy || plan.decision === 'APPROVED' || plan.impact.blockedItemCount > 0} onClick={() => void model.decideWriteback('APPROVE')}>批准</button>
          <button type="button" disabled={busy || !rejectionReason.trim()} onClick={() => void model.decideWriteback('REJECT', rejectionReason.trim())}>拒绝</button>
        </div>
      )}

      {model.sourcePatch && (
        <div className={styles.plan}>
          <div>
            <strong>{model.sourcePatch.status}</strong>
            <span>{model.sourcePatch.sourceFile} · {model.sourcePatch.edits.length} 个精确编辑 · r{model.sourcePatch.revision}</span>
            <code title="本地补丁审查产物">{model.sourcePatch.reviewArtifactPath}</code>
          </div>
          <input
            value={sourcePatchReason}
            onChange={(event) => setSourcePatchReason(event.currentTarget.value)}
            placeholder="补丁拒绝原因"
            aria-label="源码补丁拒绝原因"
          />
          <button type="button" disabled={busy || model.sourcePatch.status !== 'PROPOSED'} onClick={() => void model.decideSourcePatch('APPROVE')}>批准补丁</button>
          <button type="button" disabled={busy || model.sourcePatch.status !== 'PROPOSED' || !sourcePatchReason.trim()} onClick={() => void model.decideSourcePatch('REJECT', sourcePatchReason.trim())}>拒绝补丁</button>
          <button type="button" disabled={busy || model.sourcePatch.status !== 'APPROVED'} onClick={() => void model.applySourcePatch()}>应用源码</button>
          <button type="button" disabled={busy || model.sourcePatch.status !== 'APPLIED'} onClick={() => void model.compileRollbackPlan()}>生成回滚计划</button>
        </div>
      )}

      {model.rollbackPlan && (
        <div className={styles.summary}>
          <strong>回滚计划</strong>
          <span>{model.rollbackPlan.status} · {model.rollbackPlan.sourceFile}</span>
          <code>{model.rollbackPlan.reviewArtifactPath}</code>
        </div>
      )}

      <div className={styles.actions}>
        <button type="button" disabled={busy || !hasSession} onClick={() => void model.createRegressionBaseline()}>固化修改前基线</button>
        <button type="button" disabled={busy || !hasSession || !model.regressionBaseline} onClick={() => void model.planRegressionComparison()}>创建前后比较任务</button>
        <span>
          {model.regressionComparison
            ? `${model.regressionComparison.status} · pixel ≤ ${model.regressionComparison.thresholds.maxPixelDiffRatio}`
            : model.regressionBaseline
              ? `基线 ${model.regressionBaseline.pixels.sha256.slice(0, 12)} · ${model.regressionBaseline.route}`
              : '尚未固化修改前 PNG / UI tree'}
        </span>
      </div>

      <small className={model.error ? styles.error : ''}>{model.error || model.message || '规划和审批不会直接修改源码'}</small>
    </section>
  )
}
