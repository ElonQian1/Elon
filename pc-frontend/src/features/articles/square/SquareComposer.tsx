import { useRef, useState } from 'react'
import type { Article } from '../articleApi'
import { square, modes, stamp, requestId, type Mode, type Selection, type Preview, type Job } from './squareApi'
import { squareImage, squareVideo } from './SquareMedia'
import styles from '../Articles.module.css'
export default function SquareComposer({ article, onQueued }: { article: Article; onQueued: () => void }) {
  const [mode, setMode] = useState<Mode>('article'), [cover, setCover] = useState(article.document.cover), [images, setImages] = useState<string[]>([]), [media, setMedia] = useState(article.media)
  const [video, setVideo] = useState<{ id: string; name: string; cover: string }>(), [preview, setPreview] = useState<Preview>(), [schedule, setSchedule] = useState(''), [publicOk, setPublicOk] = useState(false), [converted, setConverted] = useState(false)
  const [busy, setBusy] = useState(false), [error, setError] = useState(''), [done, setDone] = useState<Job>()
  const requestKey = useRef(requestId())
  const invalidate = () => { setPreview(undefined); setPublicOk(false); setConverted(false); setDone(undefined); requestKey.current = requestId() }
  const run = async (fn: () => Promise<void>) => { if (busy) return; setBusy(true); setError(''); try { await fn() } catch (e) { setError(e instanceof Error ? e.message : '操作失败') } finally { setBusy(false) } }
  const selection = (): Selection => ({ article_id: article.id, version: article.revision, mode, media_ids: mode === 'images' ? images : [], cover_id: mode === 'article' ? cover : null, video_id: mode === 'video' ? video?.id || null : null })
  return <section className={styles.fields} aria-label="发布到币安广场"><h3>{article.document.title}</h3><p>使用已保存的文章 v{article.revision}。群聊中的文章权限保持原样。</p>
    {error && <p role="alert" className={styles.error}>{error}</p>}
    {done ? <><p role="status">{done.status === 'published' ? '此版本已发布' : `任务已记录：${done.status === 'queued' ? '等待发送' : done.message}`} · {stamp(done.scheduled_at)}</p><button onClick={onQueued}>查看发布记录</button></> : <>
      <fieldset disabled={busy} className={styles.fields}><label>发布形式<select value={mode} onChange={e => { setMode(e.target.value as Mode); invalidate() }}>{Object.entries(modes).map(([value, label]) => <option value={value} key={value}>{label}</option>)}</select></label>
        {mode === 'article' && <label>文章封面<select value={cover || ''} onChange={e => { setCover(e.target.value || null); invalidate() }}><option value="">不使用封面</option>{Object.keys(media).map((id, i) => <option key={id} value={id}>图片 {i + 1}{id === article.document.cover ? '（原封面）' : ''}</option>)}</select></label>}
        {(mode === 'article' || mode === 'images') && <div className={styles.toolbar}>{Object.entries(media).map(([id, url], i) => <label key={id} style={{ width: 116 }}><img style={{ width: 100, height: 70, objectFit: 'cover' }} src={url} alt={`图片 ${i + 1}`} />{mode === 'images' && <input type="checkbox" checked={images.includes(id)} disabled={busy || (!images.includes(id) && images.length >= 4)} onChange={() => { setImages(v => v.includes(id) ? v.filter(x => x !== id) : [...v, id]); invalidate() }} />}图片 {i + 1}</label>)}</div>}
        {(mode === 'article' || mode === 'images') && <label>添加图片<input type="file" accept="image/png,image/jpeg,image/webp" onChange={e => { const f = e.target.files?.[0]; e.target.value = ''; if (f) void run(async () => { const m = await squareImage(f); setMedia(v => ({ ...v, [m.id]: m.data_url })); if (mode === 'article') setCover(m.id); else setImages(v => v.length < 4 && !v.includes(m.id) ? [...v, m.id] : v); invalidate() }) }} /></label>}
        {mode === 'images' && <p>已选择 {images.length}/4 张；按选择顺序展示。</p>}
        {mode === 'video' && <><label>选择视频<input type="file" accept="video/mp4,video/webm" onChange={e => { const f = e.target.files?.[0]; e.target.value = ''; if (f) void run(async () => { setVideo(await squareVideo(f)); invalidate() }) }} /></label><p>MP4/WebM，最多32MB、10分钟，自动使用首帧封面。</p>{video && <><img className={styles.coverPreview} src={video.cover} alt="视频首帧封面" /><p>{video.name}</p></>}</>}
        <button disabled={busy || (mode === 'images' && !images.length) || (mode === 'video' && !video)} onClick={() => void run(async () => { setPreview(await square('/preview', 'POST', selection())); setPublicOk(false); setConverted(false) })}>{busy ? '正在处理…' : '生成币安版本预览'}</button>
      </fieldset>
      {preview && <section className={styles.publish} aria-label="币安版本预览"><h3>发布到：{preview.account_label}</h3>{mode === 'article' && <h2>{preview.title}</h2>}
        {(preview.selection.mode === 'images' ? preview.selection.media_ids : Object.keys(preview.media)).map(id => <img className={styles.coverPreview} key={id} src={preview.media[id]} alt="将发送的图片或视频封面" />)}
        <p style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere', maxHeight: 420, overflow: 'auto' }}>{preview.text}</p>
        {preview.warnings.map(w => <p key={w}>{w}</p>)}
        <label>发送时间（留空立即发送）<input type="datetime-local" value={schedule} disabled={busy} onChange={e => setSchedule(e.target.value)} /></label>
        <label><input type="checkbox" checked={converted} disabled={busy} onChange={e => setConverted(e.target.checked)} />已核对文字转换和选定媒体</label>
        <label><input type="checkbox" checked={publicOk} disabled={busy} onChange={e => setPublicOk(e.target.checked)} />将此内容公开发布到上述币安账号</label>
        <p className={styles.meta}>一龙撤下文章不会删除币安帖子。提交结果不确定时，请先到发布记录核实。</p>
        <button className={styles.primary} disabled={busy || !publicOk || !converted} onClick={() => void run(async () => {
          const at = schedule ? Math.floor(new Date(schedule).getTime() / 1000) : null
          if (at !== null && (!Number.isFinite(at) || at <= Date.now() / 1000)) throw Error('请选择未来的发送时间')
          setDone(await square('/jobs', 'POST', { selection: preview.selection, preview_hash: preview.preview_hash, request_key: requestKey.current, public_confirmed: publicOk, conversion_confirmed: converted, scheduled_at: at }))
        })}>{busy ? '正在创建任务…' : schedule ? '确认定时发布' : '确认公开发布'}</button>
      </section>}
    </>}
  </section>
}
