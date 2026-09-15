import { useEffect, useState } from 'react'
import { articles, articleReference, type Card } from './articleApi'
import ArticleReader from './ArticleReader'
import styles from './Articles.module.css'

export default function ArticleMessage({ content }: { content: string }) {
  const ref = articleReference(content)
  const [card, setCard] = useState<Card>(); const [open, setOpen] = useState(false)
  const id = ref?.article_id; const revision = ref?.revision
  useEffect(() => { let live = true; setCard(undefined); if (id && revision) articles.read(id, revision, true).then(a => { if (live) setCard(a) }).catch(() => {}); return () => { live = false } }, [id, revision])
  if (!ref) return null
  return <><button className={styles.card} onClick={() => setOpen(true)} aria-label={`阅读文章：${ref.title}`}>
    {card?.cover_data_url && <img src={card.cover_data_url} alt="" />}
    <span className={styles.cardText}><small>文章{card ? ` · ${card.author_name}` : ''}</small><strong>{card?.title || ref.title}</strong><span>{card?.summary || ref.summary}</span><small>阅读全文 →</small></span>
  </button>{open && <ArticleReader id={ref.article_id} revision={ref.revision} onClose={() => setOpen(false)} />}</>
}
