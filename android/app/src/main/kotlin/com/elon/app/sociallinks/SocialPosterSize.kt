package com.elon.app.sociallinks

import kotlin.math.roundToInt

/** Same compact, resolution-aware bounds as the shared web/PWA poster cards. */
internal object SocialPosterSize {
    fun widthDp(width: Int, height: Int, density: Float): Int {
        val ready = width > 0 && height > 0
        val ratio = if (ready) (width.toFloat() / height).coerceIn(.5f, 2f) else .75f
        val limit = when { ratio < 1 -> 208f; ratio == 1f -> 220f; else -> 280f }
        val pixels = if (ready) width / density.coerceIn(1f, 2f) else limit
        return minOf(limit, 280f * ratio, maxOf(144f, pixels)).roundToInt()
    }
}
