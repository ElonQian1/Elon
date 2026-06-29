import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'
import { Settings } from 'lucide-react'
import styles from './RuntimeRouteMenu.module.css'
import {
  ACTIVE_RUNTIME_ROUTE_GROUPS,
  runtimeRouteDescription,
  runtimeRouteOption,
} from './runtimeRoutes'
import type { RuntimeRoute } from './runtimeRoutes'

interface Props {
  value: RuntimeRoute
  disabled?: boolean
  onChange: (value: RuntimeRoute) => void
}

export default function RuntimeRouteMenu({ value, disabled, onChange }: Props) {
  const [open, setOpen] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)
  const selected = runtimeRouteOption(value)

  useEffect(() => {
    if (!open) return
    function onPointerDown(event: MouseEvent) {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false)
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') setOpen(false)
    }
    document.addEventListener('mousedown', onPointerDown)
    document.addEventListener('keydown', onKeyDown)
    return () => {
      document.removeEventListener('mousedown', onPointerDown)
      document.removeEventListener('keydown', onKeyDown)
    }
  }, [open])

  function choose(route: RuntimeRoute) {
    onChange(route)
    setOpen(false)
  }

  return (
    <div className={styles.routePicker} ref={rootRef}>
      <button
        className={styles.routeButton}
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        title={`运行路线：${runtimeRouteDescription(value)}`}
        disabled={disabled}
        onClick={() => setOpen((next) => !next)}
      >
        <span className={styles.routeButtonLabel}>运行路线</span>
        <strong>{selected.shortLabel}</strong>
        <span className={styles.routeChevron}>⌄</span>
      </button>

      {open && (
        <div className={styles.routePopover} role="menu" aria-label="选择项目 AI 运行路线">
          <div className={styles.routePopoverHead}>
            <strong>选择项目 AI 运行路线</strong>
            <span>决定模型在哪里调用、PC harness 在哪台电脑执行。</span>
          </div>

          {ACTIVE_RUNTIME_ROUTE_GROUPS.map((group) => (
            <section className={styles.routeGroup} key={group.title}>
              <div className={styles.routeGroupTitle}>
                <strong>{group.title}</strong>
                <span>{group.description}</span>
              </div>
              {group.options.map((route) => (
                <div
                  className={[
                    styles.routeOption,
                    route.value === value ? styles.routeOptionActive : '',
                  ].join(' ')}
                  key={route.value}
                >
                  <button
                    className={styles.routeOptionSelect}
                    type="button"
                    role="menuitemradio"
                    aria-checked={route.value === value}
                    onClick={() => choose(route.value)}
                  >
                    <span className={styles.routeCode}>{route.code}</span>
                    <span className={styles.routeOptionCopy}>
                      <strong>{route.title}</strong>
                      <em>{route.subtitle}</em>
                      <span>{route.description}</span>
                    </span>
                  </button>
                  {route.configHref && (
                    <Link
                      className={styles.routeConfigLink}
                      to={route.configHref}
                      title={route.configLabel ?? `${route.title}配置`}
                      aria-label={route.configLabel ?? `${route.title}配置`}
                      onClick={() => setOpen(false)}
                    >
                      <Settings size={15} strokeWidth={2.2} aria-hidden="true" />
                    </Link>
                  )}
                </div>
              ))}
            </section>
          ))}
        </div>
      )}
    </div>
  )
}
