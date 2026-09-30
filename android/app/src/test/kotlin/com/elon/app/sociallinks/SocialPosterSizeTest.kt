package com.elon.app.sociallinks

import org.junit.Assert.assertEquals
import org.junit.Test

class SocialPosterSizeTest {
    @Test fun coversUseRatioResolutionAndCompactBounds() {
        assertEquals(208, SocialPosterSize.widthDp(480, 640, 3f))
        assertEquals(144, SocialPosterSize.widthDp(192, 256, 3f))
        assertEquals(158, SocialPosterSize.widthDp(900, 1600, 2f))
        assertEquals(280, SocialPosterSize.widthDp(1280, 720, 2f))
        assertEquals(220, SocialPosterSize.widthDp(640, 640, 2f))
        assertEquals(140, SocialPosterSize.widthDp(32, 64, 2f))
        assertEquals(208, SocialPosterSize.widthDp(0, 0, 2f))
    }
}
