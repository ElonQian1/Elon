export function androidProjectRootCandidates(projectRoot: string) {
  const root = projectRoot.trim().replace(/[\\/]+$/, '')
  if (!root) return []
  const roots = [root]
  const recoveredMainRoot = root.replace(/-task-\d{8}-\d{6}-\d+-[a-z0-9]+$/i, '')
  if (recoveredMainRoot !== root) roots.push(recoveredMainRoot)
  const directRoots = roots
  const nestedRoots = roots.flatMap((candidate) => {
    const separator = candidate.includes('\\') ? '\\' : '/'
    return [
      `${candidate}${separator}android${separator}app`,
      `${candidate}${separator}android`,
      `${candidate}${separator}app`,
    ]
  })
  return [...new Set([...directRoots, ...nestedRoots])]
}
