import { useState } from 'react'
import { tokenSymbol } from './gridSharePickerModel'
import styles from './GridSharePicker.module.css'

// CC0 cryptocurrency-icons assets are served by our own origin. No account,
// strategy identifier or private holdings are sent to a logo service.
const icons = import.meta.glob<string>('../../../node_modules/cryptocurrency-icons/svg/color/*.svg', { eager: true, query: '?url&no-inline', import: 'default' })
export default function GridTokenIcon({ symbol }: { symbol: string }) {
  const base = tokenSymbol(symbol), [failed, setFailed] = useState('')
  const url = /^[A-Z0-9]{1,24}$/.test(base) ? icons[`../../../node_modules/cryptocurrency-icons/svg/color/${base.toLowerCase()}.svg`] : undefined
  const hue = [...base].reduce((n, c) => (n * 31 + c.charCodeAt(0)) % 360, 0)
  return <span className={styles.tokenIcon} style={{ backgroundColor: `hsl(${hue} 32% 24%)` }} aria-hidden="true">
    {url && failed !== url ? <img src={url} alt="" width="40" height="40" onError={() => setFailed(url)} /> : <span>{base.slice(0, 2)}</span>}
  </span>
}
