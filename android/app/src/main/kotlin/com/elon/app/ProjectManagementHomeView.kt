package com.elon.app

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class ProjectManagementHomeView(
    private val activity: AppCompatActivity,
    private val container: LinearLayout,
    private val projects: () -> List<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val formatTime: (Long) -> String,
    private val openProject: (Int) -> Unit,
    private val showProjectActions: (Int) -> Unit,
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

        addSection("个人项目", personal, topMargin = 12, emptyAction = showCreateProjectDialog)
        addSection("联合项目", joint, topMargin = 12, emptyAction = null)
        container.addView(bottomSpacer())
    }

    private fun createPlazaBanner(): View {
        val contentWidth = activity.resources.displayMetrics.widthPixels - dp(20)
        val bannerHeight = (contentWidth * 0.36f).toInt().coerceIn(dp(124), dp(158))
        return FrameLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                bannerHeight
            ).apply {
                marginStart = dp(10)
                marginEnd = dp(10)
                topMargin = dp(12)
            }
            clipToPadding = false
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showProjectPlaza() }

            addView(ProjectPlazaPatternView(activity), FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))

            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "项目广场"
                setTextColor(Color.parseColor("#F2F5FA"))
                alpha = 0.92f
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 22f)
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.START or Gravity.TOP
                leftMargin = dp(14)
                topMargin = dp(19)
            })

            addView(ImageView(activity).apply {
                setImageResource(R.drawable.ic_search_simple)
                imageTintList = ColorStateList.valueOf(Color.parseColor("#9FA1A6"))
                alpha = 0.82f
            }, FrameLayout.LayoutParams(dp(108), dp(108)).apply {
                gravity = Gravity.CENTER
                leftMargin = dp(112)
            })
        }
    }

    private fun addSection(
        title: String,
        items: List<IndexedProject>,
        topMargin: Int,
        emptyAction: (() -> Unit)?
    ) {
        container.addView(createSectionHeader(title), LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(46)
        ).apply {
            marginStart = dp(10)
            marginEnd = dp(10)
            this.topMargin = dp(topMargin)
        })
        addProjectGrid(items, emptyAction)
    }

    private fun createSectionHeader(title: String): LinearLayout {
        return LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = title
                setTextColor(Color.parseColor("#F2F5FA"))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 20.5f)
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = "›"
                gravity = Gravity.CENTER
                setTextColor(Color.parseColor("#A6AFBD"))
                setTextSize(TypedValue.COMPLEX_UNIT_SP, 26f)
            }, LinearLayout.LayoutParams(dp(30), LinearLayout.LayoutParams.MATCH_PARENT))
        }
    }

    private fun addProjectGrid(items: List<IndexedProject>, emptyAction: (() -> Unit)?) {
        val cells = when {
            items.isEmpty() -> listOf<IndexedProject?>(null, null)
            items.size % 2 == 0 -> items
            else -> items + listOf(null)
        }
        cells.chunked(2).forEachIndexed { rowIndex, rowItems ->
            container.addView(LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.TOP
                rowItems.forEachIndexed { cellIndex, indexed ->
                    val card = indexed?.let { createProjectCard(it) } ?: createEmptyProjectSlot(emptyAction)
                    addView(card, LinearLayout.LayoutParams(
                        0,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        1f
                    ).apply {
                        if (cellIndex == 0) marginEnd = dp(5)
                        else marginStart = dp(5)
                    })
                }
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                marginStart = dp(10)
                marginEnd = dp(10)
                topMargin = if (rowIndex == 0) dp(8) else dp(16)
            })
        }
    }

    private fun createProjectCard(item: IndexedProject): View {
        val project = item.project
        val isActive = item.index == activeProjectIndex()
        return SquareProjectCardFrame(activity).apply {
            background = rect("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openProject(item.index) }
            setOnLongClickListener {
                showProjectActions(item.index)
                true
            }

            addView(projectThumbnail(project), FrameLayout.LayoutParams(dp(38), dp(38)).apply {
                gravity = Gravity.START or Gravity.TOP
                leftMargin = dp(17)
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
                isClickable = true
                foreground = selectableForeground()
                setOnClickListener { action() }
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
}

private class SquareProjectCardFrame(context: Context) : FrameLayout(context) {
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val exactHeight = MeasureSpec.makeMeasureSpec(width, MeasureSpec.EXACTLY)
        super.onMeasure(widthMeasureSpec, exactHeight)
    }
}

private class ProjectPlazaPatternView(context: Context) : View(context) {
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
    private val tileRect = RectF()

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        canvas.drawRect(0f, 0f, width.toFloat(), height.toFloat(), bgPaint)
        drawGrid(canvas)
        drawTiles(canvas)
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

    private fun drawTiles(canvas: Canvas) {
        val tileW = dp(50).toFloat()
        val tileH = dp(50).toFloat()
        val gapX = dp(96)
        val gapY = dp(84)
        canvas.save()
        canvas.rotate(-14f, width / 2f, height / 2f)
        var row = 0
        var y = -height
        while (y < height * 2) {
            var x = -width + if (row % 2 == 0) 0 else gapX / 2
            while (x < width * 2) {
                tileRect.set(x.toFloat(), y.toFloat(), x + tileW, y + tileH)
                canvas.drawRoundRect(tileRect, dp(5).toFloat(), dp(5).toFloat(), tilePaint)
                x += gapX
            }
            row += 1
            y += gapY
        }
        canvas.restore()
    }

    private fun dp(value: Int): Int = (value * density + 0.5f).toInt()
}
