package com.elon.app

import android.graphics.PointF
import android.view.View
import android.widget.TextView
import com.davemorrissey.labs.subscaleview.SubsamplingScaleImageView

/** Reading is a viewport policy; original file, region decoder and cache remain unchanged. */
internal class ChatImageReadingController(
    private val image: SubsamplingScaleImageView,
    private val toggle: TextView
) {
    private var reading = false
    private var initialized = false
    private var previous: ChatImageReadingGeometry? = null
    private var sourceCenter: PointF? = null
    private var currentScale = 1f
    private val layoutListener = View.OnLayoutChangeListener { _, _, _, _, _, _, _, _, _ ->
        if (initialized && image.isReady) resize()
    }

    init {
        toggle.text = "长图阅读"
        toggle.contentDescription = "长图阅读"
        toggle.isEnabled = false
        toggle.setOnClickListener { select(!reading) }
        image.addOnLayoutChangeListener(layoutListener)
    }

    fun reset() { initialized = false; previous = null; sourceCenter = null; toggle.isEnabled = false }
    fun release() { image.removeOnLayoutChangeListener(layoutListener) }
    fun stateChanged() {
        sourceCenter = image.center
        currentScale = image.scale
    }

    fun ready() {
        if (initialized) return
        val geometry = geometry() ?: return
        initialized = true
        toggle.isEnabled = true
        select(geometry.isLong)
    }

    private fun geometry(): ChatImageReadingGeometry? {
        if (!image.isReady || image.width <= 0 || image.height <= 0) return null
        val rotated = image.appliedOrientation == 90 || image.appliedOrientation == 270
        return ChatImageReadingGeometry(
            if (rotated) image.sHeight else image.sWidth,
            if (rotated) image.sWidth else image.sHeight,
            image.width, image.height
        ).takeIf { it.sourceWidth > 0 && it.sourceHeight > 0 }
    }

    private fun select(value: Boolean) {
        val geometry = geometry() ?: return
        reading = value
        previous = geometry
        image.setMaxScale(geometry.maxScale)
        image.setDoubleTapZoomScale(maxOf(1f, geometry.widthScale * 2f))
        image.setScaleAndCenter(
            if (reading) geometry.widthScale else geometry.fitScale,
            PointF(geometry.sourceWidth / 2f, if (reading) geometry.topCenterY else geometry.sourceHeight / 2f)
        )
        toggle.text = if (reading) "整图" else "长图阅读"
        toggle.contentDescription = if (reading) "查看整图" else "长图阅读"
        toggle.isSelected = reading
    }

    private fun resize() {
        val next = geometry() ?: return
        val before = previous ?: return
        if (next == before) return
        val center = sourceCenter ?: image.center ?: return
        val scale = currentScale.coerceAtLeast(before.fitScale)
        previous = next
        val factor = if (reading) next.widthScale / before.widthScale else next.fitScale / before.fitScale
        val newScale = scale * factor
        val topRow = center.y - before.viewportHeight / (2f * scale)
        image.setMaxScale(next.maxScale)
        image.setScaleAndCenter(newScale, PointF(center.x,
            if (reading) topRow + next.viewportHeight / (2f * newScale) else center.y))
    }
}
