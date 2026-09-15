import { useEffect, useState } from 'react'
import { articles, errorMessage, type Article } from './articleApi'
import styles from './Articles.module.css'
import ArticleDialog from './ArticleDialog'

export function ArticleBody({ article }: { article: Article }) {
  const d = article.document
  return <article className={styles.reading}>
    {d.cover && article.media[d.cover] && <img className={styles.hero} src={article.media[d.cover]} alt="文章封面" />}
    <div className={styles.prose}>
      <h1>{d.title || '未命名文章'}</h1>
      <p className={styles.meta}>{article.author_name} · {new Date(article.updated_at).toLocaleDateString()} · 文章</p>
      {d.summary && <p className={styles.abstract}>{d.summary}</p>}
      {d.blocks.map((b, i) => b.type === 'image' ? <figure key={i}><img src={article.media[b.media_id]} alt={b.caption || '正文图片'} /><figcaption>{b.caption}</figcaption></figure> : b.type === 'heading' ? <h2 key={i}>{b.text}</h2> : b.type === 'quote' ? <blockquote key={i}>{b.text}</blockquote> : <p key={i}>{b.text}</p>)}
    </div>
  </article>
}
export default function ArticleReader({ id, revision, onClose }: { id: string; revision: number; onClose: () => void }) {
  const [article, setArticle] = useState<Article>(); const [error, setError] = useState(''); const [retry, setRetry] = useState(0)
  useEffect(() => { let live = true; setArticle(undefined); setError(''); articles.read(id, revision).then(a => { if (live) setArticle(a) }).catch(e => { if (live) setError(errorMessage(e)) }); return () => { live = false } }, [id, revision, retry])
  return <ArticleDialog label="阅读文章" onCancel={onClose}>
    <section className={styles.panel}><header className={styles.toolbar}><button onClick={onClose}>返回群聊</button><span>文章</span></header>
      {error ? <div className={styles.empty} role="alert">{error}<button onClick={() => setRetry(retry + 1)}>重试</button></div> : article ? <ArticleBody article={article} /> : <p role="status" className={styles.empty}>正在读取文章…</p>}
    </section>
  </ArticleDialog>
}
