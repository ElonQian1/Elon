export function androidProjectRootCandidates(projectRoot: string) {
  const root = projectRoot.trim().replace(/[\\/]+$/, '')
  if (!root) return []
  const roots = [root]
  const recoveredMainRoot = root.replace(/-task-\d{8}-\d{6}-\d+-[a-z0-9]+$/i, '')
  if (recoveredMainRoot !== root) roots.push(recoveredMainRoot)
  return [...new Set(roots.flatMap((candidate) => {
    const separator = candidate.includes('\\') ? '\\' : '/'
    return [
      candidate,
      `${candidate}${separator}android${separator}app`,
      `${candidate}${separator}android`,
      `${candidate}${separator}app`,
    ]
  }))]
}
