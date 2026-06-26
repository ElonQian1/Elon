import styles from './DevTasksPage.module.css'

export default function DevTasksPage() {
  return (
    <div className={styles.page}>
      <div className={styles.hero}>
        <h2>AI 开发任务</h2>
        <p>
          请在项目页面选择一个 AI 开发频道查看任务详情。
          <br />
          DevTaskCard、AgentRunsPanel 组件已就绪，等待项目频道路由集成。
        </p>
        <a className={styles.link} href="/pc">前往旧版 PC 查看 AI 开发频道 →</a>
      </div>
    </div>
  )
}
