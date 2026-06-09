package com.elon.app

import android.animation.ValueAnimator
import android.content.ClipData
import android.content.Context
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
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import kotlin.math.hypot
import kotlin.math.max

internal class ChatProjectSideMenuView(
    context: Context,
    private val projects: () -> List<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val openPersonalProject: (Int) -> Unit,
    private val openJointProject: (Int) -> Unit,
    private val showCreateJointProjectDialog: () -> Unit,
    private val requestClose: (Boolean) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) : ScrollView(context) {
    private val content = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(32), dp(92), dp(18), dp(18))
    }
    private val expandedProjectIds = linkedSetOf<String>()

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
        addTopMenu()
        addPersonalProjects()
        addJointProjects()
    }

    private fun addTopMenu() {
        content.addView(menuRow("文件库", R.drawable.ic_side_menu_files) {
            Toast.makeText(context, "文件库功能准备中", Toast.LENGTH_SHORT).show()
        })
        content.addView(menuRow("设备", R.drawable.ic_side_menu_device) {
            Toast.makeText(context, "设备功能准备中", Toast.LENGTH_SHORT).show()
        })
        content.addView(space(70))
    }

    private fun addPersonalProjects() {
        content.addView(sectionHeader("个人项目", showAddButton = false))
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
        content.addView(sectionHeader("联合项目", showAddButton = true))
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

    private fun sectionHeader(title: String, showAddButton: Boolean): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(42)
            ).apply {
                topMargin = dp(2)
            }
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(menuText(title).apply {
                setTextColor(Color.parseColor("#F2F5FA"))
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            })
            if (showAddButton) {
                addView(ImageButton(context).apply {
                    setImageResource(R.drawable.ic_add_circle_simple)
                    imageTintList = android.content.res.ColorStateList.valueOf(Color.parseColor("#F2F5FA"))
                    background = null
                    scaleType = ImageView.ScaleType.CENTER
                    contentDescription = "发起联合项目"
                    foreground = selectableForeground()
                    setPadding(dp(4), dp(4), dp(4), dp(4))
                    setOnClickListener {
                        requestClose(true)
                        postDelayed({ showCreateJointProjectDialog() }, CLOSE_DELAY_MS)
                    }
                }, LinearLayout.LayoutParams(dp(38), dp(38)))
            }
        }
    }

    private fun menuRow(title: String, iconRes: Int, action: () -> Unit): LinearLayout {
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(46))
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            foreground = selectableForeground()
            addView(ImageView(context).apply {
                setImageResource(iconRes)
                imageTintList = android.content.res.ColorStateList.valueOf(Color.parseColor("#A6AFBD"))
                scaleType = ImageView.ScaleType.CENTER
            }, LinearLayout.LayoutParams(dp(26), dp(26)))
            addView(menuText(title).apply {
                setPadding(dp(16), 0, 0, 0)
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            })
            setOnClickListener { action() }
        }
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
                    setColor(Color.parseColor("#181B20"))
                }
            }
            setOnClickListener { onClick() }
            setOnLongClickListener {
                startProjectDrag(it, project.toChatProjectShare())
                true
            }
        }
    }

    private fun jointProjectRow(index: Int, project: AppProject): LinearLayout {
        val share = project.toChatProjectShare()
        val expanded = expandedProjectIds.contains(project.id)
        return LinearLayout(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                topMargin = dp(7)
            }
            orientation = LinearLayout.VERTICAL
            addView(LinearLayout(context).apply {
                gravity = Gravity.CENTER_VERTICAL
                orientation = LinearLayout.HORIZONTAL
                isClickable = true
                foreground = selectableForeground()
                setPadding(dp(10), 0, 0, 0)
                setOnClickListener {
                    requestClose(true)
                    postDelayed({ openJointProject(index) }, CLOSE_DELAY_MS)
                }
                setOnLongClickListener {
                    startProjectDrag(it, share)
                    true
                }
                addView(LinearLayout(context).apply {
                    orientation = LinearLayout.VERTICAL
                    addView(menuText(project.title).apply {
                        layoutParams = LinearLayout.LayoutParams(
                            LinearLayout.LayoutParams.MATCH_PARENT,
                            dp(28)
                        )
                    })
                    addView(TextView(context).apply {
                        text = project.latestProjectLog()
                        setTextColor(Color.parseColor("#6F7785"))
                        textSize = 10.5f
                        maxLines = 1
                        ellipsize = TextUtils.TruncateAt.END
                        includeFontPadding = false
                    })
                }, LinearLayout.LayoutParams(0, dp(46), 1f))
                addView(TextView(context).apply {
                    text = if (expanded) "⌃" else "⌄"
                    gravity = Gravity.CENTER
                    includeFontPadding = false
                    setTextColor(Color.parseColor("#A6AFBD"))
                    textSize = 16f
                    isClickable = true
                    foreground = selectableForeground()
                    setOnClickListener {
                        if (expanded) expandedProjectIds.remove(project.id) else expandedProjectIds.add(project.id)
                        render()
                    }
                }, LinearLayout.LayoutParams(dp(34), dp(34)))
            })
            if (expanded) {
                project.events.take(3).ifEmpty { listOf(project.subtitle) }.forEach { log ->
                    addView(logRow(log))
                }
            }
        }
    }

    private fun startProjectDrag(source: View, share: ChatProjectShare) {
        val clip = ClipData.newPlainText("project", share.toMessageText())
        source.startDragAndDrop(clip, View.DragShadowBuilder(source), share, 0)
    }

    private fun logRow(text: String): TextView {
        return TextView(context).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                leftMargin = dp(10)
                rightMargin = dp(18)
                topMargin = dp(3)
            }
            maxLines = 2
            ellipsize = TextUtils.TruncateAt.END
            includeFontPadding = false
            setTextColor(Color.parseColor("#6F7785"))
            textSize = 11.5f
            this.text = text
        }
    }

    private fun emptyRow(text: String): TextView {
        return menuText(text).apply {
            setTextColor(Color.parseColor("#6F7785"))
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
            setTextColor(Color.parseColor("#A6AFBD"))
            textSize = 17f
        }
    }

    private fun space(heightDp: Int): View {
        return View(context).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(heightDp))
        }
    }

    private fun AppProject.latestProjectLog(): String {
        return events.firstOrNull()?.trim()?.takeIf { it.isNotBlank() }
            ?: subtitle.takeIf { it.isNotBlank() }
            ?: "联合项目最新的项目日志内容"
    }

    private companion object {
        const val CLOSE_DELAY_MS = 220L
    }
}

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
