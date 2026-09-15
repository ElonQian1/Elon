import { useState } from 'react'
import { square, creatorUrl, stamp, type Account } from './squareApi'
import styles from '../Articles.module.css'
export default function SquareAccount({ account, onChange }: { account: Account; onChange: (a: Account) => void }) {
  const [label, setLabel] = useState(account.label), [key, setKey] = useState(''), [busy, setBusy] = useState(false), [message, setMessage] = useState('')
  const run = async (fn: () => Promise<void>) => { if (busy) return; setBusy(true); setMessage(''); try { await fn() } catch (e) { setMessage(e instanceof Error ? e.message : '保存失败') } finally { setBusy(false) } }
  return <section className={styles.fields} aria-label="币安广场账号">
    <h3>{account.bound ? account.label : '绑定币安广场账号'}</h3>
    <p>{account.bound ? `${account.masked_key} · ${account.verified_at ? '已通过发帖验证 · ' + stamp(account.verified_at) : '凭证已保存，尚未通过实际发帖验证'}` : '使用创作者中心的专用发帖凭证。每个一龙账号绑定自己的发布账号。'}</p>
    <a href={creatorUrl} target="_blank" rel="noopener noreferrer">打开币安创作者中心，获取发帖凭证 ↗</a>
    <label>账号备注<input value={label} maxLength={60} disabled={busy} onChange={e => setLabel(e.target.value)} placeholder="例如：我的币安广场" /></label>
    <label>{account.bound ? '更换发帖凭证' : '发帖凭证'}<input type="password" autoComplete="new-password" value={key} disabled={busy} maxLength={512} onChange={e => setKey(e.target.value)} placeholder="粘贴 Square OpenAPI Key" /></label>
    <p className={styles.meta}>通过 HTTPS 传输并加密保存。这里只使用发帖凭证，无需交易密钥。更换或解绑会取消尚未提交的任务。</p>
    <button disabled={busy || !key.trim() || !label.trim()} className={styles.primary} onClick={() => void run(async () => { const a = await square<Account>('/account', 'PUT', { label, api_key: key }); setKey(''); onChange(a); setMessage('已保存。首次发帖成功后会更新验证状态。') })}>{busy ? '正在保存…' : '保存绑定'}</button>
    {account.bound && <button disabled={busy} onClick={() => { if (confirm('解绑会取消尚未提交的任务，并删除保存的发帖凭证；币安已有帖子不受影响。')) void run(async () => { onChange(await square('/account', 'DELETE')); setKey(''); setMessage('已解绑') }) }}>解绑账号</button>}
    {message && <p role="status">{message}</p>}
  </section>
}
