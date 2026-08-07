// 一龙桌面壳（elon-desktop，Tauri 无边框窗口）宿主检测与窗口控制。
//
// 桌面壳通过 initialization_script 注入 `window.__ELON_DESKTOP_FRAMELESS__ = true`
// 并开启 withGlobalTauri；浏览器里两者都不存在，页面保持原样。
// 窗口控制权限由壳的 capabilities/main.json 精确授权（拖拽/最小化/最大化/关闭）。

interface TauriWindowHandle {
  minimize(): Promise<void>
  toggleMaximize(): Promise<void>
  close(): Promise<void>
  isMaximized(): Promise<boolean>
}

export type DesktopInvoke = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>

declare global {
  interface Window {
    __ELON_DESKTOP_FRAMELESS__?: boolean
    __TAURI__?: {
      core?: {
        invoke?: DesktopInvoke
      }
      window?: {
        getCurrentWindow?: () => TauriWindowHandle
      }
    }
  }
}

/** 是否运行在一龙桌面壳的无边框窗口里（需要页面渲染自己的标题栏）。 */
export function isDesktopShellFrameless(): boolean {
  return typeof window !== 'undefined' && window.__ELON_DESKTOP_FRAMELESS__ === true
}

/** 拿到当前 Tauri 窗口句柄；不在壳里或 API 不可用时返回 null。 */
export function getDesktopWindow(): TauriWindowHandle | null {
  try {
    return window.__TAURI__?.window?.getCurrentWindow?.() ?? null
  } catch {
    return null
  }
}

/** 调用桌面壳的本地命令；普通浏览器/PWA 中返回 null。 */
export function getDesktopInvoke(): DesktopInvoke | null {
  try {
    const invoke = window.__TAURI__?.core?.invoke
    return invoke ? (command, args) => invoke(command, args) : null
  } catch {
    return null
  }
}
