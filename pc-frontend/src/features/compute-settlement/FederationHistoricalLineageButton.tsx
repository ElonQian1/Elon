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
      const [execution, settlement, release] = await Promise.all([
        federationHistoricalLineageApi.readExecution(scope, leaseId),
        federationHistoricalLineageApi.readSettlement(scope, leaseId),
        releaseAvailable
          ? federationHistoricalLineageApi.readRelease(scope, leaseId)
          : Promise.resolve(null),
      ])
      if (release) validateFederationHistoricalLineageTriple(execution, settlement, release)
      else validateFederationHistoricalLineagePair(execution, settlement)
      if (generation !== requestGeneration.current) return
      setState({ status: 'success', execution, settlement, release })
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
            ? '正在并行读取 execution、settlement 与 release 只读证据…'
            : '正在并行读取 execution 与 settlement 只读证据…'}
        </p>
      )}
      {failed && <p className={styles.error} role="alert"><TriangleAlert size={14} />{state.message}</p>}
      {expanded && (
        <div id={panelId} className={styles.panel} aria-live="polite">
          <div className={styles.verified}>
            <ShieldCheck size={14} />
            {state.release ? '三响应摘要与两级跨链等式已核验' : '双响应摘要与跨链等式已核验'}
          </div>
          <div className={styles.profiles}>
            <LineageEvidence label="Execution source" record={state.execution} />
            <LineageEvidence label="Settlement source" record={state.settlement} />
            {state.release && <ReleaseLineageEvidence record={state.release} />}
          </div>
        </div>
      )}
    </div>
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

function messageOf(reason: unknown) {
  if (reason instanceof Error && reason.message) return reason.message
  if (reason && typeof reason === 'object' && 'message' in reason && typeof reason.message === 'string') {
    return reason.message
  }
  return '历史因果链读取或核验失败，请重试。'
}
