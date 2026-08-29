import { Component, type ErrorInfo, type ReactNode } from 'react'
import styles from './WorkbenchErrorBoundary.module.css'

interface WorkbenchErrorBoundaryProps {
  children: ReactNode
}

interface WorkbenchErrorBoundaryState {
  failed: boolean
}

export default class WorkbenchErrorBoundary extends Component<
  WorkbenchErrorBoundaryProps,
  WorkbenchErrorBoundaryState
> {
  state: WorkbenchErrorBoundaryState = { failed: false }

  static getDerivedStateFromError(): WorkbenchErrorBoundaryState {
    return { failed: true }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('工作台页面渲染失败', error, info.componentStack)
  }

  render() {
    if (!this.state.failed) return this.props.children
    return (
      <main className={styles.surface} role="alert">
        <section className={styles.card}>
          <span className={styles.status}>一龙工作台仍在运行</span>
          <h1>这个页面加载失败了</h1>
          <p>已阻止整窗黑屏。你可以立即重试当前页面，或先返回本机任务页。</p>
          <div className={styles.actions}>
            <button type="button" onClick={() => window.location.reload()}>重试当前页面</button>
            <button type="button" onClick={returnToSafeHome}>返回本机任务</button>
          </div>
        </section>
      </main>
    )
  }
}

function returnToSafeHome() {
  window.location.assign(window.location.pathname.startsWith('/pc') ? '/pc/local-tasks' : '/')
}
