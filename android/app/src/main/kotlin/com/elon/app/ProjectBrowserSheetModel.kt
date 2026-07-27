package com.elon.app

import android.graphics.drawable.Drawable

internal data class ProjectBrowserSheetDependencies(
    val projects: () -> List<AppProject>,
    val openProject: (Int) -> Unit,
    val selectableForeground: () -> Drawable?
)

internal data class IndexedBrowserProject(
    val index: Int,
    val project: AppProject
)

internal data class ProjectBrowserGroups(
    val personal: List<IndexedBrowserProject>,
    val joint: List<IndexedBrowserProject>
)

internal const val PROJECT_BROWSER_COLUMN_COUNT = 4
internal const val PROJECT_BROWSER_COLLAPSED_ROW_COUNT = 2
internal const val PROJECT_BROWSER_COLLAPSED_ITEM_COUNT =
    PROJECT_BROWSER_COLUMN_COUNT * PROJECT_BROWSER_COLLAPSED_ROW_COUNT

internal fun projectBrowserGridRows(
    entries: List<IndexedBrowserProject>
): List<List<IndexedBrowserProject?>> = entries
    .chunked(PROJECT_BROWSER_COLUMN_COUNT)
    .map { row ->
        List(PROJECT_BROWSER_COLUMN_COUNT) { column -> row.getOrNull(column) }
    }

internal fun visibleProjectBrowserEntries(
    entries: List<IndexedBrowserProject>,
    expanded: Boolean
): List<IndexedBrowserProject> =
    if (expanded) entries else entries.take(PROJECT_BROWSER_COLLAPSED_ITEM_COUNT)

private fun AppProject.isGroupBrowserProject(): Boolean {
    return !isSystemArchiveProject() &&
        (isJointDevelopmentProject() || (memberCount ?: 0) > 1)
}

internal fun groupProjectsForBrowser(
    projects: List<AppProject>,
    query: String
): ProjectBrowserGroups {
    val normalizedQuery = query.trim()
    val matching = projects
        .mapIndexed(::IndexedBrowserProject)
        .filter { item ->
            normalizedQuery.isEmpty() || listOf(
                item.project.title,
                item.project.subtitle,
                item.project.ownerAccount,
                item.project.projectDescription
            ).any { value -> value?.contains(normalizedQuery, ignoreCase = true) == true }
        }

    return ProjectBrowserGroups(
        personal = matching
            .filterNot { it.project.isGroupBrowserProject() }
            .sortedWith(
                compareByDescending<IndexedBrowserProject> { it.project.isSystemArchiveProject() }
                    .thenByDescending { it.project.updatedAt }
            ),
        joint = matching
            .filter { it.project.isGroupBrowserProject() }
            .sortedByDescending { it.project.updatedAt }
    )
}
