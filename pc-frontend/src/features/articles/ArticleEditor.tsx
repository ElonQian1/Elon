import { useEffect, useState } from 'react'
import { articles, errorMessage, uploadImage, type Article, type Block, type Group } from './articleApi'
import { ArticleBody } from './ArticleReader'
import styles from './Articles.module.css'

export default function ArticleEditor({ initial, groups, groupId, onClose }: { initial: Article; groups: Group[]; groupId: string; onClose: () => void }) {
  const [a, setA] = useState(initial); const [dirty, setDirty] = useState(false); const [busy, setBusy] = useState(false)
  const [error, setError] = useState(''); const [notice, setNotice] = useState(''); const [preview, setPreview] = useState(false)
  const [targets, setTargets] = useState([groupId]); const [choosing, setChoosing] = useState(false)
  useEffect(() => { const warn = (event: BeforeUnloadEvent) => { if (dirty) { event.preventDefault(); event.returnValue = '' } }; window.addEventListener('beforeunload', warn); return () => window.removeEventListener('beforeunload', warn) }, [dirty])
  const change = (patch: Partial<Article['document']>) => { setA(v => ({ ...v, document: { ...v.document, ...patch } })); setDirty(true); setNotice('') }
  const block = (i: number, b: Block) => change({ blocks: a.document.blocks.map((v, n) => i === n ? b : v) })
  const run = async (action: () => Promise<void>) => { if (busy) return; setBusy(true); setError(''); try { await action() } catch (e) { setError(errorMessage(e)) } finally { setBusy(false) } }
  const save = async () => { if (!dirty) return a; const saved = await articles.save(a); setA(saved); setDirty(false); return saved }
  const image = (file: File | undefined, cover: boolean) => { if (!file) return; void run(async () => {
    const m = await uploadImage(file)
    setA(v => ({ ...v, media: { ...v.media, [m.id]: m.data_url }, document: { ...v.document, ...(cover ? { cover: m.id } : { blocks: [...v.document.blocks, { type: 'image', media_id: m.id, caption: '' } as Block] }) } })); setDirty(true)
  }) }
  const close = () => { if (!dirty || window.confirm('还有未保存的修改，确定放弃并返回？')) onClose() }
  return <section className={styles.editor}>
    <header className={styles.toolbar}><button disabled={busy} onClick={close}>返回文章列表</button><span>{dirty ? '尚未保存' : '已保存'} · 草稿 v{a.revision}</span>
      <button disabled={busy} onClick={() => setPreview(!preview)}>{preview ? '继续编辑' : '预览'}</button>
      <button disabled={busy || !dirty} onClick={() => void run(async () => { await save(); setNotice('草稿已保存') })}>保存草稿</button>
      <button className={styles.primary} disabled={busy} onClick={() => { setPreview(true); setChoosing(true) }}>发布到群</button>
    </header>
    {error && <p className={styles.error} role="alert">{error}。当前内容仍保留在编辑器中。</p>}{notice && <p role="status" className={styles.notice}>{notice}</p>}
    {choosing && <section className={styles.publish} aria-label="选择发布群聊"><strong>选择群聊（最多10个）</strong><p>仅作者和所选群的当前成员可读。相同版本不会重复发送。</p>
      {groups.map(g => <label key={g.id}><input type="checkbox" disabled={busy || (!targets.includes(g.id) && targets.length >= 10)} checked={targets.includes(g.id)} onChange={() => setTargets(v => v.includes(g.id) ? v.filter(id => id !== g.id) : [...v, g.id])} />{g.name}</label>)}
      <button disabled={busy} onClick={() => setChoosing(false)}>取消</button><button className={styles.primary} disabled={busy || !targets.length} onClick={() => void run(async () => { const saved = await save(); const p = await articles.publish(saved, targets); setA(v => ({ ...v, status: 'published' })); setChoosing(false); setNotice(p.messages.length ? '文章已发布到所选群聊' : '这些群已收到此版本，没有重复发送') })}>{busy ? '正在发布…' : '确认发布'}</button>
    </section>}
    {preview ? <ArticleBody article={a} /> : <fieldset disabled={busy} className={styles.fields}>
      <label>标题<input value={a.document.title} maxLength={120} placeholder="给文章起个标题" onChange={e => change({ title: e.target.value })} /></label>
      <label>摘要<textarea value={a.document.summary} maxLength={300} placeholder="显示在群聊卡片上，选填" onChange={e => change({ summary: e.target.value })} /></label>
      <label>封面<input type="file" accept="image/png,image/jpeg,image/webp" onChange={e => { image(e.target.files?.[0], true); e.target.value = '' }} /></label>
      {a.document.cover && <div><img className={styles.coverPreview} src={a.media[a.document.cover]} alt="封面预览" /><button onClick={() => change({ cover: null })}>移除封面</button></div>}
      <p>正文 · 支持段落、标题、引用与图片</p>
      {a.document.blocks.map((b, i) => <div className={styles.block} key={i}>
        <div className={styles.blockTools}><span>第{i + 1}段</span><button disabled={i === 0} aria-label={`上移第${i + 1}段`} onClick={() => { const items = [...a.document.blocks]; [items[i - 1], items[i]] = [items[i], items[i - 1]]; change({ blocks: items }) }}>上移</button><button onClick={() => change({ blocks: a.document.blocks.filter((_, n) => i !== n) })}>删除</button></div>
        {b.type === 'image' ? <><img src={a.media[b.media_id]} alt="正文图片" /><input aria-label="图片说明" value={b.caption} maxLength={300} onChange={e => block(i, { ...b, caption: e.target.value })} placeholder="图片说明（选填）" /></> : <><select aria-label="段落类型" value={b.type} onChange={e => block(i, { type: e.target.value as 'paragraph' | 'heading' | 'quote', text: b.text })}><option value="paragraph">正文</option><option value="heading">小标题</option><option value="quote">引用</option></select><textarea aria-label={`第${i + 1}段内容`} value={b.text} rows={5} onChange={e => block(i, { ...b, text: e.target.value })} /></>}
      </div>)}
      <div className={styles.toolbar}><button disabled={a.document.blocks.length >= 120} onClick={() => change({ blocks: [...a.document.blocks, { type: 'paragraph', text: '' }] })}>＋ 添加段落</button><label>＋ 插入图片<input type="file" accept="image/png,image/jpeg,image/webp" disabled={a.document.blocks.length >= 120} onChange={e => { image(e.target.files?.[0], false); e.target.value = '' }} /></label></div>
      <p className={styles.meta}>修改已发布文章后，须再次发布才会发送新版本。旧卡片保留旧版本。</p>
      <button onClick={() => { if (window.confirm('撤下后，所有群中的文章将无法打开，也不能再次发布。确定撤下？')) void run(async () => { const saved = await save(); await articles.withdraw(saved); onClose() }) }}>撤下文章</button>
    </fieldset>}
  </section>
}
