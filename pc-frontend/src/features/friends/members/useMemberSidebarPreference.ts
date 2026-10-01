import { useEffect, useState, type SetStateAction } from 'react'

/** A per-account desktop preference; opening a mobile drawer does not overwrite it. */
export function useMemberSidebarPreference(userId: string) {
  const key = `group-member-sidebar:${userId}`
  const desktop = () => window.matchMedia('(min-width: 1101px)').matches
  const preferred = () => { try { return localStorage.getItem(key) !== 'closed' } catch { return true } }
  const [open, update] = useState(() => desktop() && preferred())
  useEffect(() => {
    const media = window.matchMedia('(min-width: 1101px)')
    const change = () => { let stored = true; try { stored = localStorage.getItem(key) !== 'closed' } catch { /* Storage can be unavailable. */ }; update(media.matches && stored) }
    media.addEventListener('change', change)
    return () => media.removeEventListener('change', change)
  }, [key])
  const setOpen = (value: SetStateAction<boolean>) => update(previous => {
    const next = typeof value === 'function' ? value(previous) : value
    if (desktop()) { try { localStorage.setItem(key, next ? 'open' : 'closed') } catch { /* Keep the live preference. */ } }
    return next
  })
  return [open, setOpen] as const
}
