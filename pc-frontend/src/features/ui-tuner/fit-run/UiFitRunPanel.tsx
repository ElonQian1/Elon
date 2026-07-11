import type { useFitRun } from './useFitRun'
import { fitSourceParityPassed, fitTargetPassed } from './fitRunSelectors'
import { useFitRunCodexHandoff } from './useFitRunCodexHandoff'
import styles from './UiFitRunPanel.module.css'

interface UiFitRunPanelProps {
  fitRun: ReturnType<typeof useFitRun>
  pairReady: boolean
}

export function UiFitRunPanel({ fitRun, pairReady }: UiFitRunPanelProps) {
  const { run } = fitRun
  const codexHandoff = useFitRunCodexHandoff({ run, command: fitRun.command })
  const interactionBusy = fitRun.busy || codexHandoff.launching
  const terminal = Boolean(run && ['ACCEPTED', 'PLATEAU', 'FAILED', 'CANCELLED'].includes(run.phase))
  return (
    <section className={styles.panel} aria-label="设计稿自动拟合">
      <header>
        <div>
          <strong>自动拟合</strong>
          <span>{run ? phaseLabel(run.phase) : pairReady ? '配对就绪' : '等待左右区域配对'}</span>
        </div>
        {run?.best && <Score value={run.best.score.overallLoss} />}
      </header>

      {run ? (
        <>
          <div className={styles.metrics}>
            <Metric label="本地试探" value={`${run.usage.localEvaluations}/${run.budget.maxLocalEvaluations}`} />
            <Metric label="Codex 轮次" value={`${run.usage.codexRounds}/${run.budget.maxCodexRounds}`} />
            <Metric label="构建轮次" value={`${run.usage.buildRounds}/${run.budget.maxBuildRounds}`} />
            <Metric label="无改善" value={`${run.usage.noImprovementTrials}/${run.budget.maxNoImprovementTrials}`} />
          </div>
          {run.best && (
            <div className={styles.gates}>
              <Gate label="目标设计" passed={fitTargetPassed(run)} />
              <Gate label="源码一致" passed={fitSourceParityPassed(run)} />
              <small>只有两个门禁同时通过，任务才会成为可学习案例。</small>
            </div>
          )}
          {run.stopReason && <p className={styles.reason}>{run.stopReason}</p>}
          {run.phase === 'AWAITING_CODEX' && run.handoff && (
            <div className={styles.handoff}>
              <strong>需要 Codex 处理结构或源码</strong>
              <span>{run.handoff.reason}</span>
              <label>
                <input type="checkbox" checked={codexHandoff.autoCodex} onChange={(event) => {
                  codexHandoff.setAutoCodex(event.currentTarget.checked)
                }} />
                自动接力（受轮次和构建预算限制）
              </label>
              <button type="button" disabled={interactionBusy} onClick={() => { void codexHandoff.launch() }}>
                {codexHandoff.launching ? '正在绑定 Codex 任务…' : '让 Codex 继续拟合'}
              </button>
              {codexHandoff.error && (
                <small className={styles.error}>
                  {codexHandoff.error}
                  {codexHandoff.autoCodex && codexHandoff.retryAttempt > 0 ? ' · 将自动退避重试' : ''}
                </small>
              )}
            </div>
          )}
          <div className={styles.actions}>
            {run.phase === 'PAUSED' && <CommandButton label="继续" busy={interactionBusy} onClick={() => fitRun.command({ type: 'RESUME' })} />}
            {run.phase !== 'PAUSED' && run.phase !== 'CODEX_RUNNING' && !terminal && (
              <CommandButton label="暂停" busy={interactionBusy} onClick={() => fitRun.command({ type: 'PAUSE' })} />
            )}
            {run.phase === 'CANDIDATE_READY' && (
              <CommandButton label="确认最佳结果" busy={interactionBusy} onClick={() => fitRun.command({ type: 'ACCEPT_BEST' })} />
            )}
            {!terminal && (
              <CommandButton label="取消" busy={interactionBusy} onClick={() => fitRun.command({ type: 'CANCEL' })} />
            )}
            {terminal && (
              <CommandButton label="开始新的拟合" busy={interactionBusy} onClick={async () => { fitRun.clear() }} />
            )}
          </div>
        </>
      ) : (
        <button type="button" className={styles.primary} disabled={!fitRun.canStart || fitRun.busy} onClick={() => { void fitRun.start() }}>
          {fitRun.busy ? '正在建立基线…' : '开始自动拟合'}
        </button>
      )}
      {fitRun.error && <p className={styles.error}>{fitRun.error}</p>}
    </section>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>
}

function Gate({ label, passed }: { label: string; passed?: boolean }) {
  return <span className={passed ? styles.pass : passed === false ? styles.fail : styles.pending}>{label}</span>
}

function Score({ value }: { value: number }) {
  return <span className={styles.score}>损失 {value.toFixed(4)}</span>
}

function CommandButton({ label, busy, onClick }: { label: string; busy: boolean; onClick: () => Promise<unknown> }) {
  return <button type="button" disabled={busy} onClick={() => { void onClick() }}>{label}</button>
}

function phaseLabel(phase: string) {
  const labels: Record<string, string> = {
    CREATED: '已创建', BASELINING: '建立基线', LOCAL_SOLVING: '本地数值求解',
    AWAITING_CODEX: '等待 Codex', CODEX_RUNNING: 'Codex 修改中', REBUILDING: '重新构建',
    EVALUATING: '目标评分', CANDIDATE_READY: '候选已达标', SOURCE_VERIFYING: '源码验收',
    PAUSED: '已暂停', ACCEPTED: '双门禁通过', PLATEAU: '预算耗尽', FAILED: '失败', CANCELLED: '已取消',
  }
  return labels[phase] ?? phase
}
