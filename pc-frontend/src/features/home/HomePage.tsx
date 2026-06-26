import styles from './HomePage.module.css'

export default function HomePage() {
  return (
    <div className={styles.page}>
      <div className={styles.hero}>
        <h2 className={styles.heading}>PC 工作台新版</h2>
        <p className={styles.desc}>
          基于 Vite + React + TypeScript 重构中。
          <br />
          功能模块将逐步从旧版迁移至此。
        </p>
        <a className={styles.legacyBtn} href="/pc">
          前往旧版 PC 工作台 →
        </a>
      </div>
    </div>
  )
}
