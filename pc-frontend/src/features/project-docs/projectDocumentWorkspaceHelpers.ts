export function projectDocumentErrorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}

export function normalizeProjectDocumentPath(path: string) {
  return path.trim().replace(/\\/g, '/')
}
