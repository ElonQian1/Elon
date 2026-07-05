import { Clipboard, FileDown, RefreshCw, Wrench } from 'lucide-react'
import { useMemo, useState } from 'react'
import { buildLocalDiagnosticCopy, localDiagnosticView, type DiagnosticTone } from './localNodeDiagnostics'
import type { LocalNodeStatus } from './types'
import styles from './LocalNodeReliability.module.css'

interface Props {
  status: LocalNodeStatus
  onRefresh: () => void | Promise<void>
}

export default function LocalNodeHealthPanel({ status, onRefresh }: Props) {
  const view = useMemo(() => localDiagnosticView(status), [status])
  const diagnosticCopy = useMemo(() => buildLocalDiagnosticCopy(status), [status])
  const [copyState, setCopyState] = useState<'idle' | 'ok' | 'fail'>('idle')

  async function copyDiagnostics() {
    try {
      await navigator.clipboard.writeText(diagnosticCopy)
      setCopyState('ok')
      window.setTimeout(() => setCopyState('idle'), 2200)
    } catch {
      setCopyState('fail')
      window.setTimeout(() => setCopyState('idle'), 2600)
    }
  }

  return (
    <section className={styles.panel}>
      <div className={styles.header}>
        <div>
          <p className={styles.eyebrow}>本机诊断</p>
          <h3 className={styles.title}>{view.title}</h3>
          <p className={styles.detail}>{view.detail}</p>
        </div>
        <span className={[styles.badge, toneClass(view.tone)].join(' ')}>{toneLabel(view.tone)}</span>
      </div>

      <div className={styles.grid}>
        {view.items.map((item) => (
          <div key={item.key} className={styles.item}>
            <div className={styles.itemTop}>
              <span className={styles.itemTitle}>{item.title}</span>
              <span className={[styles.badge, toneClass(item.tone)].join(' ')}>{item.badge}</span>
            </div>
            <p className={styles.itemDetail}>{item.detail}</p>
          </div>
        ))}
      </div>

      <div className={styles.actions}>
        <button className={styles.button} onClick={() => { void onRefresh() }}>
          <RefreshCw size={16} aria-hidden="true" />
          刷新状态
        </button>
        <button className={styles.button} onClick={copyDiagnostics}>
          <Clipboard size={16} aria-hidden="true" />
          {copyState === 'ok' ? '已复制' : copyState === 'fail' ? '复制失败' : '复制诊断'}
        </button>
        <a className={styles.button} href="elon-node://repair">
          <Wrench size={16} aria-hidden="true" />
          修复客户端
        </a>
        <a className={styles.button} href="elon-node://diagnostics/export">
          <FileDown size={16} aria-hidden="true" />
          导出诊断
        </a>
      </div>

      {copyState === 'fail' && (
        <p className={styles.hint}>浏览器没有允许剪贴板权限，请使用“导出诊断”。</p>
      )}
    </section>
  )
}

function toneClass(tone: DiagnosticTone): string {
  if (tone === 'ok') return styles.toneOk
  if (tone === 'warning') return styles.toneWarning
  if (tone === 'danger') return styles.toneDanger
  return styles.toneNeutral
}

function toneLabel(tone: DiagnosticTone): string {
  if (tone === 'ok') return '正常'
  if (tone === 'warning') return '需确认'
  if (tone === 'danger') return '需处理'
  return '待确认'
}
