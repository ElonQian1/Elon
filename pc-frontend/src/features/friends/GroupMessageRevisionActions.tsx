import { useEffect, useRef, useState } from 'react'
import { api } from '../../api/client'
import type { SocialMessage } from './socialMessageTypes'
import { changedText, revisionOf, revisionsPath, type MessageEdit, type MessageHistory, type MessageRevision } from './groupMessageRevisions'
import styles from './GroupMessageRevisions.module.css'

interface Props { groupId: string; message: SocialMessage; own: boolean; onSaved: (message: MessageEdit) => void }

export default function GroupMessageRevisionActions({ groupId, message, own, onSaved }: Props) {
  const [mode, setMode] = useState<'edit' | 'history' | null>(null)
  const [draft, setDraft] = useState('')
  const [expected, setExpected] = useState(1)
  const [versions, setVersions] = useState<MessageRevision[]>([])
  const [before, setBefore] = useState<number | null>(null)
  const [conflict, setConflict] = useState<MessageRevision | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const dialog = useRef<HTMLDialogElement>(null)
  const epoch = useRef(0)
  const path = revisionsPath(groupId, message.id)
  const recalled = !!(message.recalled_at || message.recalledAt)
  const editable = own && !!message.content.trim() && !message.content.startsWith('【一龙项目卡片】')
  useEffect(() => () => { epoch.current++ }, [])
  useEffect(() => {
    if (mode && dialog.current && !dialog.current.open) dialog.current.showModal()
  }, [mode])
  useEffect(() => { if (recalled) { epoch.current++; setMode(null) } }, [recalled])
  if (recalled || message.id.startsWith('tmp-')) return null

  function close() { if (!busy) { epoch.current++; setMode(null) } }
  async function history(append = false) {
    const request = ++epoch.current
    setMode('history'); setError(''); setBusy(true)
    if (!append) { setVersions([]); setBefore(null) }
    try {
      const data = await api.get<MessageHistory>(`${path}/revisions?limit=20${append && before ? `&before_revision=${before}` : ''}`)
      if (request !== epoch.current) return
      setVersions(previous => append ? [...previous, ...data.revisions] : data.revisions)
      setBefore(data.next_before_revision)
    } catch (failure) {
      if (request === epoch.current) setError((failure as Error).message || '读取失败，请重试')
    } finally { if (request === epoch.current) setBusy(false) }
  }
  function edit() {
    epoch.current++; setDraft(message.content); setExpected(revisionOf(message)); setConflict(null); setError(''); setBusy(false); setMode('edit')
  }
  async function save() {
    if (busy || conflict || !draft.trim() || Array.from(draft.trim()).length > 4000) return
    const request = ++epoch.current
    setBusy(true); setError('')
    try {
      const result = await api.patch<{ message: MessageEdit }>(path, { content: draft.trim(), expected_revision: expected })
      if (request !== epoch.current) return
      onSaved(result.message); setMode(null)
    } catch (failure) {
      if (request !== epoch.current) return
      setError((failure as Error).message || '保存失败，草稿已保留，请重试')
      if ((failure as { status?: number }).status === 409) {
        try {
          const latest = await api.get<MessageHistory>(`${path}/revisions?limit=1`)
          if (request === epoch.current) setConflict(latest.revisions[0] ?? null)
        } catch { /* Keep original expected version: retry must still detect the conflict. */ }
      }
    } finally { if (request === epoch.current) setBusy(false) }
  }
  return <>
    <div className={styles.actions}>
      {revisionOf(message) > 1 && <button type="button" onClick={() => void history()} title={message.edited_at ? `最后修改：${new Date(message.edited_at).toLocaleString()}` : undefined}>已编辑 · {revisionOf(message) - 1} 次</button>}
      {editable && <button type="button" onClick={edit}>编辑</button>}
    </div>
    {mode && <dialog ref={dialog} className={styles.dialog} onCancel={event => { event.preventDefault(); close() }} aria-labelledby={`revision-title-${message.id}`}>
      <header><h2 id={`revision-title-${message.id}`}>{mode === 'edit' ? '编辑消息' : '修改记录'}</h2><button type="button" onClick={close} disabled={busy} aria-label="关闭">关闭</button></header>
      <div className={styles.body}>
        <p className={styles.hint}>{mode === 'edit' ? '保存后将标记为已编辑，群成员可查看每一版文字。附件保持原样。' : '按最新到最早排列，每一版均保留完整文字。'}</p>
        {mode === 'edit' ? <>
          <label htmlFor={`revision-draft-${message.id}`}>消息文字</label>
          <textarea id={`revision-draft-${message.id}`} autoFocus value={draft} onChange={event => setDraft(event.target.value)} disabled={busy} rows={7} />
          <p className={styles.hint}>{Array.from(draft.trim()).length} / 4000 字</p>
          {conflict && <section className={styles.conflict}>
            <strong>其他设备已保存第 {conflict.revision} 版</strong><p>最新文字：</p><pre>{conflict.content}</pre>
            <button type="button" onClick={() => { setExpected(conflict.revision); setConflict(null); setError('已保留你的草稿，请核对后点击保存') }}>已核对，继续编辑我的草稿</button>
          </section>}
        </> : <>
          {versions.map((version, index) => {
            const previous = versions[index + 1]
            const diff = previous ? changedText(previous.content, version.content) : null
            return <article className={styles.version} key={version.revision}>
              <h3>第 {version.revision} 版{version.revision === 1 ? ' · 原始文字' : ''}</h3>
              <time>{new Date(version.created_at).toLocaleString()}</time><pre>{version.content}</pre>
              {diff && <details><summary>与上一版相比</summary>{diff.removed && <p>删除：<del>{diff.removed}</del></p>}{diff.added && <p>新增：<ins>{diff.added}</ins></p>}</details>}
            </article>
          })}
          {!busy && !error && !versions.length && <p>暂无修改记录</p>}
          {before && <button type="button" disabled={busy} onClick={() => void history(true)}>查看更早版本</button>}
          {error && <button type="button" disabled={busy} onClick={() => void history(versions.length > 0)}>重新读取</button>}
        </>}
        {error && <p role="alert" className={styles.error}>{error}</p>}
        {busy && <p role="status">{mode === 'edit' ? '正在保存…' : '正在读取…'}</p>}
      </div>
      {mode === 'edit' && <footer><button type="button" onClick={close} disabled={busy}>取消</button><button type="button" onClick={() => void save()} disabled={busy || !!conflict || !draft.trim() || Array.from(draft.trim()).length > 4000}>保存修改</button></footer>}
    </dialog>}
  </>
}
