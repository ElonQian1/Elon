package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Rect
import android.graphics.RectF
import android.graphics.Typeface
import android.util.TypedValue
import android.view.View
import kotlin.math.cos
import kotlin.math.sin

internal class ProjectPlazaPatternView(
    context: Context,
    projects: List<StoreProject>
) : View(context) {
    private data class BannerSlot(
        val left: Float,
        val top: Float,
        val size: Float
    )

    private data class BannerPoint(
        val x: Float,
        val y: Float
    )

    private val bannerRotation = -14f
    private val sortedProjects = projects
        .sortedWith(compareByDescending<StoreProject> { it.memberCount }.thenBy { it.displayTitle() })
        .take(14)
    private val density = resources.displayMetrics.density
    private val bgPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#0E1116")
    }
    private val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#667B8793")
        strokeWidth = dp(1).toFloat()
        alpha = 130
    }
    private val tilePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#D5D5D5")
    }
    private val iconTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#253140")
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }
    private val lensPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#9FA1A6")
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        alpha = 210
    }
    private val tileRect = RectF()
    private val bitmapSource = Rect()
    private val clipPath = Path()

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), bgPaint)
        drawGrid(canvas)
        drawProjectIcons(canvas)
        drawMagnifier(canvas)
    }

    private fun drawGrid(canvas: Canvas) {
        val step = dp(54)
        var x = 0
        while (x <= width) {
            canvas.drawLine(x.toFloat(), 0f, x.toFloat(), height.toFloat(), gridPaint)
            x += step
        }
        var y = 0
        while (y <= height) {
            canvas.drawLine(0f, y.toFloat(), width.toFloat(), y.toFloat(), gridPaint)
            y += step
        }
    }

    private fun drawProjectIcons(canvas: Canvas) {
        val slots = buildBannerSlots()
        val assignments = assignProjects(slots)

        canvas.save()
        canvas.rotate(bannerRotation, width / 2f, height / 2f)
        slots.forEachIndexed { index, slot ->
            if (index != FOCUS_SLOT_INDEX) {
                drawProjectIcon(canvas, assignments[index], slot)
            }
        }
        if (slots.isNotEmpty()) {
            drawProjectIcon(canvas, assignments[FOCUS_SLOT_INDEX], slots[FOCUS_SLOT_INDEX])
        }
        canvas.restore()
    }

    private fun drawProjectIcon(canvas: Canvas, project: StoreProject?, slot: BannerSlot) {
        val radius = dp(5).toFloat()
        tileRect.set(slot.left, slot.top, slot.left + slot.size, slot.top + slot.size)
        canvas.drawRoundRect(tileRect, radius, radius, tilePaint)

        val icon = UserProfileStore.decodeAvatar(project?.iconDataUrl)
        if (icon != null) {
            drawBitmapIcon(canvas, icon, tileRect, radius)
        } else if (project != null) {
            drawInitialIcon(canvas, project, tileRect)
        }
    }

    private fun buildBannerSlots(): List<BannerSlot> {
        val tileSize = dp(50).toFloat()
        val focusSize = dp(72).toFloat()
        val gapX = dp(76).toFloat()
        val gapY = dp(66).toFloat()
        val focusCenter = inverseRotatedPoint(width * 0.60f, height * 0.43f)
        val slots = mutableListOf<BannerSlot>()
        slots += BannerSlot(
            focusCenter.x - focusSize / 2f,
            focusCenter.y - focusSize / 2f,
            focusSize
        )
        for (row in -3..3) {
            for (column in -5..5) {
                if (row == 0 && column == 0) continue
                val offsetX = if (row % 2 == 0) 0f else gapX / 2f
                val cx = focusCenter.x + column * gapX + offsetX
                val cy = focusCenter.y + row * gapY
                val slot = BannerSlot(cx - tileSize / 2f, cy - tileSize / 2f, tileSize)
                if (isVisibleSlot(slot)) {
                    slots += slot
                }
            }
        }
        return slots
    }

    private fun assignProjects(slots: List<BannerSlot>): Map<Int, StoreProject> {
        if (sortedProjects.isEmpty()) return emptyMap()
        val assignments = mutableMapOf<Int, StoreProject>()
        if (slots.isNotEmpty()) {
            assignments[FOCUS_SLOT_INDEX] = sortedProjects.first()
        }
        val restProjects = sortedProjects.drop(1)
        val orderedSlots = slots.indices
            .filter { it != FOCUS_SLOT_INDEX }
            .sortedWith(compareBy({ rotatedCenter(slots[it]).y }, { rotatedCenter(slots[it]).x }))
        restProjects.forEachIndexed { index, project ->
            val slotIndex = orderedSlots.getOrNull(index) ?: return@forEachIndexed
            assignments[slotIndex] = project
        }
        return assignments
    }

    private fun isVisibleSlot(slot: BannerSlot): Boolean {
        val center = rotatedCenter(slot)
        val margin = slot.size * 1.2f
        return center.x >= -margin &&
            center.x <= width + margin &&
            center.y >= -margin &&
            center.y <= height + margin
    }

    private fun inverseRotatedPoint(screenX: Float, screenY: Float): BannerPoint {
        return rotatePoint(screenX, screenY, -bannerRotation)
    }

    private fun rotatedCenter(slot: BannerSlot): BannerPoint {
        val cx = slot.left + slot.size / 2f
        val cy = slot.top + slot.size / 2f
        return rotatePoint(cx, cy, bannerRotation)
    }

    private fun rotatePoint(x: Float, y: Float, angle: Float): BannerPoint {
        val originX = width / 2f
        val originY = height / 2f
        val radians = Math.toRadians(angle.toDouble())
        val dx = (x - originX).toDouble()
        val dy = (y - originY).toDouble()
        val screenX = dx * cos(radians) - dy * sin(radians) + originX
        val screenY = dx * sin(radians) + dy * cos(radians) + originY
        return BannerPoint(screenX.toFloat(), screenY.toFloat())
    }

    private fun drawBitmapIcon(canvas: Canvas, bitmap: Bitmap, rect: RectF, radius: Float) {
        val sourceSize = minOf(bitmap.width, bitmap.height)
        val left = (bitmap.width - sourceSize) / 2
        val top = (bitmap.height - sourceSize) / 2
        bitmapSource.set(left, top, left + sourceSize, top + sourceSize)
        clipPath.reset()
        clipPath.addRoundRect(rect, radius, radius, Path.Direction.CW)
        canvas.save()
        canvas.clipPath(clipPath)
        canvas.drawBitmap(bitmap, bitmapSource, rect, null)
        canvas.restore()
    }

    private fun drawInitialIcon(canvas: Canvas, project: StoreProject, rect: RectF) {
        iconTextPaint.textSize = sp(24)
        val text = avatarText(project.displayTitle())
        val metrics = iconTextPaint.fontMetrics
        val baseline = rect.centerY() - (metrics.ascent + metrics.descent) / 2f
        canvas.drawText(text, rect.centerX(), baseline, iconTextPaint)
    }

    private fun drawMagnifier(canvas: Canvas) {
        val cx = width * 0.60f
        val cy = height * 0.43f
        lensPaint.strokeWidth = dp(8).toFloat()
        canvas.drawCircle(cx, cy, dp(39).toFloat(), lensPaint)
        canvas.drawLine(
            cx + dp(29),
            cy + dp(29),
            cx + dp(64),
            cy + dp(64),
            lensPaint
        )
    }

    private fun dp(value: Int): Int = (value * density + 0.5f).toInt()

    private fun sp(value: Int): Float =
        TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, value.toFloat(), resources.displayMetrics)

    private companion object {
        private const val FOCUS_SLOT_INDEX = 0
    }
}
