package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class ChatImageReadingGeometryTest {
    @Test fun normalPhotoKeepsOverview() {
        assertFalse(ChatImageReadingGeometry(1600, 1200, 1080, 1800).isLong)
        assertFalse(ChatImageReadingGeometry(1080, 2400, 1080, 1800).isLong)
    }
    @Test fun longScreenshotFitsWidthAndStartsAtTop() {
        val geometry = ChatImageReadingGeometry(1080, 6000, 1080, 1800)
        assertTrue(geometry.isLong)
        assertEquals(1f, geometry.widthScale, .0001f)
        assertEquals(.3f, geometry.fitScale, .0001f)
        assertEquals(900f, geometry.topCenterY, .0001f)
    }
    @Test fun ultraLongImageIsNotLimitedToEightTimesOverview() {
        val geometry = ChatImageReadingGeometry(1080, 50000, 720, 1500)
        assertTrue(geometry.widthScale / geometry.fitScale > 8f)
        assertTrue(geometry.maxScale > geometry.widthScale)
        assertEquals(1125f, geometry.topCenterY, .01f)
    }
    @Test fun narrowScreenshotCanUpscaleAndLandscapeRemainsPhoto() {
        assertEquals(2f, ChatImageReadingGeometry(360, 5000, 720, 1500).widthScale, .0001f)
        assertFalse(ChatImageReadingGeometry(6000, 1080, 1080, 1800).isLong)
    }
    @Test fun readingModeCentersShortImages() {
        val geometry = ChatImageReadingGeometry(1080, 500, 1080, 1800)
        assertEquals(250f, geometry.topCenterY, .0001f)
    }
}
