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
import android.view.ViewGroup
import android.view.Window
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.GridLayout
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
    private lateinit var undoButton: TextView
    private lateinit var redoButton: TextView
    private val toolButtons = mutableMapOf<ChatImageEditTool, TextView>()
    private var selectedTool = ChatImageEditTool.BRUSH
    private var sourceName = "图片"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        window.statusBarColor = Color.TRANSPARENT
        window.navigationBarColor = Color.BLACK
        val sourcePath = intent.getStringExtra(EXTRA_INPUT_PATH).orEmpty()
        sourceName = intent.getStringExtra(EXTRA_DISPLAY_NAME).orEmpty().ifBlank { "图片" }
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
            addView(colorPalette())
            addView(bottomBar())
        })
        selectTool(ChatImageEditTool.BRUSH)
        refreshUndoRedo()
    }

    private fun topBar(): View {
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(86),
                Gravity.TOP
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(24), dp(28), dp(24), 0)

            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(88), dp(48))
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "取消"
                setTextColor(Color.WHITE)
                textSize = 20f
                setOnClickListener { finish() }
            })
            addView(View(context), LinearLayout.LayoutParams(0, 1, 1f))
            undoButton = iconTextButton("↶", "撤销").apply {
                setOnClickListener {
                    if (canvasView.undo()) refreshUndoRedo()
                }
            }
            redoButton = iconTextButton("↷", "重做").apply {
                setOnClickListener {
                    if (canvasView.redo()) refreshUndoRedo()
                }
            }
            addView(undoButton)
            addView(redoButton)
        }
    }

    private fun colorPalette(): View {
        val colors = intArrayOf(
            Color.WHITE,
            Color.BLACK,
            Color.parseColor("#E62129"),
            Color.parseColor("#F2C94C"),
            Color.parseColor("#58BE6A"),
            Color.parseColor("#2EA7FF")
        )
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                dp(44),
                Gravity.BOTTOM or Gravity.START
            ).apply {
                leftMargin = dp(18)
                bottomMargin = dp(92)
            }
            gravity = Gravity.CENTER
            orientation = LinearLayout.HORIZONTAL
            colors.forEach { color ->
                addView(View(context).apply {
                    layoutParams = LinearLayout.LayoutParams(dp(28), dp(28)).apply {
                        marginEnd = dp(10)
                    }
                    background = GradientDrawable().apply {
                        shape = GradientDrawable.OVAL
                        setColor(color)
                        setStroke(dp(2), Color.parseColor("#CCFFFFFF"))
                    }
                    setOnClickListener {
                        canvasView.setBrushColor(color)
                    }
                })
            }
        }
    }

    private fun bottomBar(): View {
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(92),
                Gravity.BOTTOM
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(22), 0, dp(18), dp(18))

            addToolButton(this, ChatImageEditTool.BRUSH, "✎", "画笔") {
                selectTool(ChatImageEditTool.BRUSH)
            }
            addToolButton(this, ChatImageEditTool.STICKER, "☺", "表情") {
                showStickerDialog()
            }
            addToolButton(this, ChatImageEditTool.TEXT, "T", "文字") {
                showTextDialog()
            }
            addToolButton(this, ChatImageEditTool.CROP, "⌗", "裁剪") {
                selectTool(ChatImageEditTool.CROP)
            }
            addToolButton(this, ChatImageEditTool.MOSAIC, "▦", "马赛克") {
                selectTool(ChatImageEditTool.MOSAIC)
            }
            addView(View(context), LinearLayout.LayoutParams(0, 1, 1f))
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(92), dp(54))
                background = roundedRect("#58BE6A", dp(10))
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = "完成"
                setTextColor(Color.WHITE)
                textSize = 18f
                setOnClickListener { finishWithEditedImage() }
            })
        }
    }

    private fun addToolButton(
        row: LinearLayout,
        tool: ChatImageEditTool,
        icon: String,
        description: String,
        onClick: () -> Unit
    ) {
        val button = iconTextButton(icon, description).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(54)).apply {
                marginEnd = dp(10)
            }
            setOnClickListener { onClick() }
        }
        toolButtons[tool] = button
        row.addView(button)
    }

    private fun selectTool(tool: ChatImageEditTool) {
        selectedTool = tool
        canvasView.setTool(tool)
        toolButtons.forEach { (key, button) ->
            button.alpha = if (key == tool) 1f else 0.68f
            button.background = if (key == tool) {
                roundedRect("#303030", dp(8))
            } else {
                ColorDrawable(Color.TRANSPARENT)
            }
        }
    }

    private fun refreshUndoRedo() {
        if (::undoButton.isInitialized) {
            undoButton.alpha = if (canvasView.canUndo()) 1f else 0.35f
        }
        if (::redoButton.isInitialized) {
            redoButton.alpha = if (canvasView.canRedo()) 1f else 0.35f
        }
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

    private fun showStickerDialog() {
        selectTool(ChatImageEditTool.STICKER)
        val grid = GridLayout(this).apply {
            columnCount = 4
            setPadding(dp(12), dp(12), dp(12), dp(4))
        }
        val emojis = listOf("😀", "😂", "😍", "👍", "🔥", "🎉", "❤️", "✨")
        var dialog: AlertDialog? = null
        emojis.forEach { emoji ->
            grid.addView(TextView(this).apply {
                layoutParams = ViewGroup.LayoutParams(dp(58), dp(54))
                gravity = Gravity.CENTER
                text = emoji
                textSize = 30f
                setOnClickListener {
                    canvasView.addSticker(emoji)
                    refreshUndoRedo()
                    dialog?.dismiss()
                }
            })
        }
        dialog = AlertDialog.Builder(this)
            .setTitle("添加表情")
            .setView(grid)
            .setNegativeButton("取消", null)
            .show()
        dialog.window?.setBackgroundDrawable(ColorDrawable(Color.parseColor("#1A1A1A")))
        dialog.getButton(AlertDialog.BUTTON_NEGATIVE)?.setTextColor(Color.parseColor("#B8B8B8"))
    }

    private fun finishWithEditedImage() {
        canvasView.applyCropIfActive()
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

    private fun iconTextButton(icon: String, description: String): TextView {
        return TextView(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
            contentDescription = description
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = icon
            setTextColor(Color.WHITE)
            textSize = 30f
            isClickable = true
        }
    }

    private fun roundedRect(color: String, radius: Int): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = radius.toFloat()
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
