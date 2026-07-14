import { useCallback, useEffect, useState } from 'react'

import { api, type ApiError } from '../../api/client'
import type { DocumentFile } from './projectDocumentModel'
import {
  createCustomSection,
  customSectionKey,
  EMPTY_SECTION_MANIFEST,
  ORGANIZATION_SUGGESTIONS_PATH,
  parseOrganizationSuggestions,
  parseSectionManifest,
  SECTION_CONFIG_PATH,
  serializeProjectDocumentJson,
  type DocumentOrganizationSuggestions,
  type DocumentSectionManifest,
} from './projectDocumentSections'

interface PersistedJson<T> {
  value: T
  revision?: string
}

interface AppliedOrganizationResponse {
  manifest: DocumentSectionManifest
  suggestions: DocumentOrganizationSuggestions
  manifest_revision?: string
  suggestions_revision?: string
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

  const applySuggestions = useCallback(async (catalogRevision: string) => {
    const suggestions = suggestionsFile.value
    if (!suggestions || suggestions.status !== 'ready') return manifestFile.value
    const result = await api.post<AppliedOrganizationResponse>(
      `/api/projects/${encodeURIComponent(projectId)}/docs/organization/apply`,
      {
        reviewed: true,
        expected_catalog_revision: catalogRevision,
        expected_manifest_revision: manifestFile.revision,
        expected_suggestions_revision: suggestionsFile.revision,
      },
    )
    setManifestFile({ value: result.manifest, revision: result.manifest_revision })
    setSuggestionsFile({ value: result.suggestions, revision: result.suggestions_revision })
    return result.manifest
  }, [manifestFile.revision, manifestFile.value, projectId, suggestionsFile.revision, suggestionsFile.value])

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

function errorMessage(error: unknown, fallback: string) {
  return (error as { message?: string })?.message ?? fallback
}
