package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.Rect
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.animation.DecelerateInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import kotlin.math.cos
import kotlin.math.sin

internal class ProjectManagementHomeView(
    private val activity: AppCompatActivity,
    private val container: LinearLayout,
    private val projects: () -> List<AppProject>,
    private val plazaProjects: () -> List<StoreProject>,
    private val activeProjectIndex: () -> Int,
    private val personalProjectsExpanded: () -> Boolean,
    private val jointProjectsExpanded: () -> Boolean,
    private val setPersonalProjectsExpanded: (Boolean) -> Unit,
    private val setJointProjectsExpanded: (Boolean) -> Unit,
    private val formatTime: (Long) -> String,
    private val openProject: (Int) -> Unit,
    private val showProjectActions: (Int, View?) -> Unit,
    private val showCreateProjectDialog: () -> Unit,
    private val showProjectPlaza: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private data class IndexedProject(
        val index: Int,
        val project: AppProject
    )

    fun render() {
        container.removeAllViews()
        container.setBackgroundColor(Color.parseColor("#101010"))
        container.addView(createPlazaBanner())

        val indexed = projects().mapIndexed { index, project -> IndexedProject(index, project) }
        val personal = indexed.filter { !it.project.isJointDevelopmentProject() }
        val joint = indexed.filter { it.project.isJointDevelopmentProject() }

        addSection(
            title = "个人项目",
            items = personal,
            topMargin = 4,
            emptyAction = showCreateProjectDialog,
            expanded = personalProjectsExpanded(),
            setExpanded = setPersonalProjectsExpanded
        )
        addSection(
            title = "联合项目",
            items = joint,
            topMargin = 4,
            emptyAction = null,
            expanded = jointProjectsExpanded(),
            setExpanded = setJointProjectsExpanded
        )
        container.addView(bottomSpacer())
    }

    private fun createPlazaBanner(): View {
        val contentWidth = activity.resources.displayMetrics.widthPixels - dp(16)
        val bannerHeight = (contentWidth * 0.36f).toInt().coerceIn(dp(124), dp(158))
        val bannerProjects = plazaProjects()
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                bannerHeight
            ).apply {
                marginStart = dp(8)
                marginEnd = dp(8)
                topMargin = dp(12)
            }
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showProjectPlaza() }

            addView(ProjectPlazaPatternView(activity, bannerProjects), FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "项目广场"
                setPadding(dp(4), dp(3), dp(4), dp(3))
                background = rect("#660F1217")
                setTextColor(Color.parseColor("#F2F5FA"))
                alpha = 0.92f
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
                setShadowLayer(dp(2).toFloat(), 0f, dp(1).toFloat(), Color.parseColor("#AA000000"))
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.START or Gravity.TOP
                leftMargin = dp(3)
                topMargin = dp(8)
            })

        }
    }

    private fun addSection(
        title: String,
        items: List<IndexedProject>,
        topMargin: Int,
        emptyAction: (() -> Unit)?,
        expanded: Boolean,
        setExpanded: (Boolean) -> Unit
    ) {
        val canExpand = items.size > COLLAPSED_PROJECT_LIMIT
        val initiallyExpanded = expanded && canExpand
        val gridContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            clipChildren = true
        }
        lateinit var header: LinearLayout
        header = createSectionHeader(title, canExpand, initiallyExpanded) { arrow ->
            val targetExpanded = !arrow.isSelected
            setExpanded(targetExpanded)
            header.contentDescription = "${title}${if (targetExpanded) "收起" else "展开"}"
            header.isEnabled = false
            animateSectionGrid(
                gridContainer = gridContainer,
                allItems = items,
                emptyAction = emptyAction,
                expanded = targetExpanded,
                arrow = arrow
            ) {
                header.isEnabled = true
            }
        }
        container.addView(header, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(42)
        ).apply {
            marginStart = dp(8)
            marginEnd = dp(8)
            this.topMargin = dp(topMargin)
        })
        container.addView(gridContainer, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ))
        addProjectGrid(gridContainer, sectionItems(items, initiallyExpanded), emptyAction)
    }

    private fun createSectionHeader(
        title: String,
        canExpand: Boolean,
        expanded: Boolean,
        onToggle: (TextView) -> Unit
    ): LinearLayout {
        val arrow = TextView(activity).apply {
            includeFontPadding = false
            text = "›"
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#A6AFBD"))
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 21f)
            isSelected = expanded
            rotation = if (expanded) 90f else 0f
            visibility = if (canExpand) View.VISIBLE else View.INVISIBLE
        }
        return LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(6), 0, 0, 0)
            if (canExpand) {
                isClickable = true
                foreground = selectableForeground()
                contentDescription = "${title}${if (expanded) "收起" else "展开"}"
                setOnClickListener { onToggle(arrow) }
            }
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#F2F5FA"))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 16f)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(arrow, LinearLayout.LayoutParams(dp(24), LinearLayout.LayoutParams.MATCH_PARENT))
        }
    }

    private fun sectionItems(items: List<IndexedProject>, expanded: Boolean): List<IndexedProject> {
        if (expanded || items.size <= COLLAPSED_PROJECT_LIMIT) return items
        return items.take(COLLAPSED_PROJECT_LIMIT)
    }

    private fun addProjectGrid(
        target: LinearLayout,
        items: List<IndexedProject>,
        emptyAction: (() -> Unit)?
    ) {
        val cells = when {
            items.isEmpty() -> listOf<IndexedProject?>(null, null)
            items.size % 2 == 0 -> items
            else -> items + listOf(null)
        }
        cells.chunked(2).forEachIndexed { rowIndex, rowItems ->
            target.addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.TOP
                rowItems.forEachIndexed { cellIndex, indexed ->
                    val card = indexed?.let { createProjectCard(it) } ?: createEmptyProjectSlot(emptyAction)
                    addView(card, LinearLayout.LayoutParams(
                        0,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        1f
                    ).apply {
                        if (cellIndex == 0) marginEnd = dp(4)
                        else marginStart = dp(4)
                    })
                }
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(8)
                marginEnd = dp(8)
                topMargin = if (rowIndex == 0) dp(6) else dp(14)
            })
        }
    }

    private fun animateSectionGrid(
        gridContainer: LinearLayout,
        allItems: List<IndexedProject>,
        emptyAction: (() -> Unit)?,
        expanded: Boolean,
        arrow: TextView,
        onFinished: () -> Unit
    ) {
        val startHeight = gridContainer.height.takeIf { it > 0 }
            ?: measureProjectGridHeight(sectionItems(allItems, !expanded), emptyAction)
        val targetItems = sectionItems(allItems, expanded)
        val targetHeight = measureProjectGridHeight(targetItems, emptyAction)
        gridContainer.animate().cancel()
        arrow.animate().cancel()
        arrow.isSelected = expanded
        arrow.animate()
            .rotation(if (expanded) 90f else 0f)
            .setDuration(180L)
            .setInterpolator(DecelerateInterpolator())
            .start()

        if (expanded) {
            gridContainer.removeAllViews()
            addProjectGrid(gridContainer, targetItems, emptyAction)
        }
        gridContainer.layoutParams = gridContainer.layoutParams.apply { height = startHeight }
        gridContainer.requestLayout()

        ValueAnimator.ofInt(startHeight, targetHeight).apply {
            duration = SECTION_ANIMATION_MS
            interpolator = DecelerateInterpolator()
            addUpdateListener { animator ->
                gridContainer.layoutParams = gridContainer.layoutParams.apply {
                    height = animator.animatedValue as Int
                }
                gridContainer.requestLayout()
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    if (!expanded) {
                        gridContainer.removeAllViews()
                        addProjectGrid(gridContainer, targetItems, emptyAction)
                    }
                    gridContainer.layoutParams = gridContainer.layoutParams.apply {
                        height = LinearLayout.LayoutParams.WRAP_CONTENT
                    }
                    gridContainer.requestLayout()
                    onFinished()
                }
            })
            start()
        }
    }

    private fun measureProjectGridHeight(
        items: List<IndexedProject>,
        emptyAction: (() -> Unit)?
    ): Int {
        val temp = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            visibility = View.INVISIBLE
        }
        addProjectGrid(temp, items, emptyAction)
        val width = container.width.takeIf { it > 0 } ?: activity.resources.displayMetrics.widthPixels
        temp.measure(
            View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED)
        )
        return temp.measuredHeight
    }

    private fun createProjectCard(item: IndexedProject): View {
        val project = item.project
        val isActive = item.index == activeProjectIndex()
        return SquareProjectCardFrame(activity).apply {
            background = rect("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(item.index) }
            setOnLongClickListener { anchor ->
                showProjectActions(item.index, anchor)
                true
            }

            addView(projectThumbnail(project), FrameLayout.LayoutParams(dp(38), dp(38)).apply {
                gravity = Gravity.START or Gravity.TOP
                leftMargin = dp(10)
                topMargin = dp(17)
            })

            addView(createProjectInfoBar(project, isActive), FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(47),
                Gravity.BOTTOM
            ))
        }
    }

    private fun projectThumbnail(project: AppProject): View {
        return FrameLayout(activity).apply {
            contentDescription = "${project.title.ifBlank { "项目" }}封面"
            background = GradientDrawable().apply {
                cornerRadius = dp(6).toFloat()
                setColor(Color.parseColor("#D2D2D2"))
            }
            val iconBitmap = UserProfileStore.decodeAvatar(project.iconDataUrl)
            if (iconBitmap != null) {
                addView(ImageView(activity).apply {
                    setImageBitmap(iconBitmap)
                    scaleType = ImageView.ScaleType.CENTER_CROP
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            } else {
                addView(TextView(activity).apply {
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    text = avatarText(project.title.ifBlank { "项目" })
                    setTextColor(Color.parseColor("#253140"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 17f)
                    setTypeface(typeface, Typeface.BOLD)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun createProjectInfoBar(project: AppProject, active: Boolean): View {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(10), dp(5), dp(10), dp(5))
            background = rect(if (active) "#283345" else "#253140")

            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = project.title.ifBlank { "未命名项目" }
                    maxLines = 1
                    ellipsize = TextUtils.TruncateAt.END
                    setTextColor(Color.parseColor("#F2F5FA"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 14.2f)
                    setTypeface(typeface, Typeface.BOLD)
                }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = projectTime(project)
                    maxLines = 1
                    setTextColor(Color.parseColor("#DDE8FC"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 12.2f)
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    marginStart = dp(7)
                })
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = projectMeta(project)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor("#DDE8FC"))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 12.6f)
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(5)
            })
        }
    }

    private fun projectMeta(project: AppProject): String {
        val kind = if (project.isJointDevelopmentProject()) "联合开发" else "个人独立"
        val stage = project.stage.takeIf { it.isNotBlank() } ?: "待提交需求"
        return "$kind · ${project.conversations.size}个会话 · $stage"
    }

    private fun projectTime(project: AppProject): String {
        if (project.updatedAt <= 0L) return "时间"
        return formatTime(project.updatedAt).ifBlank { "时间" }
    }

    private fun createEmptyProjectSlot(emptyAction: (() -> Unit)?): View {
        return SquareProjectCardFrame(activity).apply {
            background = rect("#181B20")
            emptyAction?.let { action ->
                contentDescription = "新建项目"
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { action() }
                addView(TextView(activity).apply {
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    text = "+"
                    setTextColor(Color.parseColor("#A6AFBD"))
                    setTextSize(TypedValue.COMPLEX_UNIT_SP, 34f)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
            }
        }
    }

    private fun bottomSpacer(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(34)
            )
        }
    }

    private fun rect(color: String): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            setColor(Color.parseColor(color))
        }
    }

    private companion object {
        const val COLLAPSED_PROJECT_LIMIT = 4
        const val SECTION_ANIMATION_MS = 260L
    }
}

private class SquareProjectCardFrame(context: Context) : FrameLayout(context) {
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val exactHeight = MeasureSpec.makeMeasureSpec(width, MeasureSpec.EXACTLY)
        super.onMeasure(widthMeasureSpec, exactHeight)
    }
}

private class ProjectPlazaPatternView(
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
        .sortedWith(compareByDescending<StoreProject> { it.memberCount }.thenBy { it.name })
        .take(14)
    private val density = resources.displayMetrics.density
    private val bgPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#181B20")
    }
    private val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#1E2126")
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
            drawInitialIcon(canvas, project, tileRect, slot.size)
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

    private fun drawInitialIcon(canvas: Canvas, project: StoreProject, rect: RectF, size: Float) {
        iconTextPaint.textSize = size * 0.36f
        val text = avatarText(project.name.ifBlank { "项目" })
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

    private companion object {
        private const val FOCUS_SLOT_INDEX = 0
    }
}
