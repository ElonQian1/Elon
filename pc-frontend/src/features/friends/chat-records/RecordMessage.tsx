import { useCallback, useMemo, useState } from 'react'
import type { RecordRow } from './recordApi'
import type { LinkPreview } from '../socialLinks'
import SocialLinkCards from '../SocialLinkCards'
import styles from './ChatRecords.module.css'

export default function RecordMessage({ row, owner, openLink }: { row: RecordRow; owner: string; openLink: () => void }) {
  const initial = useMemo(() => ElonSocialLinks.links(row.text), [row.text])
  const [cards, setCards] = useState(initial)
  const preview = useCallback((value: LinkPreview) => setCards(old => [...old.filter(c => c.url !== value.url), value]), [])
  const text = ElonRecordPresentation.text(row, cards)
  return <>
    {text && <p className={styles.text}>{text.split(/(https?:\/\/[^\s]+)/g).map((s, i) => /^https?:\/\//.test(s) ? <a key={i} href={s} target="_blank" rel="noreferrer">{s}</a> : s)}</p>}
    <SocialLinkCards text={row.text} owner={owner} compact onDesktopOpen={openLink} recordActions onPreview={preview} />
  </>
}
