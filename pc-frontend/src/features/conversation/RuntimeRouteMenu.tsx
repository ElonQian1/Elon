import { useEffect, useRef, useState } from 'react'
import styles from './RuntimeRouteMenu.module.css'
import {
  ACTIVE_RUNTIME_ROUTE_GROUPS,
  FUTURE_RUNTIME_ROUTES,
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
            <span>决定模型在哪里调用、PC harness 在哪里执行。</span>
          </div>

          {ACTIVE_RUNTIME_ROUTE_GROUPS.map((group) => (
            <section className={styles.routeGroup} key={group.title}>
              <div className={styles.routeGroupTitle}>
                <strong>{group.title}</strong>
                <span>{group.description}</span>
              </div>
              {group.options.map((route) => (
                <button
                  className={[
                    styles.routeOption,
                    route.value === value ? styles.routeOptionActive : '',
                  ].join(' ')}
                  key={route.value}
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
              ))}
            </section>
          ))}

          <section className={styles.routeGroup}>
            <div className={styles.routeGroupTitle}>
              <strong>下一阶段</strong>
              <span>远程别人 PC 节点需要先补授权、隔离、计费和审计。</span>
            </div>
            {FUTURE_RUNTIME_ROUTES.map((route) => (
              <div className={styles.routeOptionDisabled} key={route.key} aria-disabled="true">
                <span className={styles.routeCode}>{route.code}</span>
                <span className={styles.routeOptionCopy}>
                  <strong>{route.title}</strong>
                  <em>{route.subtitle}</em>
                  <span>{route.description}</span>
                </span>
                <b>{route.stage}</b>
              </div>
            ))}
          </section>
        </div>
      )}
    </div>
  )
}
