package com.elon.app

import android.app.Activity
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
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import java.io.File

internal class ChatImagePreviewActivity : AppCompatActivity() {
    private lateinit var addButton: TextView
    private lateinit var thumbStrip: LinearLayout
    private var previewItems = mutableListOf<PreviewImage>()
    private var pendingEditIndex = -1
    private var keepResultFiles = false

    private val editLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        val index = pendingEditIndex
        pendingEditIndex = -1
        if (result.resultCode != Activity.RESULT_OK || index !in previewItems.indices) return@registerForActivityResult
        val data = result.data ?: return@registerForActivityResult
        val nextPath = data.getStringExtra(ChatImageEditActivity.EXTRA_OUTPUT_PATH).orEmpty()
        val nextFile = File(nextPath)
        if (!nextFile.exists()) {
            Toast.makeText(this, "编辑后的图片已失效", Toast.LENGTH_SHORT).show()
            return@registerForActivityResult
        }
        val previous = previewItems[index]
        previewItems[index] = previous.copy(
            path = nextPath,
            name = data.getStringExtra(ChatImageEditActivity.EXTRA_OUTPUT_NAME) ?: nextFile.name,
            width = data.getIntExtra(ChatImageEditActivity.EXTRA_OUTPUT_WIDTH, 0),
            height = data.getIntExtra(ChatImageEditActivity.EXTRA_OUTPUT_HEIGHT, 0)
        )
        if (previous.path != nextPath) runCatching { File(previous.path).delete() }
        renderThumbnails()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        requestWindowFeature(Window.FEATURE_NO_TITLE)
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK
        previewItems = readInputItems().toMutableList()
        if (previewItems.isEmpty()) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            finish()
            return
        }
        setContentView(FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(topBar())
            addView(bottomPreviewTray())
        })
        renderThumbnails()
    }

    private fun readInputItems(): List<PreviewImage> {
        val paths = intent.getStringArrayListExtra(EXTRA_INPUT_PATHS)
            ?: arrayListOf(intent.getStringExtra(EXTRA_INPUT_PATH).orEmpty())
        val names = intent.getStringArrayListExtra(EXTRA_DISPLAY_NAMES)
            ?: arrayListOf(intent.getStringExtra(EXTRA_DISPLAY_NAME).orEmpty())
        val widths = intent.getIntegerArrayListExtra(EXTRA_INPUT_WIDTHS).orEmpty()
        val heights = intent.getIntegerArrayListExtra(EXTRA_INPUT_HEIGHTS).orEmpty()
        return paths.mapIndexedNotNull { index, path ->
            if (!File(path).exists()) return@mapIndexedNotNull null
            PreviewImage(
                sourceIndex = index,
                path = path,
                name = names.getOrNull(index).orEmpty().ifBlank { "图片" },
                width = widths.getOrNull(index) ?: 0,
                height = heights.getOrNull(index) ?: 0
            )
        }
    }

    private fun topBar(): View {
        return LinearLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(TOP_BAR_HEIGHT_DP),
                Gravity.TOP
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(24), 0, dp(20), 0)
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(0, dp(PRIMARY_CONTROL_HEIGHT_DP), 1f)
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "预览"
                setTextColor(getColor(R.color.elon_text_primary))
                textSize = 16f
            })
            addButton = TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(112), dp(PRIMARY_CONTROL_HEIGHT_DP))
                background = roundedRect(getColor(R.color.elon_button_primary_bg), dp(24))
                gravity = Gravity.CENTER
                includeFontPadding = false
                setTextColor(getColor(R.color.elon_button_primary_text))
                textSize = 16f
                setOnClickListener { finishWithImages() }
            }
            addView(addButton)
        }
    }

    private fun bottomPreviewTray(): View {
        return HorizontalScrollView(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(BOTTOM_TRAY_HEIGHT_DP),
                Gravity.BOTTOM
            )
            clipToPadding = false
            isHorizontalScrollBarEnabled = false
            setPadding(dp(24), 0, dp(24), dp(20))
            thumbStrip = LinearLayout(context).apply {
                layoutParams = FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                    FrameLayout.LayoutParams.MATCH_PARENT
                )
                gravity = Gravity.BOTTOM
                orientation = LinearLayout.HORIZONTAL
            }
            addView(thumbStrip)
        }
    }

    private fun renderThumbnails() {
        if (!::thumbStrip.isInitialized) return
        addButton.text = "添加 (${previewItems.size})"
        thumbStrip.removeAllViews()
        previewItems.forEachIndexed { index, item ->
            thumbStrip.addView(thumbnailCell(index, item))
        }
    }

    private fun thumbnailCell(index: Int, item: PreviewImage): View {
        return FrameLayout(this).apply {
            layoutParams = LinearLayout.LayoutParams(dp(THUMB_CELL_WIDTH_DP), dp(THUMB_CELL_HEIGHT_DP)).apply {
                marginEnd = dp(12)
            }
            addView(ImageView(context).apply {
                layoutParams = FrameLayout.LayoutParams(
                    dp(THUMB_SIZE_DP),
                    dp(THUMB_SIZE_DP),
                    Gravity.BOTTOM or Gravity.START
                )
                background = roundedRect(getColor(R.color.elon_surface_card), dp(6))
                clipToOutline = true
                scaleType = ImageView.ScaleType.CENTER_CROP
                contentDescription = "已选图片"
                decodePreviewBitmap(item.path)?.let { setImageBitmap(it) }
            })
            addView(editButton(index))
            addView(closeButton(index))
        }
    }

    private fun editButton(index: Int): ImageButton {
        return ImageButton(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(64), dp(48), Gravity.TOP or Gravity.START)
            background = ColorDrawable(Color.TRANSPARENT)
            contentDescription = "编辑图片"
            scaleType = ImageView.ScaleType.FIT_START
            setPadding(0, 0, dp(12), dp(8))
            setImageResource(R.drawable.ic_chat_image_edit_marker)
            setOnClickListener { openEditor(index) }
        }
    }

    private fun closeButton(index: Int): TextView {
        return TextView(this).apply {
            layoutParams = FrameLayout.LayoutParams(dp(48), dp(48), Gravity.TOP or Gravity.START).apply {
                leftMargin = dp(52)
                topMargin = dp(22)
            }
            background = ColorDrawable(Color.TRANSPARENT)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = "×"
            setTextColor(getColor(R.color.elon_text_primary))
            textSize = 22f
            contentDescription = "取消这张图片"
            setOnClickListener { removeImage(index) }
        }
    }

    private fun openEditor(index: Int) {
        val item = previewItems.getOrNull(index) ?: return
        if (!File(item.path).exists()) {
            Toast.makeText(this, "图片读取失败", Toast.LENGTH_SHORT).show()
            return
        }
        pendingEditIndex = index
        editLauncher.launch(ChatImageEditActivity.createIntent(this, item.path, item.name))
    }

    private fun removeImage(index: Int) {
        val item = previewItems.getOrNull(index) ?: return
        previewItems.removeAt(index)
        runCatching { File(item.path).delete() }
        if (previewItems.isEmpty()) {
            setResult(Activity.RESULT_CANCELED)
            finish()
        } else {
            renderThumbnails()
        }
    }

    private fun finishWithImages() {
        if (previewItems.isEmpty()) {
            setResult(Activity.RESULT_CANCELED)
            finish()
            return
        }
        setResult(
            Activity.RESULT_OK,
            Intent().apply {
                putStringArrayListExtra(EXTRA_OUTPUT_PATHS, ArrayList(previewItems.map { it.path }))
                putStringArrayListExtra(EXTRA_OUTPUT_NAMES, ArrayList(previewItems.map { it.name }))
                putIntegerArrayListExtra(EXTRA_OUTPUT_WIDTHS, ArrayList(previewItems.map { it.width }))
                putIntegerArrayListExtra(EXTRA_OUTPUT_HEIGHTS, ArrayList(previewItems.map { it.height }))
                putIntegerArrayListExtra(EXTRA_OUTPUT_SOURCE_INDEXES, ArrayList(previewItems.map { it.sourceIndex }))
                previewItems.firstOrNull()?.let { first ->
                    putExtra(EXTRA_OUTPUT_PATH, first.path)
                    putExtra(EXTRA_OUTPUT_NAME, first.name)
                    putExtra(EXTRA_OUTPUT_WIDTH, first.width)
                    putExtra(EXTRA_OUTPUT_HEIGHT, first.height)
                }
            }
        )
        keepResultFiles = true
        finish()
    }

    override fun onDestroy() {
        if (!keepResultFiles) {
            previewItems.forEach { item ->
                runCatching { File(item.path).delete() }
            }
        }
        super.onDestroy()
    }

    private fun decodePreviewBitmap(path: String): Bitmap? {
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeFile(path, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        var sample = 1
        val maxPixels = 400_000
        while ((bounds.outWidth / sample).toLong() * (bounds.outHeight / sample).toLong() > maxPixels) {
            sample *= 2
        }
        return BitmapFactory.decodeFile(path, BitmapFactory.Options().apply { inSampleSize = sample })
    }

    private fun roundedRect(color: Int, radius: Int): GradientDrawable {
        return GradientDrawable().apply {
            cornerRadius = radius.toFloat()
            setColor(color)
        }
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

    private data class PreviewImage(
        val sourceIndex: Int,
        val path: String,
        val name: String,
        val width: Int,
        val height: Int
    )

    companion object {
        const val EXTRA_INPUT_PATH = "chat_image_preview_input_path"
        const val EXTRA_DISPLAY_NAME = "chat_image_preview_display_name"
        const val EXTRA_OUTPUT_PATH = "chat_image_preview_output_path"
        const val EXTRA_OUTPUT_NAME = "chat_image_preview_output_name"
        const val EXTRA_OUTPUT_WIDTH = "chat_image_preview_output_width"
        const val EXTRA_OUTPUT_HEIGHT = "chat_image_preview_output_height"
        const val EXTRA_INPUT_PATHS = "chat_image_preview_input_paths"
        const val EXTRA_DISPLAY_NAMES = "chat_image_preview_display_names"
        const val EXTRA_INPUT_WIDTHS = "chat_image_preview_input_widths"
        const val EXTRA_INPUT_HEIGHTS = "chat_image_preview_input_heights"
        const val EXTRA_OUTPUT_PATHS = "chat_image_preview_output_paths"
        const val EXTRA_OUTPUT_NAMES = "chat_image_preview_output_names"
        const val EXTRA_OUTPUT_WIDTHS = "chat_image_preview_output_widths"
        const val EXTRA_OUTPUT_HEIGHTS = "chat_image_preview_output_heights"
        const val EXTRA_OUTPUT_SOURCE_INDEXES = "chat_image_preview_output_source_indexes"
        private const val TOP_BAR_HEIGHT_DP = 64
        private const val PRIMARY_CONTROL_HEIGHT_DP = 48
        private const val BOTTOM_TRAY_HEIGHT_DP = 116
        private const val THUMB_CELL_WIDTH_DP = 100
        private const val THUMB_CELL_HEIGHT_DP = 96
        private const val THUMB_SIZE_DP = 64

        fun createIntent(context: Context, path: String, displayName: String): Intent {
            return Intent(context, ChatImagePreviewActivity::class.java).apply {
                putExtra(EXTRA_INPUT_PATH, path)
                putExtra(EXTRA_DISPLAY_NAME, displayName)
            }
        }

        fun createIntent(context: Context, attachments: List<PendingAttachment>): Intent {
            return Intent(context, ChatImagePreviewActivity::class.java).apply {
                putStringArrayListExtra(EXTRA_INPUT_PATHS, ArrayList(attachments.map { it.file.absolutePath }))
                putStringArrayListExtra(EXTRA_DISPLAY_NAMES, ArrayList(attachments.map { it.displayName }))
                putIntegerArrayListExtra(EXTRA_INPUT_WIDTHS, ArrayList(attachments.map { it.imageWidth ?: 0 }))
                putIntegerArrayListExtra(EXTRA_INPUT_HEIGHTS, ArrayList(attachments.map { it.imageHeight ?: 0 }))
            }
        }
    }
}
