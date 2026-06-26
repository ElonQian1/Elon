import { useState } from 'react'
import { CreateProjectModal } from './CreateProjectModal'
import styles from './ProjectsPage.module.css'

export default function ProjectsPage() {
  const [showCreate, setShowCreate] = useState(false)

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h2>项目中心</h2>
        <button className={styles.createBtn} onClick={() => setShowCreate(true)}>
          + 新建项目
        </button>
      </header>
      <p className={styles.hint}>项目列表迁移中，请暂时使用旧版 <a href="/pc">/pc</a></p>

      {showCreate && (
        <CreateProjectModal
          onClose={() => setShowCreate(false)}
          onCreated={(p) => {
            setShowCreate(false)
            if (p.id) window.location.href = `/pc#project-${p.id}`
          }}
        />
      )}
    </div>
  )
}
