package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProjectPlazaCarouselTest {
    @Test
    fun scaleInterpolatesContinuouslyFromCenterToAdjacentCard() {
        assertEquals(1f, projectPlazaCardScale(0f, 300f), 0.0001f)
        assertEquals(0.98f, projectPlazaCardScale(150f, 300f), 0.0001f)
        assertEquals(PROJECT_PLAZA_CARD_MIN_SCALE, projectPlazaCardScale(300f, 300f), 0.0001f)
    }

    @Test
    fun scaleClampsDistanceAndMinimumBoundaries() {
        assertEquals(PROJECT_PLAZA_CARD_MIN_SCALE, projectPlazaCardScale(-900f, 300f), 0.0001f)
        assertEquals(1f, projectPlazaCardScale(0f, 0f), 0.0001f)
        assertEquals(PROJECT_PLAZA_CARD_MIN_SCALE, projectPlazaCardScale(1f, 0f), 0.0001f)
        assertEquals(1f, projectPlazaCardScale(300f, 300f, 2f), 0.0001f)
        assertEquals(0f, projectPlazaCardScale(300f, 300f, -1f), 0.0001f)
    }

    @Test
    fun nearestCardTracksDragAndSnapDestinationForAllListSizes() {
        assertNull(nearestProjectPlazaCardIndex(emptyList(), 120f))
        assertEquals(0, nearestProjectPlazaCardIndex(listOf(120f), 120f))
        assertEquals(0, nearestProjectPlazaCardIndex(listOf(80f, 220f), 120f))
        assertEquals(1, nearestProjectPlazaCardIndex(listOf(-40f, 118f, 280f), 120f))
        assertEquals(2, nearestProjectPlazaCardIndex(listOf(-300f, -140f, 121f, 282f), 120f))
    }

    @Test
    fun trailingPaddingLetsTheLastCardReachThePreviewCenterAfterResize() {
        assertEquals(107, projectPlazaTrailingPadding(360, 17, 236, 98))
        assertEquals(122, projectPlazaTrailingPadding(412, 20, 270, 98))
        assertEquals(98, projectPlazaTrailingPadding(320, 20, 220, 98))
    }

    @Test
    fun velocityProjectionCanAdvanceBeyondTheNearestRestingCard() {
        val offsets = listOf(0, 278, 556, 834)
        assertEquals(0, projectedProjectPlazaCardIndex(offsets, scrollX = 80, velocityX = 0))
        assertEquals(1, projectedProjectPlazaCardIndex(offsets, scrollX = 80, velocityX = 1_500))
        assertEquals(2, projectedProjectPlazaCardIndex(offsets, scrollX = 80, velocityX = 2_600))
        assertEquals(0, projectedProjectPlazaCardIndex(offsets, scrollX = 80, velocityX = -2_000))
        assertNull(projectedProjectPlazaCardIndex(emptyList(), scrollX = 0, velocityX = 1_000))
    }
}
