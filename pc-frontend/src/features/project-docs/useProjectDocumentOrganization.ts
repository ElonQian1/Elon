import { useCallback, useEffect, useState } from 'react'

import { api, type ApiError } from '../../api/client'
import type { DocumentFile, ProjectDocumentEntry } from './projectDocumentModel'
import {
  buildDocumentSections,
  createCustomSection,
  customSectionKey,
  EMPTY_SECTION_MANIFEST,
  ORGANIZATION_SUGGESTIONS_PATH,
  parseOrganizationSuggestions,
  parseSectionManifest,
  SECTION_CONFIG_PATH,
  serializeProjectDocumentJson,
  SYSTEM_DOCUMENT_SECTIONS,
  type DocumentOrganizationSuggestions,
  type DocumentSectionManifest,
} from './projectDocumentSections'

interface PersistedJson<T> {
  value: T
  revision?: string
}

export function useProjectDocumentOrganization(projectId: string) {
  const [manifestFile, setManifestFile] = useState<PersistedJson<DocumentSectionManifest>>({
    value: EMPTY_SECTION_MANIFEST,
  })
  const [suggestionsFile, setSuggestionsFile] = useState<PersistedJson<DocumentOrganizationSuggestions | null>>({
    value: null,
  })
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    setError('')
    const [manifest, suggestions] = await Promise.all([
      readJsonFile(projectId, SECTION_CONFIG_PATH),
      readJsonFile(projectId, ORGANIZATION_SUGGESTIONS_PATH),
    ])
    setManifestFile({
      value: manifest ? parseSectionManifest(manifest.content) : { version: 1, sections: [], assignments: {} },
      revision: manifest?.revision,
    })
    setSuggestionsFile({
      value: suggestions ? parseOrganizationSuggestions(suggestions.content) : null,
      revision: suggestions?.revision,
    })
    setLoading(false)
  }, [projectId])

  useEffect(() => {
    load().catch((reason) => {
      setError(errorMessage(reason, '读取项目分区配置失败'))
      setLoading(false)
    })
  }, [load])

  const saveManifest = useCallback(async (value: DocumentSectionManifest) => {
    const saved = await writeJsonFile(projectId, SECTION_CONFIG_PATH, value, manifestFile.revision)
    setManifestFile({ value, revision: saved.revision })
    return value
  }, [manifestFile.revision, projectId])

  const saveSuggestions = useCallback(async (value: DocumentOrganizationSuggestions) => {
    const saved = await writeJsonFile(
      projectId,
      ORGANIZATION_SUGGESTIONS_PATH,
      value,
      suggestionsFile.revision,
    )
    setSuggestionsFile({ value, revision: saved.revision })
    return value
  }, [projectId, suggestionsFile.revision])

  const addSection = useCallback(async (label: string) => {
    const section = createCustomSection(label, manifestFile.value.sections)
    await saveManifest({
      ...manifestFile.value,
      sections: [...manifestFile.value.sections, section],
    })
    return customSectionKey(section.id)
  }, [manifestFile.value, saveManifest])

  const removeSection = useCallback(async (sectionKey: string) => {
    const id = sectionKey.replace(/^custom:/, '')
    const assignments = Object.fromEntries(
      Object.entries(manifestFile.value.assignments).filter(([, assigned]) => assigned !== sectionKey),
    )
    await saveManifest({
      version: 1,
      sections: manifestFile.value.sections.filter((section) => section.id !== id),
      assignments,
    })
  }, [manifestFile.value, saveManifest])

  const assignDocument = useCallback(async (path: string, sectionKey: string) => {
    const normalized = normalizePath(path)
    const assignments = { ...manifestFile.value.assignments }
    if (sectionKey) assignments[normalized] = sectionKey
    else delete assignments[normalized]
    const nextManifest = {
      ...manifestFile.value,
      assignments,
    }
    await saveManifest(nextManifest)
    return nextManifest
  }, [manifestFile.value, saveManifest])

  const applySuggestions = useCallback(async (documents: ProjectDocumentEntry[]) => {
    const suggestions = suggestionsFile.value
    if (!suggestions || suggestions.status !== 'ready') return manifestFile.value
    const knownPaths = new Set(documents.map((document) => normalizePath(document.path)))
    const sectionsById = new Map(manifestFile.value.sections.map((section) => [section.id, section]))
    for (const section of suggestions.proposed_sections) sectionsById.set(section.id, section)
    const nextSections = [...sectionsById.values()]
    const validKeys = new Set(buildDocumentSections({
      version: 1,
      sections: nextSections,
      assignments: {},
    }).filter((section) => !section.virtual).map((section) => section.key))
    const assignments = { ...manifestFile.value.assignments }
    for (const suggestion of suggestions.assignments) {
      const path = normalizePath(suggestion.path)
      const sectionKey = normalizeSuggestedSectionKey(suggestion.section_id, nextSections.map((section) => section.id))
      if (knownPaths.has(path) && validKeys.has(sectionKey)) assignments[path] = sectionKey
    }
    const nextManifest: DocumentSectionManifest = { version: 1, sections: nextSections, assignments }
    await saveManifest(nextManifest)
    await saveSuggestions({ ...suggestions, status: 'applied' })
    return nextManifest
  }, [manifestFile.value, saveManifest, saveSuggestions, suggestionsFile.value])

  return {
    manifest: manifestFile.value,
    suggestions: suggestionsFile.value,
    loading,
    error,
    reload: load,
    addSection,
    removeSection,
    assignDocument,
    applySuggestions,
  }
}

async function readJsonFile(projectId: string, path: string) {
  try {
    return await api.get<DocumentFile>(documentFileUrl(projectId, path))
  } catch (error) {
    if ((error as ApiError)?.status === 404) return null
    throw error
  }
}

async function writeJsonFile(
  projectId: string,
  path: string,
  value: unknown,
  expectedRevision?: string,
) {
  return api.put<{ revision: string; byte_len: number }>(
    `/api/projects/${encodeURIComponent(projectId)}/docs/file`,
    {
      path,
      content: serializeProjectDocumentJson(value),
      expected_revision: expectedRevision,
    },
  )
}

function documentFileUrl(projectId: string, path: string) {
  return `/api/projects/${encodeURIComponent(projectId)}/docs/file?path=${encodeURIComponent(path)}`
}

function normalizePath(path: string) {
  return path.trim().replace(/\\/g, '/')
}

function normalizeSuggestedSectionKey(sectionId: string, customIds: string[]) {
  const value = sectionId.trim()
  if (SYSTEM_DOCUMENT_SECTIONS.some((section) => section.key === value)) return value
  if (value.startsWith('custom:')) return value
  if (customIds.includes(value)) return customSectionKey(value)
  return value
}

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}
