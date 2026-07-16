export function androidProjectRootCandidates(projectRoot: string) {
  const root = projectRoot.trim().replace(/[\\/]+$/, '')
  if (!root) return []
  const separator = root.includes('\\') ? '\\' : '/'
  return [
    root,
    `${root}${separator}android${separator}app`,
    `${root}${separator}android`,
    `${root}${separator}app`,
  ]
}
