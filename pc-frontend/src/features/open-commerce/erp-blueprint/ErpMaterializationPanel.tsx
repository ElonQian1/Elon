import { AlertTriangle, CheckCircle2, CircleDashed, GitBranch } from 'lucide-react'
import type { ErpMaterializationStatus } from './erpBlueprintTypes'
import styles from './ErpBlueprintPanel.module.css'

const labels: Record<string, string> = {
  not_planned: '尚未规划',
  awaiting_approval: '等待商户批准',
  ready_to_start: '可以启动',
  blocked_no_authorized_bot: '缺少授权节点',
  executing: '正在执行',
  execution_failed: '执行失败，可恢复',
  awaiting_materialization_evidence: '等待物化证据',
  awaiting_acceptance: '等待人工验收',
  accepted_without_manifest_evidence: '已验收，证据不完整',
  accepted_verified: '已验收并核对证据',
  canceled: '已取消',
}

export default function ErpMaterializationPanel({ status }: { status: ErpMaterializationStatus }) {
  const assignment = status.matter?.assignments
  const validEvidence = status.evidence.filter((item) => item.valid).length
  const complete = status.state === 'accepted_verified' && status.blockers.length === 0
  const warning = status.blockers.length > 0

  return (
    <section className={styles.band}>
      <header>
        {complete ? <CheckCircle2 size={17} /> : warning ? <AlertTriangle size={17} /> : <CircleDashed size={17} />}
        <h3>商户项目初始化</h3>
        <span className={styles.statusBadge} data-state={complete ? 'complete' : warning ? 'warning' : 'active'}>
          {labels[status.state] ?? status.state}
        </span>
      </header>
      <div className={styles.materializationGrid}>
        <div>
          <span>固定版本</span>
          <strong>{status.contract.source.blueprint_key} v{status.contract.source.version}</strong>
          <small><GitBranch size={12} />{status.contract.source.git_commit.slice(0, 12)}</small>
        </div>
        <div>
          <span>配置修订</span>
          <strong>{status.contract.configuration.revision}</strong>
          <small>{status.contract.target_onboarding_mode === 'existing_project' ? '已有项目纳入' : '蓝图新建'} · {status.contract.configuration.theme_key}</small>
        </div>
        <div>
          <span>任务进度</span>
          <strong>{assignment ? `${assignment.completed}/${assignment.total}` : '0/0'}</strong>
          <small>{assignment ? `运行 ${assignment.running} · 失败 ${assignment.failed}` : '尚未生成 Assignment'}</small>
        </div>
        <div>
          <span>有效证据</span>
          <strong>{validEvidence}</strong>
          <small>{status.contract.required_artifact.instance_manifest_path}</small>
        </div>
      </div>
      {status.blockers.length > 0 && (
        <div className={styles.blockerList}>
          {status.blockers.map((item) => <p key={item}>{item}</p>)}
          {assignment?.failed_assignment_ids.map((item) => <p key={item}>失败 Assignment：{item}</p>)}
        </div>
      )}
      <p className={styles.nextAction}>{status.next_action}</p>
      {status.matter && (
        <p className={styles.mutedLine}>
          Matter {status.matter.id} · {status.matter.status} · 合同{status.matter.plan_contract_matches ? '一致' : '待补齐'}
        </p>
      )}
    </section>
  )
}
