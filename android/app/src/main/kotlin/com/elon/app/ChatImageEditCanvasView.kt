package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PointF
import android.graphics.RectF
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
import kotlin.math.abs
import kotlin.math.max
import kotlin.math.min

internal class ChatImageEditCanvasView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null
) : View(context, attrs) {
    private enum class CropDrag {
        NONE,
        MOVE,
        LEFT,
        TOP,
        RIGHT,
        BOTTOM,
        TOP_LEFT,
        TOP_RIGHT,
        BOTTOM_LEFT,
        BOTTOM_RIGHT
    }

    private val imageMatrix = Matrix()
    private val inverseMatrix = Matrix()
    private val imageRect = RectF()
    private val cropPaint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val dimPaint = Paint()
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        typeface = android.graphics.Typeface.DEFAULT_BOLD
    }
    private val basePaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)

    private var baseBitmap: Bitmap? = null
    private var mosaicBitmap: Bitmap? = null
    private val operations = mutableListOf<ChatImageEditOp>()
    private val redoOperations = mutableListOf<ChatImageEditOp>()
    private var activePath: Path? = null
    private var activeLabel: ChatImageEditOp.Label? = null
    private var lastBitmapPoint: PointF? = null
    private var cropRect: RectF? = null
    private var cropDrag = CropDrag.NONE
    private var currentTool = ChatImageEditTool.BRUSH
    private var currentColor = Color.WHITE
    var onHistoryChanged: (() -> Unit)? = null

    init {
        setBackgroundColor(Color.BLACK)
        cropPaint.apply {
            color = Color.WHITE
            style = Paint.Style.STROKE
            strokeWidth = dp(2).toFloat()
        }
        dimPaint.color = Color.parseColor("#88000000")
    }

    fun setBitmap(bitmap: Bitmap) {
        baseBitmap = bitmap
        mosaicBitmap = null
        operations.clear()
        redoOperations.clear()
        cropRect = null
        updateImageMatrix()
        invalidate()
    }

    fun setTool(tool: ChatImageEditTool) {
        currentTool = tool
        if (tool == ChatImageEditTool.CROP) {
            ensureCropRect()
        }
        invalidate()
    }

    fun setBrushColor(color: Int) {
        currentColor = color
    }

    fun addText(value: String) {
        val clean = value.trim().takeIf { it.isNotEmpty() } ?: return
        val bitmap = baseBitmap ?: return
        val scale = matrixScale().coerceAtLeast(0.01f)
        operations.add(
            ChatImageEditOp.Label(
                value = clean,
                x = bitmap.width * 0.5f,
                y = bitmap.height * 0.5f,
                color = currentColor,
                textSize = sp(30) / scale,
                sticker = false
            )
        )
        redoOperations.clear()
        onHistoryChanged?.invoke()
        currentTool = ChatImageEditTool.TEXT
        invalidate()
    }

    fun addSticker(value: String) {
        val clean = value.trim().takeIf { it.isNotEmpty() } ?: return
        val bitmap = baseBitmap ?: return
        val scale = matrixScale().coerceAtLeast(0.01f)
        operations.add(
            ChatImageEditOp.Label(
                value = clean,
                x = bitmap.width * 0.5f,
                y = bitmap.height * 0.5f,
                color = Color.WHITE,
                textSize = sp(42) / scale,
                sticker = true
            )
        )
        redoOperations.clear()
        onHistoryChanged?.invoke()
        currentTool = ChatImageEditTool.STICKER
        invalidate()
    }

    fun undo(): Boolean {
        if (operations.isEmpty()) return false
        redoOperations.add(operations.removeAt(operations.lastIndex))
        invalidate()
        onHistoryChanged?.invoke()
        return true
    }

    fun redo(): Boolean {
        if (redoOperations.isEmpty()) return false
        operations.add(redoOperations.removeAt(redoOperations.lastIndex))
        invalidate()
        onHistoryChanged?.invoke()
        return true
    }

    fun canUndo(): Boolean = operations.isNotEmpty()
    fun canRedo(): Boolean = redoOperations.isNotEmpty()

    fun applyCropIfActive() {
        val rect = cropRect ?: return
        val bitmap = renderEditedBitmap(rect)
        baseBitmap = bitmap
        mosaicBitmap = null
        operations.clear()
        redoOperations.clear()
        onHistoryChanged?.invoke()
        cropRect = null
        currentTool = ChatImageEditTool.BRUSH
        updateImageMatrix()
        invalidate()
    }

    fun renderEditedBitmap(): Bitmap {
        return renderEditedBitmap(null)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        updateImageMatrix()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val bitmap = baseBitmap ?: return
        canvas.drawBitmap(bitmap, imageMatrix, basePaint)
        canvas.save()
        canvas.concat(imageMatrix)
        canvas.clipRect(0f, 0f, bitmap.width.toFloat(), bitmap.height.toFloat())
        drawOperations(canvas)
        canvas.restore()
        drawCropOverlay(canvas)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val point = eventToBitmap(event.x, event.y) ?: return true
        when (currentTool) {
            ChatImageEditTool.CROP -> handleCropTouch(event, point)
            ChatImageEditTool.TEXT,
            ChatImageEditTool.STICKER -> handleLabelTouch(event, point)
            ChatImageEditTool.BRUSH,
            ChatImageEditTool.MOSAIC -> handleStrokeTouch(event, point)
        }
        return true
    }

    private fun handleStrokeTouch(event: MotionEvent, point: PointF) {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activePath = Path().apply { moveTo(point.x, point.y) }
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val path = activePath ?: return
                val last = lastBitmapPoint
                if (last == null) {
                    path.lineTo(point.x, point.y)
                } else {
                    path.quadTo(last.x, last.y, (last.x + point.x) / 2f, (last.y + point.y) / 2f)
                }
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                activePath?.let { path ->
                    path.lineTo(point.x, point.y)
                    val width = dp(if (currentTool == ChatImageEditTool.MOSAIC) 24 else 5) /
                        matrixScale().coerceAtLeast(0.01f)
                    operations.add(
                        ChatImageEditOp.Stroke(
                            path = Path(path),
                            color = currentColor,
                            width = width,
                            mosaic = currentTool == ChatImageEditTool.MOSAIC
                        )
                    )
                    redoOperations.clear()
                    onHistoryChanged?.invoke()
                }
                activePath = null
                lastBitmapPoint = null
                invalidate()
            }
        }
    }

    private fun handleLabelTouch(event: MotionEvent, point: PointF) {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activeLabel = findLabelAt(point)
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val label = activeLabel ?: return
                val last = lastBitmapPoint ?: point
                label.x += point.x - last.x
                label.y += point.y - last.y
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                activeLabel = null
                lastBitmapPoint = null
            }
        }
    }

    private fun handleCropTouch(event: MotionEvent, point: PointF) {
        ensureCropRect()
        val rect = cropRect ?: return
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                cropDrag = cropDragFor(point, rect)
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val last = lastBitmapPoint ?: point
                moveCropRect(rect, cropDrag, point.x - last.x, point.y - last.y)
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                cropDrag = CropDrag.NONE
                lastBitmapPoint = null
            }
        }
    }

    private fun drawOperations(canvas: Canvas) {
        val mosaic = ensureMosaicBitmap()
        operations.forEach { op ->
            drawOperation(canvas, op, mosaic)
        }
        activePath?.let { path ->
            drawOperation(
                canvas,
                ChatImageEditOp.Stroke(
                    path = path,
                    color = currentColor,
                    width = dp(if (currentTool == ChatImageEditTool.MOSAIC) 24 else 5) /
                        matrixScale().coerceAtLeast(0.01f),
                    mosaic = currentTool == ChatImageEditTool.MOSAIC
                ),
                mosaic
            )
        }
    }

    private fun drawOperation(canvas: Canvas, op: ChatImageEditOp, mosaic: Bitmap?) {
        when (op) {
            is ChatImageEditOp.Stroke -> {
                if (op.mosaic && mosaic != null) {
                    canvas.save()
                    canvas.clipPath(op.path)
                    canvas.drawBitmap(mosaic, 0f, 0f, basePaint)
                    canvas.restore()
                } else {
                    strokePaint.color = op.color
                    strokePaint.strokeWidth = op.width
                    canvas.drawPath(op.path, strokePaint)
                }
            }
            is ChatImageEditOp.Label -> {
                textPaint.color = op.color
                textPaint.textSize = op.textSize
                textPaint.textAlign = Paint.Align.CENTER
                if (!op.sticker) {
                    textPaint.setShadowLayer(op.textSize * 0.08f, 0f, 0f, Color.BLACK)
                } else {
                    textPaint.clearShadowLayer()
                }
                canvas.drawText(op.value, op.x, op.y, textPaint)
                textPaint.clearShadowLayer()
            }
        }
    }

    private fun drawCropOverlay(canvas: Canvas) {
        if (currentTool != ChatImageEditTool.CROP) return
        val rect = cropRect ?: return
        val mapped = RectF(rect)
        imageMatrix.mapRect(mapped)
        canvas.drawRect(imageRect.left, imageRect.top, imageRect.right, mapped.top, dimPaint)
        canvas.drawRect(imageRect.left, mapped.bottom, imageRect.right, imageRect.bottom, dimPaint)
        canvas.drawRect(imageRect.left, mapped.top, mapped.left, mapped.bottom, dimPaint)
        canvas.drawRect(mapped.right, mapped.top, imageRect.right, mapped.bottom, dimPaint)
        canvas.drawRect(mapped, cropPaint)
        val thirdW = mapped.width() / 3f
        val thirdH = mapped.height() / 3f
        canvas.drawLine(mapped.left + thirdW, mapped.top, mapped.left + thirdW, mapped.bottom, cropPaint)
        canvas.drawLine(mapped.left + thirdW * 2f, mapped.top, mapped.left + thirdW * 2f, mapped.bottom, cropPaint)
        canvas.drawLine(mapped.left, mapped.top + thirdH, mapped.right, mapped.top + thirdH, cropPaint)
        canvas.drawLine(mapped.left, mapped.top + thirdH * 2f, mapped.right, mapped.top + thirdH * 2f, cropPaint)
    }

    private fun renderEditedBitmap(crop: RectF?): Bitmap {
        val source = baseBitmap ?: error("image is not loaded")
        val rect = crop ?: RectF(0f, 0f, source.width.toFloat(), source.height.toFloat())
        val left = rect.left.toInt().coerceIn(0, source.width - 1)
        val top = rect.top.toInt().coerceIn(0, source.height - 1)
        val right = rect.right.toInt().coerceIn(left + 1, source.width)
        val bottom = rect.bottom.toInt().coerceIn(top + 1, source.height)
        val output = Bitmap.createBitmap(right - left, bottom - top, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(output)
        canvas.drawBitmap(source, -left.toFloat(), -top.toFloat(), basePaint)
        canvas.save()
        canvas.translate(-left.toFloat(), -top.toFloat())
        canvas.clipRect(left.toFloat(), top.toFloat(), right.toFloat(), bottom.toFloat())
        drawOperations(canvas)
        canvas.restore()
        return output
    }

    private fun ensureMosaicBitmap(): Bitmap? {
        mosaicBitmap?.let { return it }
        val source = baseBitmap ?: return null
        val smallWidth = max(8, source.width / 24)
        val smallHeight = max(8, source.height / 24)
        val small = Bitmap.createScaledBitmap(source, smallWidth, smallHeight, false)
        return Bitmap.createScaledBitmap(small, source.width, source.height, false).also {
            mosaicBitmap = it
            if (small !== it) small.recycle()
        }
    }

    private fun updateImageMatrix() {
        val bitmap = baseBitmap ?: return
        if (width <= 0 || height <= 0) return
        val scale = min(width / bitmap.width.toFloat(), height / bitmap.height.toFloat())
        val dx = (width - bitmap.width * scale) / 2f
        val dy = (height - bitmap.height * scale) / 2f
        imageMatrix.reset()
        imageMatrix.postScale(scale, scale)
        imageMatrix.postTranslate(dx, dy)
        imageMatrix.invert(inverseMatrix)
        imageRect.set(0f, 0f, bitmap.width.toFloat(), bitmap.height.toFloat())
        imageMatrix.mapRect(imageRect)
    }

    private fun eventToBitmap(x: Float, y: Float): PointF? {
        val bitmap = baseBitmap ?: return null
        val points = floatArrayOf(x, y)
        inverseMatrix.mapPoints(points)
        return PointF(
            points[0].coerceIn(0f, bitmap.width.toFloat()),
            points[1].coerceIn(0f, bitmap.height.toFloat())
        )
    }

    private fun matrixScale(): Float {
        val values = FloatArray(9)
        imageMatrix.getValues(values)
        return values[Matrix.MSCALE_X]
    }

    private fun ensureCropRect() {
        val bitmap = baseBitmap ?: return
        if (cropRect != null) return
        val insetX = bitmap.width * 0.08f
        val insetY = bitmap.height * 0.08f
        cropRect = RectF(insetX, insetY, bitmap.width - insetX, bitmap.height - insetY)
    }

    private fun cropDragFor(point: PointF, rect: RectF): CropDrag {
        val threshold = dp(28) / matrixScale().coerceAtLeast(0.01f)
        val nearLeft = abs(point.x - rect.left) <= threshold
        val nearRight = abs(point.x - rect.right) <= threshold
        val nearTop = abs(point.y - rect.top) <= threshold
        val nearBottom = abs(point.y - rect.bottom) <= threshold
        return when {
            nearLeft && nearTop -> CropDrag.TOP_LEFT
            nearRight && nearTop -> CropDrag.TOP_RIGHT
            nearLeft && nearBottom -> CropDrag.BOTTOM_LEFT
            nearRight && nearBottom -> CropDrag.BOTTOM_RIGHT
            nearLeft -> CropDrag.LEFT
            nearRight -> CropDrag.RIGHT
            nearTop -> CropDrag.TOP
            nearBottom -> CropDrag.BOTTOM
            rect.contains(point.x, point.y) -> CropDrag.MOVE
            else -> CropDrag.NONE
        }
    }

    private fun moveCropRect(rect: RectF, drag: CropDrag, dx: Float, dy: Float) {
        val bitmap = baseBitmap ?: return
        val minSize = min(bitmap.width, bitmap.height) * 0.18f
        when (drag) {
            CropDrag.MOVE -> rect.offset(dx, dy)
            CropDrag.LEFT, CropDrag.TOP_LEFT, CropDrag.BOTTOM_LEFT -> rect.left += dx
            else -> Unit
        }
        when (drag) {
            CropDrag.RIGHT, CropDrag.TOP_RIGHT, CropDrag.BOTTOM_RIGHT -> rect.right += dx
            else -> Unit
        }
        when (drag) {
            CropDrag.TOP, CropDrag.TOP_LEFT, CropDrag.TOP_RIGHT -> rect.top += dy
            else -> Unit
        }
        when (drag) {
            CropDrag.BOTTOM, CropDrag.BOTTOM_LEFT, CropDrag.BOTTOM_RIGHT -> rect.bottom += dy
            else -> Unit
        }
        if (rect.width() < minSize) rect.right = rect.left + minSize
        if (rect.height() < minSize) rect.bottom = rect.top + minSize
        if (rect.left < 0f) rect.offset(-rect.left, 0f)
        if (rect.top < 0f) rect.offset(0f, -rect.top)
        if (rect.right > bitmap.width) rect.offset(bitmap.width - rect.right, 0f)
        if (rect.bottom > bitmap.height) rect.offset(0f, bitmap.height - rect.bottom)
    }

    private fun findLabelAt(point: PointF): ChatImageEditOp.Label? {
        return operations.asReversed().firstNotNullOfOrNull { op ->
            val label = op as? ChatImageEditOp.Label ?: return@firstNotNullOfOrNull null
            textPaint.textSize = label.textSize
            val width = textPaint.measureText(label.value)
            val height = label.textSize * 1.25f
            val rect = RectF(
                label.x - width / 2f,
                label.y - height,
                label.x + width / 2f,
                label.y + height * 0.25f
            )
            label.takeIf { rect.contains(point.x, point.y) }
        }
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

    private fun sp(value: Int): Float {
        return android.util.TypedValue.applyDimension(android.util.TypedValue.COMPLEX_UNIT_SP, value.toFloat(), resources.displayMetrics)
    }
}
