package com.elon.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.drawable.Drawable
import android.text.Editable
import android.text.TextUtils
import android.text.TextWatcher
import android.util.Base64
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.animation.AccelerateInterpolator
import android.view.animation.DecelerateInterpolator
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import kotlin.math.roundToInt

internal class ProjectBrowserSheetController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val projects: () -> List<AppProject>,
    private val openProject: (Int) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private val root = binding.projectBrowserSheet
    private val groupsContainer = LinearLayout(activity).apply {
        id = R.id.projectBrowserGroups
        orientation = LinearLayout.VERTICAL
        setPadding(dp(27), dp(24), dp(27), dp(104))
    }
    private val searchInput = EditText(activity).apply {
        id = R.id.projectBrowserSearchInput
        background = null
        contentDescription = "搜索项目"
        gravity = Gravity.CENTER
        hint = "搜索项目"
        includeFontPadding = false
        isSingleLine = true
        setHintTextColor(activity.getColor(R.color.elon_text_placeholder))
        setPadding(dp(48), 0, dp(48), 0)
        setTextColor(activity.getColor(R.color.elon_text_primary))
        textSize = 16f
    }
    private var personalExpanded = true
    private var jointExpanded = true
    private var dragStartY = 0f

    val isOpen: Boolean
        get() = root.visibility == View.VISIBLE

    fun setup() {
        binding.projectBrowserSheetContent.removeAllViews()
        binding.projectBrowserSheetContent.addView(buildSheetContent())
        searchInput.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(value: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(value: CharSequence?, start: Int, before: Int, count: Int) {
                renderProjects()
            }
            override fun afterTextChanged(value: Editable?) = Unit
        })
    }

    fun toggle() {
        if (isOpen) close() else open()
    }

    fun open() {
        if (isOpen) return
        showLoading()
        root.visibility = View.VISIBLE
        root.alpha = 1f
        root.bringToFront()
        binding.pageTabs.bringToFront()
        setMenuSelected(true)
        root.post {
            root.translationY = root.height.toFloat()
            root.animate()
                .translationY(0f)
                .setDuration(280L)
                .setInterpolator(DecelerateInterpolator())
                .withEndAction { renderProjects() }
                .start()
        }
    }

    fun close(animate: Boolean = true) {
        if (!isOpen) return
        searchInput.clearFocus()
        setMenuSelected(false)
        if (!animate) {
            root.animate().cancel()
            root.translationY = root.height.toFloat()
            root.visibility = View.GONE
            return
        }
        root.animate()
            .translationY(root.height.toFloat())
            .setDuration(220L)
            .setInterpolator(AccelerateInterpolator())
            .withEndAction {
                root.visibility = View.GONE
                root.translationY = 0f
            }
            .start()
    }

    fun handleBack(): Boolean {
        if (!isOpen) return false
        close()
        return true
    }

    private fun buildSheetContent(): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            addView(buildDragTarget(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(48)
            ))
            addView(buildSearchField(), LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(48)
            ).apply {
                leftMargin = designPx(200)
                rightMargin = designPx(200)
                topMargin = -dp(10)
            })
            addView(ScrollView(activity).apply {
                overScrollMode = View.OVER_SCROLL_NEVER
                isFillViewport = false
                isVerticalScrollBarEnabled = false
                addView(groupsContainer, ViewGroup.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.WRAP_CONTENT
                ))
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                0,
                1f
            ))
        }
    }

    private fun buildDragTarget(): FrameLayout {
        return FrameLayout(activity).apply {
            id = R.id.projectBrowserDragTarget
            contentDescription = "下滑关闭项目查看"
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            addView(ImageView(activity).apply {
                id = R.id.projectBrowserDragHandle
                contentDescription = "项目查看拖拽条"
                scaleType = ImageView.ScaleType.FIT_XY
                setImageResource(R.drawable.project_view_drag_handle)
            }, FrameLayout.LayoutParams(designPx(232), designPx(16)).apply {
                gravity = Gravity.TOP or Gravity.CENTER_HORIZONTAL
                topMargin = designPx(49)
            })
            setOnTouchListener { view, event -> handleDrag(view, event) }
        }
    }

    private fun buildSearchField(): FrameLayout {
        return FrameLayout(activity).apply {
            id = R.id.projectBrowserSearch
            addView(FrameLayout(activity).apply {
                id = R.id.projectBrowserSearchVisual
                addView(ImageView(activity).apply {
                    id = R.id.projectBrowserSearchBackground
                    contentDescription = "项目搜索框背景"
                    scaleType = ImageView.ScaleType.FIT_XY
                    setImageResource(R.drawable.project_view_search_field)
                }, FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                ))
                addView(ImageView(activity).apply {
                    id = R.id.projectBrowserSearchIcon
                    contentDescription = "搜索项目图标"
                    scaleType = ImageView.ScaleType.FIT_CENTER
                    setImageResource(R.drawable.project_view_search_icon)
                }, FrameLayout.LayoutParams(designPx(97), designPx(97)).apply {
                    gravity = Gravity.START or Gravity.CENTER_VERTICAL
                    leftMargin = designPx(34)
                })
            }, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                designPx(123)
            ).apply {
                gravity = Gravity.CENTER_VERTICAL
            })
            addView(searchInput, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }
    }

    private fun handleDrag(view: View, event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                root.animate().cancel()
                dragStartY = event.rawY
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                root.translationY = (event.rawY - dragStartY).coerceAtLeast(0f)
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                if (root.translationY >= dp(72)) {
                    close()
                } else if (event.actionMasked == MotionEvent.ACTION_UP && root.translationY < dp(8)) {
                    view.performClick()
                    close()
                } else {
                    root.animate()
                        .translationY(0f)
                        .setDuration(180L)
                        .setInterpolator(DecelerateInterpolator())
                        .start()
                }
                return true
            }
        }
        return false
    }

    private fun renderProjects() {
        val groups = runCatching { groupProjectsForBrowser(projects(), searchInput.text?.toString().orEmpty()) }
            .getOrElse {
                showFailure()
                return
            }
        groupsContainer.removeAllViews()
        addSection("个人项目", groups.personal, personalExpanded) {
            personalExpanded = !personalExpanded
            renderProjects()
        }
        groupsContainer.addView(space(24))
        addSection("群体项目", groups.joint, jointExpanded) {
            jointExpanded = !jointExpanded
            renderProjects()
        }
    }

    private fun addSection(
        title: String,
        entries: List<IndexedBrowserProject>,
        expanded: Boolean,
        toggle: () -> Unit
    ) {
        groupsContainer.addView(sectionHeader(title, expanded, toggle))
        if (!expanded) return
        if (entries.isEmpty()) {
            groupsContainer.addView(statusRow(
                if (searchInput.text.isNullOrBlank()) "暂无$title" else "没有匹配的$title"
            ))
            return
        }
        groupsContainer.addView(projectGrid(entries))
    }

    private fun sectionHeader(title: String, expanded: Boolean, toggle: () -> Unit): LinearLayout {
        return LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            contentDescription = if (expanded) "收起$title" else "展开$title"
            addView(TextView(activity).apply {
                gravity = Gravity.CENTER_VERTICAL
                includeFontPadding = false
                text = title
                setTextColor(activity.getColor(R.color.elon_text_placeholder))
                textSize = 17f
            }, LinearLayout.LayoutParams(0, dp(48), 1f))
            addView(ImageView(activity).apply {
                contentDescription = if (expanded) "收起$title" else "展开$title"
                scaleType = ImageView.ScaleType.CENTER
                setImageResource(R.drawable.project_view_chevron)
            }, LinearLayout.LayoutParams(dp(48), dp(48)))
            setOnClickListener { toggle() }
        }
    }

    private fun projectGrid(entries: List<IndexedBrowserProject>): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            projectBrowserGridRows(entries).forEach { rowEntries ->
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    rowEntries.forEach { entry ->
                        addView(FrameLayout(activity).apply {
                            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                            entry?.let { project ->
                                addView(projectCell(project), FrameLayout.LayoutParams(
                                    FrameLayout.LayoutParams.MATCH_PARENT,
                                    FrameLayout.LayoutParams.MATCH_PARENT
                                ))
                            }
                        }, LinearLayout.LayoutParams(0, dp(101), 1f))
                    }
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(101)
                ))
            }
        }
    }

    private fun projectCell(entry: IndexedBrowserProject): LinearLayout {
        val project = entry.project
        return LinearLayout(activity).apply {
            gravity = Gravity.TOP or Gravity.CENTER_HORIZONTAL
            orientation = LinearLayout.VERTICAL
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            contentDescription = "打开项目 ${project.title}"
            addView(ImageView(activity).apply {
                contentDescription = "${project.title} 项目图标"
                scaleType = ImageView.ScaleType.CENTER_CROP
                setImageResource(R.drawable.project_view_avatar_placeholder)
                decodeDataUrlBitmap(project.iconDataUrl)?.let(::setImageBitmap)
            }, LinearLayout.LayoutParams(designPx(156), designPx(156)))
            addView(TextView(activity).apply {
                ellipsize = TextUtils.TruncateAt.END
                gravity = Gravity.CENTER
                includeFontPadding = false
                maxLines = 1
                text = project.title.ifBlank { "未命名项目" }
                setTextColor(activity.getColor(R.color.elon_text_primary))
                textSize = 16f
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(30)
            ).apply { topMargin = dp(7) })
            setOnClickListener {
                close()
                root.postDelayed({ openProject(entry.index) }, 230L)
            }
        }
    }

    private fun showLoading() {
        groupsContainer.removeAllViews()
        groupsContainer.addView(statusRow("正在加载项目…"))
    }

    private fun showFailure() {
        groupsContainer.removeAllViews()
        groupsContainer.addView(statusRow("项目加载失败，点此重试").apply {
            isClickable = true
            isFocusable = true
            foreground = selectableForeground()
            contentDescription = "重新加载项目"
            setOnClickListener { renderProjects() }
        })
    }

    private fun statusRow(message: String): TextView {
        return TextView(activity).apply {
            gravity = Gravity.CENTER
            includeFontPadding = false
            minHeight = dp(64)
            text = message
            setTextColor(activity.getColor(R.color.elon_text_tertiary))
            textSize = 15f
        }
    }

    private fun space(height: Int): View = View(activity).apply {
        layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(height))
    }

    private fun setMenuSelected(selected: Boolean) {
        binding.bottomMenuSelection.isSelected = selected
        binding.bottomMenuIcon.isSelected = selected
    }

    private fun designPx(targetPixels: Int): Int {
        val width = activity.resources.displayMetrics.widthPixels.coerceAtLeast(1)
        return (width * targetPixels / 1273f).roundToInt().coerceAtLeast(1)
    }

    private fun decodeDataUrlBitmap(dataUrl: String?): Bitmap? {
        val data = dataUrl?.substringAfter(',', "")?.takeIf(String::isNotBlank) ?: return null
        return runCatching {
            val bytes = Base64.decode(data, Base64.DEFAULT)
            BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
        }.getOrNull()
    }
}
