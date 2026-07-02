package com.elon.app

import android.animation.ValueAnimator
import android.content.ClipData
import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import kotlin.math.hypot
import kotlin.math.max

internal class ChatProjectSideMenuView(
    context: Context,
    private val projects: () -> List<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val activeConversationIndex: () -> Int,
    private val openPersonalProject: (Int) -> Unit,
    private val openJointProject: (Int) -> Unit,
    private val openRecentConversation: (Int, Int) -> Unit,
    private val isRecentConversationWorking: (Int, Int) -> Boolean,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) : ScrollView(context) {
    private val content = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(32), dp(92), dp(18), dp(18))
    }
    private var personalProjectsExpanded = false
    private var jointProjectsExpanded = false
    private var recentConversationsExpanded = true

    init {
        overScrollMode = OVER_SCROLL_NEVER
        isVerticalScrollBarEnabled = false
        isFillViewport = false
        addView(
            content,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            )
        )
    }

    fun render() {
        content.removeAllViews()
        addPersonalProjects()
        addJointProjects()
        addRecentConversations()
    }

    private fun addPersonalProjects() {
        content.addView(sectionHeader("个人项目", personalProjectsExpanded) {
            personalProjectsExpanded = !personalProjectsExpanded
            render()
        })
        if (!personalProjectsExpanded) return

        val list = projects()
            .mapIndexed { index, project -> index to project }
            .filter { (_, project) -> !project.isJointDevelopmentProject() }
            .sortedWith(
                compareByDescending<Pair<Int, AppProject>> { it.second.isSystemArchiveProject() }
                    .thenByDescending { it.second.updatedAt }
            )
        if (list.isEmpty()) {
            content.addView(emptyRow("暂无个人项目"))
            return
        }
        list.forEach { (index, project) ->
            content.addView(projectNameRow(project, active = index == activeProjectIndex()) {
                requestClose(true)
                postDelayed({ openPersonalProject(index) }, CLOSE_DELAY_MS)
            })
        }
        content.addView(space(34))
    }

    private fun addJointProjects() {
        content.addView(sectionHeader("联合项目", jointProjectsExpanded) {
            jointProjectsExpanded = !jointProjectsExpanded
            render()
        })
        if (!jointProjectsExpanded) return

        val jointProjects = projects()
            .mapIndexed { index, project -> index to project }
            .filter { (_, project) -> project.isJointDevelopmentProject() }
            .sortedByDescending { it.second.updatedAt }
        if (jointProjects.isEmpty()) {
            content.addView(emptyRow("暂无联合项目"))
            return
        }
        jointProjects.forEach { (index, project) ->
            content.addView(jointProjectRow(index, project))
        }
    }

    private fun addRecentConversations() {
        val topGap = if (!personalProjectsExpanded && !jointProjectsExpanded) 132 else 28
        content.addView(space(topGap))
        content.addView(sectionHeader("最近会话", recentConversationsExpanded, showFolderIcon = false) {
            recentConversationsExpanded = !recentConversationsExpanded
            render()
        })
        if (!recentConversationsExpanded) return

        val items = recentConversationItems()
        if (items.isEmpty()) {
            content.addView(emptyRow("暂无最近会话"))
            return
        }
        items.forEach { entry ->
            content.addView(recentConversationRow(entry))
        }
    }

    private fun sectionHeader(
        title: String,
        expanded: Boolean,
        showFolderIcon: Boolean = true,
        onClick: () -> Unit
    ): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(46)
            ).apply {
                topMargin = dp(2)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            foreground = selectableForeground()
            contentDescription = if (expanded) "收起$title" else "展开$title"
            addView(menuText(title).apply {
                setTextColor(Color.parseColor("#D6D6D6"))
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.MATCH_PARENT
                )
            })
            if (showFolderIcon) {
                addView(sectionFolderIcon(expanded))
            }
            setOnClickListener { onClick() }
        }
    }

    private fun sectionFolderIcon(expanded: Boolean): ImageView {
        return ImageView(context).apply {
            setImageResource(
                if (expanded) {
                    R.drawable.ic_side_menu_folder_open
                } else {
                    R.drawable.ic_side_menu_folder_closed
                }
            )
            imageTintList = ColorStateList.valueOf(Color.parseColor("#D6D6D6"))
            scaleType = ImageView.ScaleType.FIT_CENTER
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            layoutParams = LinearLayout.LayoutParams(dp(28), dp(28)).apply {
                leftMargin = dp(16)
            }
        }
    }

    private fun recentConversationItems(): List<RecentConversationEntry> {
        return projects().flatMapIndexed { projectIndex, project ->
            project.conversations.mapIndexedNotNull { conversationIndex, conversation ->
                conversation.title.trim().takeIf { it.isNotBlank() }?.let { title ->
                    RecentConversationEntry(
                        projectIndex = projectIndex,
                        conversationIndex = conversationIndex,
                        title = title,
                        updatedAt = conversation.updatedAt,
                        ended = conversation.ended,
                        working = isRecentConversationWorking(projectIndex, conversationIndex)
                    )
                }
            }
        }.sortedWith(
            compareByDescending<RecentConversationEntry> { conversationWorkingSortKey(it.working) }
                .thenByDescending { conversationOpenSortKey(it.ended) }
                .thenByDescending { it.updatedAt }
                .thenBy { it.title }
        )
            .take(RECENT_CONVERSATION_LIMIT)
    }

    private fun projectNameRow(project: AppProject, active: Boolean, onClick: () -> Unit): TextView {
        return menuText(project.title).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
            setPadding(dp(10), 0, dp(10), 0)
            isClickable = true
            foreground = selectableForeground()
            if (active) {
                background = GradientDrawable().apply {
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#222222"))
                }
            }
            setOnClickListener { onClick() }
            setOnLongClickListener {
                startProjectDrag(it, project.toChatProjectShare())
                true
            }
        }
    }

    private fun recentConversationRow(entry: RecentConversationEntry): TextView {
        val active = entry.projectIndex == activeProjectIndex() &&
            entry.conversationIndex == activeConversationIndex()
        return menuText(entry.title).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
            setPadding(dp(10), 0, dp(10), 0)
            isClickable = true
            foreground = selectableForeground()
            setTextColor(Color.parseColor(if (active) "#D6D6D6" else "#A8A8A8"))
            if (active) {
                background = GradientDrawable().apply {
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#222222"))
                }
            }
            setOnClickListener {
                requestClose(true)
                postDelayed({
                    openRecentConversation(entry.projectIndex, entry.conversationIndex)
                }, CLOSE_DELAY_MS)
            }
        }
    }

    private fun jointProjectRow(index: Int, project: AppProject): TextView {
        return projectNameRow(project, active = index == activeProjectIndex()) {
            requestClose(true)
            postDelayed({ openJointProject(index) }, CLOSE_DELAY_MS)
        }
    }

    private fun startProjectDrag(source: View, share: ChatProjectShare) {
        val clip = ClipData.newPlainText("project", share.toMessageText())
        source.startDragAndDrop(clip, View.DragShadowBuilder(source), share, 0)
    }

    private fun emptyRow(text: String): TextView {
        return menuText(text).apply {
            setTextColor(Color.parseColor("#777777"))
            textSize = 14f
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(34))
        }
    }

    private fun menuText(title: String): TextView {
        return TextView(context).apply {
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = title
            setTextColor(Color.parseColor("#A8A8A8"))
            textSize = 17f
        }
    }

    private fun space(heightDp: Int): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(heightDp))
        }
    }

    private companion object {
        const val CLOSE_DELAY_MS = 220L
        const val RECENT_CONVERSATION_LIMIT = 8
    }
}

