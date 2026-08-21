import { useId, useLayoutEffect, useRef, useState } from 'react'
import { LoaderCircle, RefreshCw, ShieldCheck, TriangleAlert } from 'lucide-react'
import {
  type FederationHistoricalLineageScope,
  type ValidatedFederationHistoricalLineageRead,
  validateFederationHistoricalLineagePair,
} from '../compute-attempt/federationHistoricalLineageContracts'
import {
  type ValidatedSettlementReleaseLineageRead,
  validateFederationHistoricalLineageTriple,
} from '../compute-attempt/federationHistoricalReleaseLineageContracts'
import {
  type ValidatedExecutionVerificationLineageRead,
  validateExecutionVerificationLineagePair,
} from '../compute-attempt/federationHistoricalVerificationLineageContracts'
import {
  type ValidatedVerificationDecisionRead,
  validateVerificationDecisionLineage,
} from '../compute-attempt/verificationDecisionReadContracts'
import { federationHistoricalLineageApi } from './federationHistoricalLineageApi'
import styles from './FederationHistoricalLineageButton.module.css'

interface Props {
  leaseId: string
  scope: FederationHistoricalLineageScope
  releaseAvailable: boolean
}

type AuditState =
  | { status: 'idle' }
  | { status: 'loading' }
  | { status: 'error'; message: string }
  | {
      status: 'success'
      execution: ValidatedFederationHistoricalLineageRead
      verificationDecision: ValidatedVerificationDecisionRead
      verification: ValidatedExecutionVerificationLineageRead
      settlement: ValidatedFederationHistoricalLineageRead
      release: ValidatedSettlementReleaseLineageRead | null
    }

export default function FederationHistoricalLineageButton({ leaseId, scope, releaseAvailable }: Props) {
  const [state, setState] = useState<AuditState>({ status: 'idle' })
  const requestGeneration = useRef(0)
  const busy = useRef(false)
  const panelId = useId()

  useLayoutEffect(() => {
    requestGeneration.current += 1
    busy.current = false
    setState({ status: 'idle' })
    return () => {
      requestGeneration.current += 1
      busy.current = false
    }
  }, [leaseId, releaseAvailable, scope])

  async function verify() {
    if (busy.current) return
    busy.current = true
    const generation = ++requestGeneration.current
    setState({ status: 'loading' })
    try {
      const [execution, verificationDecision, verification, settlement, release] = await Promise.all([
        federationHistoricalLineageApi.readExecution(scope, leaseId),
        federationHistoricalLineageApi.readVerificationDecision(scope, leaseId),
        federationHistoricalLineageApi.readVerification(scope, leaseId),
        federationHistoricalLineageApi.readSettlement(scope, leaseId),
        releaseAvailable
          ? federationHistoricalLineageApi.readRelease(scope, leaseId)
          : Promise.resolve(null),
      ])
      validateExecutionVerificationLineagePair(execution, verification)
      validateVerificationDecisionLineage(verificationDecision, verification)
      if (release) validateFederationHistoricalLineageTriple(execution, settlement, release)
      else validateFederationHistoricalLineagePair(execution, settlement)
      if (generation !== requestGeneration.current) return
      setState({ status: 'success', execution, verificationDecision, verification, settlement, release })
    } catch (reason) {
      if (generation !== requestGeneration.current) return
      setState({ status: 'error', message: messageOf(reason) })
    } finally {
      if (generation === requestGeneration.current) busy.current = false
    }
  }

  const expanded = state.status === 'success'
  const loading = state.status === 'loading'
  const failed = state.status === 'error'

  return (
    <div className={styles.root} data-state={state.status}>
      <button
        type="button"
        className={styles.trigger}
        onClick={() => void verify()}
        disabled={loading}
        aria-busy={loading}
        aria-expanded={expanded}
        aria-controls={expanded ? panelId : undefined}
      >
        {loading && <LoaderCircle size={14} className={styles.spinning} />}
        {failed && <RefreshCw size={14} />}
        {!loading && !failed && <ShieldCheck size={14} />}
        {loading ? '并行核验因果链' : failed ? '重试因果链核验' : expanded ? '重新核验因果链' : '核验历史因果链'}
      </button>

      {loading && (
        <p className={styles.status} role="status">
          {releaseAvailable
            ? '正在并行读取 execution、native v192、verification、settlement 与 release 只读证据…'
            : '正在并行读取 execution、native v192、verification 与 settlement 只读证据…'}
        </p>
      )}
      {failed && <p className={styles.error} role="alert"><TriangleAlert size={14} />{state.message}</p>}
      {expanded && (
        <div id={panelId} className={styles.panel} aria-live="polite">
          <div className={styles.verified}>
            <ShieldCheck size={14} />
            {state.release ? '五响应摘要与 native v192 十四项闭合等式已核验' : '四响应摘要与 native v192 十四项闭合等式已核验'}
          </div>
          <div className={styles.profiles}>
            <LineageEvidence label="Execution source" record={state.execution} />
            <VerificationDecisionEvidence record={state.verificationDecision} />
            <VerificationLineageEvidence record={state.verification} />
            <LineageEvidence label="Settlement source" record={state.settlement} />
            {state.release && <ReleaseLineageEvidence record={state.release} />}
          </div>
        </div>
      )}
    </div>
  )
}

