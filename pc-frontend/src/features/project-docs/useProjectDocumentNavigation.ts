import { useMemo } from 'react'

import {
  buildKnowledgeSections,
  DOCUMENT_HEALTH_SECTION,
  knowledgeSectionCounts,
  topicSectionsForDocument,
  type DocumentNavigationMode,
} from './projectDocumentArchitecture'
import {
  sortDocuments,
  sortHierarchicalSections,
  type ProjectDocumentViewPreferences,
} from './projectDocumentCommands'
import type { DocumentCatalog } from './projectDocumentModel'
import {
  buildDocumentSections,
  governanceSectionForDocument,
  type DocumentOrganizationSuggestions,
  type DocumentSectionManifest,
} from './projectDocumentSections'

interface Input {
  catalog: DocumentCatalog | null
  manifest: DocumentSectionManifest
  suggestions: DocumentOrganizationSuggestions | null
  navigationMode: DocumentNavigationMode
  activeSection: string
  query: string
  viewPreferences: ProjectDocumentViewPreferences
}

export function useProjectDocumentNavigation(input: Input) {
  const { catalog, manifest, suggestions, navigationMode, activeSection, query, viewPreferences } = input
  const governanceSections = useMemo(() => buildDocumentSections(manifest)
    .filter((section) => !section.custom), [manifest])
  const knowledgeSections = useMemo(() => buildKnowledgeSections(catalog, manifest), [catalog, manifest])
  const baseSections = navigationMode === 'knowledge' ? knowledgeSections : governanceSections
  const governanceCounts = useMemo(() => {
    const counts = Object.fromEntries(governanceSections.map((section) => [section.key, 0])) as Record<string, number>
    for (const entry of catalog?.documents ?? []) {
      const section = governanceSectionForDocument(entry, manifest)
      counts[section] = (counts[section] ?? 0) + 1
    }
    counts['governance-overview'] = catalog?.documents.length ?? 0
    return counts
  }, [catalog, governanceSections, manifest])
  const sectionCounts = useMemo(() => {
    const counts = navigationMode === 'knowledge'
      ? knowledgeSectionCounts(catalog, manifest, knowledgeSections)
      : { ...governanceCounts }
    counts.suggestions = suggestions
      ? suggestions.proposed_sections.length + suggestions.assignments.length
        + suggestions.file_operations.filter((operation) => operation.status === 'proposed').length || 1
      : 0
    counts[DOCUMENT_HEALTH_SECTION] = catalog?.analysis?.governance_workflow?.total_issues
      ?? catalog?.analysis?.quality.summary.total_issues ?? 0
    return counts
  }, [catalog, governanceCounts, knowledgeSections, manifest, navigationMode, suggestions])
  const sections = useMemo(() => navigationMode === 'knowledge'
    ? sortHierarchicalSections(baseSections, viewPreferences.sectionSort, sectionCounts)
    : baseSections, [baseSections, navigationMode, sectionCounts, viewPreferences.sectionSort])
  const activeSectionDefinition = sections.find((section) => section.key === activeSection) ?? sections[0]
  const sectionDocuments = useMemo(() => (catalog?.documents ?? []).filter((entry) => navigationMode === 'knowledge'
    ? topicSectionsForDocument(entry, catalog, manifest).includes(activeSection)
    : governanceSectionForDocument(entry, manifest) === activeSection),
  [activeSection, catalog, manifest, navigationMode])
  const visibleDocuments = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase()
    const documents = sectionDocuments.filter((entry) => !normalizedQuery
      || entry.title.toLowerCase().includes(normalizedQuery)
      || entry.path.toLowerCase().includes(normalizedQuery))
    return sortDocuments(documents, manifest, viewPreferences.documentSort)
  }, [manifest, query, sectionDocuments, viewPreferences.documentSort])
  return {
    governanceSections, knowledgeSections, baseSections, governanceCounts, sectionCounts,
    sections, activeSectionDefinition, sectionDocuments, visibleDocuments,
  }
}
