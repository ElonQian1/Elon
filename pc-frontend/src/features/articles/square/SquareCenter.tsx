import { useEffect, useState } from 'react'
import type { Article } from '../articleApi'
import ArticleDialog from '../ArticleDialog'
import SquareAccount from './SquareAccount'
import SquareComposer from './SquareComposer'
import SquareHistory from './SquareHistory'
import { square, secureBase, type Account } from './squareApi'
import styles from '../Articles.module.css'
import squareStyles from './Square.module.css'
export default function SquareCenter({ article, onClose }: { article?: Article; onClose: () => void }) {
  const securePageRequired = location.protocol === 'http:' && !window.isSecureContext
  const [account, setAccount] = useState<Account>(), [tab, setTab] = useState(article ? 'publish' : 'history'), [error, setError] = useState(''), [reload, setReload] = useState(0)
  useEffect(() => { if (securePageRequired) return; let live = true; setError(''); square<Account>('/account').then(a => { if (live) { setAccount(a); if (!a.bound) setTab('account') } }).catch(e => { if (live) setError(e.message) }); return () => { live = false } }, [reload, securePageRequired])
  if (securePageRequired) {
    let href = '', message = ''; try { href = secureBase() + '/square' + (article ? '?article=' + encodeURIComponent(article.id) : '') } catch(e) { message = e instanceof Error ? e.message : '安全页面暂不可用' }
    return <ArticleDialog label="币安发布安全页面" onCancel={onClose}><section className={styles.fields}><h2>打开币安发布安全页面</h2><p>请在完整 HTTPS 页面中登录并管理发帖凭证。</p>{href ? <a href={href} target="_blank" rel="noopener noreferrer">打开安全页面 ↗</a> : <p role="alert">{message}</p>}<button onClick={onClose}>返回文章</button></section></ArticleDialog>
  }
  return <ArticleDialog label="币安广场发布中心" onCancel={onClose}><section className={`${styles.panel} ${squareStyles.center}`}>
    <header className={styles.toolbar}><button onClick={onClose}>返回文章</button><h2>币安广场</h2></header>
    <nav className={styles.tabs} aria-label="币安发布中心">{article && <button aria-pressed={tab === 'publish'} onClick={() => setTab('publish')}>发布内容</button>}<button aria-pressed={tab === 'history'} onClick={() => setTab('history')}>发布记录</button><button aria-pressed={tab === 'account'} onClick={() => setTab('account')}>绑定账号</button></nav>
    {error && <p role="alert" className={styles.error}>{error} <button onClick={() => setReload(v => v + 1)}>重试连接</button></p>}
    {!account && !error && <p className={styles.empty} role="status">正在读取账号状态…</p>}
    {account && tab === 'account' && <SquareAccount account={account} onChange={setAccount} />}
    {account && tab === 'publish' && article && (account.bound ? <SquareComposer key={`${article.id}:${article.revision}:${account.generation}`} article={article} onQueued={() => setTab('history')} /> : <p className={styles.empty}>请先在“绑定账号”中保存发帖凭证。</p>)}
    {account && tab === 'history' && <SquareHistory />}
  </section></ArticleDialog>
}
