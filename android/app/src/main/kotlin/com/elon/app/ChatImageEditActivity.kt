package com.elon.app

import android.app.Activity
import android.app.AlertDialog
import android.content.Context
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.Window
import android.view.animation.AccelerateDecelerateInterpolator
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import java.io.ByteArrayOutputStream
import java.io.File
import kotlin.concurrent.thread
import kotlin.math.max

internal class ChatImageEditActivity : AppCompatActivity() {
    private lateinit var canvasView: ChatImageEditCanvasView
    private lateinit var undoButton: ImageButton
    private lateinit var redoButton: ImageButton
    private lateinit var bottomToolPanel: FrameLayout
    private lateinit var collapsedToolButton: ImageButton
    private val toolButtons = mutableMapOf<ChatImageEditTool, ImageButton>()
    private var bottomToolsCollapsed = false
    private var bottomToolsAnimating = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = Color.BLACK
        val sourcePath = intent.getStringExtra(EXTRA_INPUT_PATH).orEmpty()
        val bitmap = decodeBitmap(sourcePath)
        if (bitmap == null) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        canvasView = ChatImageEditCanvasView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
            setBitmap(bitmap)
            setBrushColor(Color.WHITE)
            onHistoryChanged = { refreshUndoRedo() }
        }

        setContentView(FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(canvasView)
            addView(topBar())
            addView(createBottomToolPanel())
            addView(createCollapsedToolButton())
        })
        selectTool(ChatImageEditTool.BRUSH)
        refreshUndoRedo()
    }

    private fun topBar(): View {
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(88),
                Gravity.TOP
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(32), dp(20), dp(30), 0)

            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(76), dp(44))
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "取消"
                setTextColor(Color.parseColor("#D9D9D9"))
                textSize = 16f
                setOnClickListener { finish() }
            })
            addView(View(context), LinearLayout.LayoutParams(0, 1, 1f))
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(82), dp(40))
                background = roundedRect("#FFFFFF", dp(20))
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "完成"
                setTextColor(Color.BLACK)
                textSize = 16f
                setOnClickListener { finishWithEditedImage() }
            })
        }
    }

    private fun createBottomToolPanel(): View {
        return FrameLayout(this).apply {
            bottomToolPanel = this
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(162),
                Gravity.BOTTOM
            )
            background = roundedTopRect("#1A1A1A", dp(32))
            addView(toolBar())
            addView(colorPalette())
            addView(collapseButton())
        }
    }

    private fun toolBar(): View {
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                dp(58),
                Gravity.TOP or Gravity.CENTER_HORIZONTAL
            ).apply {
                topMargin = dp(18)
            }
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL

            addToolButton(this, ChatImageEditTool.BRUSH, R.drawable.ic_chat_image_tool_brush, "画笔") {
                selectTool(ChatImageEditTool.BRUSH)
            }
            addToolButton(this, ChatImageEditTool.CIRCLE, R.drawable.ic_chat_image_tool_circle, "圆形圈选") {
                selectTool(ChatImageEditTool.CIRCLE)
            }
            addToolButton(this, ChatImageEditTool.SQUARE, R.drawable.ic_chat_image_tool_square, "方形圈选") {
                selectTool(ChatImageEditTool.SQUARE)
            }
            addToolButton(this, ChatImageEditTool.TEXT, R.drawable.ic_chat_image_tool_text, "文字") {
                showTextDialog()
            }
            addToolButton(this, ChatImageEditTool.MOSAIC, R.drawable.ic_chat_image_tool_mosaic, "马赛克") {
                selectTool(ChatImageEditTool.MOSAIC)
            }
        }
    }

    private fun colorPalette(): View {
        val colors = intArrayOf(
            Color.WHITE,
            Color.BLACK,
            Color.parseColor("#E62129"),
            Color.parseColor("#F2C94C"),
            Color.parseColor("#2EA7FF"),
            Color.parseColor("#58BE6A")
        )
        return FrameLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(64),
                Gravity.BOTTOM
            ).apply {
                bottomMargin = dp(20)
            }

            undoButton = iconButton(R.drawable.ic_chat_image_tool_undo, "撤销", padding = 0).apply {
                layoutParams = FrameLayout.LayoutParams(dp(64), dp(64), Gravity.START or Gravity.CENTER_VERTICAL).apply {
                    leftMargin = dp(14)
                }
                setOnClickListener {
                    if (canvasView.undo()) refreshUndoRedo()
                }
            }
            redoButton = iconButton(R.drawable.ic_chat_image_tool_redo, "重做", padding = 0).apply {
                layoutParams = FrameLayout.LayoutParams(dp(64), dp(64), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                    rightMargin = dp(14)
                }
                setOnClickListener {
                    if (canvasView.redo()) refreshUndoRedo()
                }
            }
            addView(undoButton)
            addView(LinearLayout(context).apply {
                layoutParams = FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                    dp(56),
                    Gravity.CENTER
                )
                gravity = Gravity.CENTER
                orientation = LinearLayout.HORIZONTAL
                colors.forEachIndexed { index, color ->
                    addView(View(context).apply {
                        layoutParams = LinearLayout.LayoutParams(dp(28), dp(28)).apply {
                            if (index < colors.lastIndex) marginEnd = dp(12)
                        }
                        background = GradientDrawable().apply {
                            shape = GradientDrawable.OVAL
                            setColor(color)
                            setStroke(dp(2), Color.parseColor("#D9D9D9"))
                        }
                        setOnClickListener {
                            canvasView.setBrushColor(color)
                        }
                    })
                }
            })
            addView(redoButton)
        }
    }

    private fun addToolButton(
        row: LinearLayout,
        tool: ChatImageEditTool,
        iconRes: Int,
        description: String,
        onClick: () -> Unit
    ) {
        val button = iconButton(iconRes, description).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(58))
            setOnClickListener { onClick() }
        }
        toolButtons[tool] = button
        row.addView(button)
    }

    private fun selectTool(tool: ChatImageEditTool) {
        canvasView.setTool(tool)
        toolButtons.values.forEach { button ->
            button.alpha = 1f
            button.background = ColorDrawable(Color.TRANSPARENT)
        }
    }

    private fun refreshUndoRedo() {
        if (::undoButton.isInitialized) {
            undoButton.alpha = if (canvasView.canUndo()) 1f else 0.72f
        }
        if (::redoButton.isInitialized) {
            redoButton.alpha = if (canvasView.canRedo()) 1f else 0.72f
        }
    }

    private fun collapseButton(): View {
        return iconButton(R.drawable.ic_chat_image_tool_collapse_down, "Collapse toolbar", padding = 8).apply {
            layoutParams = FrameLayout.LayoutParams(dp(56), dp(56), Gravity.TOP or Gravity.END).apply {
                topMargin = dp(19)
                rightMargin = dp(18)
            }
            setOnClickListener { collapseBottomTools() }
        }
    }

    private fun createCollapsedToolButton(): View {
        return iconButton(R.drawable.ic_chat_image_tool_expand, "Expand toolbar", padding = 0, tintWhite = false).apply {
            collapsedToolButton = this
            layoutParams = FrameLayout.LayoutParams(dp(68), dp(48), Gravity.BOTTOM or Gravity.END).apply {
                rightMargin = dp(18)
                bottomMargin = 0
            }
            visibility = View.INVISIBLE
            alpha = 0f
            setOnClickListener { expandBottomTools() }
        }
    }

    private fun collapseBottomTools() {
        if (bottomToolsCollapsed || bottomToolsAnimating) return
        bottomToolsAnimating = true
        bottomToolPanel.animate().cancel()
        collapsedToolButton.animate().cancel()
        val panelHeight = bottomToolPanel.height.takeIf { it > 0 } ?: dp(162)
        bottomToolPanel.animate()
            .translationY(panelHeight.toFloat())
            .setDuration(220L)
            .setInterpolator(AccelerateDecelerateInterpolator())
            .withEndAction {
                bottomToolPanel.visibility = View.INVISIBLE
                bottomToolPanel.translationY = panelHeight.toFloat()
                bottomToolsCollapsed = true
                bottomToolsAnimating = false
                showCollapsedToolButton()
            }
            .start()
    }

    private fun showCollapsedToolButton() {
        collapsedToolButton.visibility = View.VISIBLE
        collapsedToolButton.alpha = 0f
        collapsedToolButton.translationY = dp(18).toFloat()
        collapsedToolButton.animate()
            .alpha(1f)
            .translationY(0f)
            .setDuration(160L)
            .setInterpolator(AccelerateDecelerateInterpolator())
            .start()
    }

    private fun expandBottomTools() {
        if (!bottomToolsCollapsed || bottomToolsAnimating) return
        bottomToolsAnimating = true
        collapsedToolButton.animate().cancel()
        collapsedToolButton.visibility = View.INVISIBLE
        collapsedToolButton.alpha = 0f
        val panelHeight = bottomToolPanel.height.takeIf { it > 0 } ?: dp(162)
        bottomToolPanel.visibility = View.VISIBLE
        bottomToolPanel.alpha = 1f
        bottomToolPanel.translationY = panelHeight.toFloat()
        bottomToolPanel.animate()
            .translationY(0f)
            .setDuration(220L)
            .setInterpolator(AccelerateDecelerateInterpolator())
            .withEndAction {
                bottomToolsCollapsed = false
                bottomToolsAnimating = false
            }
            .start()
    }

    private fun showTextDialog() {
        selectTool(ChatImageEditTool.TEXT)
        val input = EditText(this).apply {
            setSingleLine(false)
            minLines = 1
            maxLines = 3
            setTextColor(Color.WHITE)
            setHintTextColor(Color.parseColor("#AFAFAF"))
            hint = "输入文字"
        }
        val dialog = AlertDialog.Builder(this)
            .setTitle("添加文字")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("添加") { _, _ ->
                canvasView.addText(input.text.toString())
                refreshUndoRedo()
            }
            .show()
        dialog.window?.setBackgroundDrawable(ColorDrawable(Color.parseColor("#1A1A1A")))
        dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.setTextColor(Color.WHITE)
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE)?.setTextColor(Color.parseColor("#B8B8B8"))
    }

    private fun finishWithEditedImage() {
        val bitmap = runCatching { canvasView.renderEditedBitmap() }.getOrNull()
        if (bitmap == null) {
            Toast.makeText(this, "图片生成失败", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(this, "正在生成图片...", Toast.LENGTH_SHORT).show()
        thread(name = "chat-image-editor-save") {
            val result = runCatching { saveBitmap(bitmap) }
            runOnUiThread {
                result.onSuccess { saved ->
                    setResult(
                        Activity.RESULT_OK,
                        Intent().apply {
                            putExtra(EXTRA_OUTPUT_PATH, saved.file.absolutePath)
                            putExtra(EXTRA_OUTPUT_NAME, saved.file.name)
                            putExtra(EXTRA_OUTPUT_WIDTH, saved.width)
                            putExtra(EXTRA_OUTPUT_HEIGHT, saved.height)
                        }
                    )
                    finish()
                }.onFailure {
                    Toast.makeText(this, "图片保存失败：${it.message}", Toast.LENGTH_LONG).show()
                }
            }
        }
    }

    private fun saveBitmap(bitmap: Bitmap): SavedImage {
        val dir = File(cacheDir, "pending_attachments").apply { mkdirs() }
        val file = File(dir, "edited_${System.currentTimeMillis()}.jpg")
        var working = bitmap
        var bytes = compressJpeg(working, 88)
        while (bytes.size > MAX_ATTACHMENT_BYTES) {
            val nextWidth = max(480, (working.width * 0.82f).toInt())
            val nextHeight = max(480, (working.height * 0.82f).toInt())
            if (nextWidth >= working.width || nextHeight >= working.height) break
            val scaled = Bitmap.createScaledBitmap(working, nextWidth, nextHeight, true)
            if (working !== bitmap) working.recycle()
            working = scaled
            bytes = compressJpeg(working, 82)
        }
        require(bytes.size <= MAX_ATTACHMENT_BYTES) { "图片超过 8MB" }
        file.writeBytes(bytes)
        return SavedImage(file = file, width = working.width, height = working.height)
    }

    private fun compressJpeg(bitmap: Bitmap, quality: Int): ByteArray {
        return ByteArrayOutputStream().use { output ->
            bitmap.compress(Bitmap.CompressFormat.JPEG, quality, output)
            output.toByteArray()
        }
    }

    private fun decodeBitmap(path: String): Bitmap? {
        val file = File(path)
        if (!file.exists()) return null
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        var sample = 1
        while ((bounds.outWidth / sample) * (bounds.outHeight / sample) > MAX_EDITOR_PIXELS) {
            sample *= 2
        }
        return BitmapFactory.decodeFile(
            path,
            BitmapFactory.Options().apply {
                inSampleSize = sample
                inPreferredConfig = Bitmap.Config.ARGB_8888
            }
        )
    }

    private fun iconButton(
        iconRes: Int,
        description: String,
        padding: Int = 8,
        tintWhite: Boolean = true
    ): ImageButton {
        return ImageButton(this).apply {
            contentDescription = description
            background = ColorDrawable(Color.TRANSPARENT)
            setImageResource(iconRes)
            if (tintWhite) {
                setColorFilter(Color.WHITE)
            } else {
                clearColorFilter()
            }
            scaleType = ImageView.ScaleType.CENTER_INSIDE
            setPadding(dp(padding), dp(padding), dp(padding), dp(padding))
            isClickable = true
        }
    }

    private fun roundedRect(color: String, radius: Int): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = radius.toFloat()
            setColor(Color.parseColor(color))
        }
    }

    private fun roundedTopRect(color: String, radius: Int): GradientDrawable {
        val corner = radius.toFloat()
        return GradientDrawable().apply {
            cornerRadii = floatArrayOf(
                corner, corner,
                corner, corner,
                0f, 0f,
                0f, 0f
            )
            setColor(Color.parseColor(color))
        }
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

    private data class SavedImage(
        val file: File,
        val width: Int,
        val height: Int
    )

    companion object {
        const val EXTRA_INPUT_PATH = "chat_image_edit_input_path"
        const val EXTRA_DISPLAY_NAME = "chat_image_edit_display_name"
        const val EXTRA_OUTPUT_PATH = "chat_image_edit_output_path"
        const val EXTRA_OUTPUT_NAME = "chat_image_edit_output_name"
        const val EXTRA_OUTPUT_WIDTH = "chat_image_edit_output_width"
        const val EXTRA_OUTPUT_HEIGHT = "chat_image_edit_output_height"
        private const val MAX_EDITOR_PIXELS = 4_000_000

        fun createIntent(context: Context, path: String, displayName: String): Intent {
            return Intent(context, ChatImageEditActivity::class.java).apply {
                putExtra(EXTRA_INPUT_PATH, path)
                putExtra(EXTRA_DISPLAY_NAME, displayName)
            }
        }
    }
}