function VerificationDecisionEvidence({ record }: { record: ValidatedVerificationDecisionRead }) {
  return (
    <section className={styles.profile}>
      <div className={styles.profileHeader}>
        <strong>Retained v192 Verification</strong>
        <span>{record.decision}</span>
      </div>
      <span className={styles.kind}>{record.schema}</span>
      <code className={styles.digest}>{record.event_digest}</code>
      <div className={styles.nativeRefs}>
        <span>verification_id</span><code>{record.verification_decision_id}</code>
        <span>verified_usage</span><code>{record.verified_usage_digest}</code>
        <span>compensable_usage</span><code>{record.compensable_usage_digest}</code>
      </div>
    </section>
  )
}

function VerificationLineageEvidence({ record }: { record: ValidatedExecutionVerificationLineageRead }) {
  return (
    <section className={styles.profile}>
      <div className={styles.profileHeader}>
        <strong>Execution verification source</strong>
        <span>{record.response.read_effect}</span>
      </div>
      <span className={styles.kind}>{record.response.lineage_kind}</span>
      <code className={styles.digest}>{record.response.lineage_digest}</code>
      <details className={styles.details}>
        <summary>查看 canonical Carrier JSON</summary>
        <pre>{record.response.canonical_carrier_json}</pre>
      </details>
    </section>
  )
}

function ReleaseLineageEvidence({ record }: { record: ValidatedSettlementReleaseLineageRead }) {
  return (
    <section className={styles.profile}>
      <div className={styles.profileHeader}>
        <strong>Settlement release source</strong>
        <span>{record.response.read_effect}</span>
      </div>
      <span className={styles.kind}>{record.response.lineage_kind}</span>
      <code className={styles.digest}>{record.response.lineage_digest}</code>
      <details className={styles.details}>
        <summary>查看 canonical Carrier JSON</summary>
        <pre>{record.response.canonical_carrier_json}</pre>
      </details>
    </section>
  )
}

function LineageEvidence({
  label,
  record,
}: {
  label: string
  record: ValidatedFederationHistoricalLineageRead
}) {
  return (
    <section className={styles.profile}>
      <div className={styles.profileHeader}>
        <strong>{label}</strong>
        <span>{record.response.read_effect}</span>
      </div>
      <span className={styles.kind}>{record.response.lineage_kind}</span>
      <code className={styles.digest}>{record.response.lineage_digest}</code>
      <details className={styles.details}>
        <summary>查看 canonical Carrier JSON</summary>
        <pre>{record.response.canonical_carrier_json}</pre>
      </details>
    </section>
  )
}

function messageOf(_reason: unknown) {
  return '历史因果链读取或核验失败，请重试。'
}
