package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ProjectPlazaCachePolicyTest {
    private val snapshot = ProjectPlazaSnapshot(
        projects = emptyList(),
        joinedIds = emptySet(),
        savedAtMillis = 10_000L
    )

    @Test
    fun snapshotRemainsFreshForOneMinute() {
        assertTrue(isProjectPlazaSnapshotFresh(snapshot, 10_000L + PROJECT_PLAZA_FRESH_MS))
        assertFalse(isProjectPlazaSnapshotFresh(snapshot, 10_001L + PROJECT_PLAZA_FRESH_MS))
        assertFalse(isProjectPlazaSnapshotFresh(snapshot, 9_999L))
    }

    @Test
    fun skeletonWaitsAndNeverCoversVisibleContent() {
        val startedAt = 1_000L
        assertFalse(shouldShowProjectPlazaSkeleton(false, startedAt, startedAt + 179L))
        assertTrue(shouldShowProjectPlazaSkeleton(false, startedAt, startedAt + 180L))
        assertFalse(shouldShowProjectPlazaSkeleton(true, startedAt, startedAt + 5_000L))
    }
}