private data class RecentConversationEntry(
    val projectIndex: Int,
    val conversationIndex: Int,
    val title: String,
    val updatedAt: Long,
    val ended: Boolean,
    val working: Boolean
)

internal fun showChatProjectDropRipple(
    overlay: View,
    contentContainer: ViewGroup,
    share: ChatProjectShare,
    overlayX: Float,
    overlayY: Float
) {
    val overlayLocation = IntArray(2)
    val contentLocation = IntArray(2)
    overlay.getLocationOnScreen(overlayLocation)
    contentContainer.getLocationOnScreen(contentLocation)
    val localX = overlayLocation[0] + overlayX - contentLocation[0]
    val localY = overlayLocation[1] + overlayY - contentLocation[1]
    val ripple = ChatProjectDropRippleView(contentContainer.context, projectCardPaletteFor(share.id).first())
    contentContainer.addView(
        ripple,
        FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        )
    )
    ripple.start(localX, localY) {
        (ripple.parent as? ViewGroup)?.removeView(ripple)
    }
}

private class ChatProjectDropRippleView(
    context: Context,
    private val baseColor: Int
) : View(context) {
    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeWidth = context.resources.displayMetrics.density * 1.4f
    }
    private var centerX = 0f
    private var centerY = 0f
    private var fraction = 0f

    fun start(x: Float, y: Float, onEnd: () -> Unit) {
        centerX = x
        centerY = y
        ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 620L
            interpolator = LinearInterpolator()
            addUpdateListener {
                fraction = it.animatedFraction
                invalidate()
            }
            addListener(object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: android.animation.Animator) {
                    onEnd()
                }
            })
            start()
        }
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val maxRadius = max(
            hypot(centerX.toDouble(), centerY.toDouble()),
            hypot((width - centerX).toDouble(), (height - centerY).toDouble())
        ).toFloat()
        val radius = maxRadius * fraction
        val alpha = ((1f - fraction) * 78).toInt().coerceIn(0, 78)
        fillPaint.color = withAlpha(baseColor, alpha)
        strokePaint.color = withAlpha(baseColor, (alpha * 1.4f).toInt().coerceIn(0, 110))
        canvas.drawCircle(centerX, centerY, radius, fillPaint)
        canvas.drawCircle(centerX, centerY, radius * 0.72f, strokePaint)
    }

    private fun withAlpha(color: Int, alpha: Int): Int {
        return Color.argb(alpha, Color.red(color), Color.green(color), Color.blue(color))
    }
}
