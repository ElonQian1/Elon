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
    private val imageMatrix = Matrix()
    private val inverseMatrix = Matrix()
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
    private var activeShapeStart: PointF? = null
    private var activeShapeRect: RectF? = null
    private var lastBitmapPoint: PointF? = null
    private var currentTool = ChatImageEditTool.BRUSH
    private var currentColor = Color.WHITE
    var onHistoryChanged: (() -> Unit)? = null

    init {
        setBackgroundColor(Color.BLACK)
    }

    fun setBitmap(bitmap: Bitmap) {
        baseBitmap = bitmap
        mosaicBitmap = null
        operations.clear()
        redoOperations.clear()
        activeShapeStart = null
        activeShapeRect = null
        updateImageMatrix()
        invalidate()
    }

    fun setTool(tool: ChatImageEditTool) {
        currentTool = tool
        activeShapeStart = null
        activeShapeRect = null
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
                textSize = sp(30) / scale
            )
        )
        redoOperations.clear()
        onHistoryChanged?.invoke()
        currentTool = ChatImageEditTool.TEXT
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

    fun renderEditedBitmap(): Bitmap {
        val source = baseBitmap ?: error("image is not loaded")
        val output = Bitmap.createBitmap(source.width, source.height, Bitmap.Config.ARGB_8888)
        val canvas = Canvas(output)
        canvas.drawBitmap(source, 0f, 0f, basePaint)
        drawOperations(canvas)
        return output
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
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val point = eventToBitmap(event.x, event.y) ?: return true
        when (currentTool) {
            ChatImageEditTool.CIRCLE,
            ChatImageEditTool.SQUARE -> handleShapeTouch(event, point)
            ChatImageEditTool.TEXT -> handleLabelTouch(event, point)
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

    private fun handleShapeTouch(event: MotionEvent, point: PointF) {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activeShapeStart = point
                activeShapeRect = squareRectFrom(point, point)
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val start = activeShapeStart ?: return
                activeShapeRect = squareRectFrom(start, point)
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                val start = activeShapeStart
                val rect = if (start == null) null else squareRectFrom(start, point)
                if (event.actionMasked == MotionEvent.ACTION_UP && rect != null && rect.width() >= minShapeSize()) {
                    operations.add(
                        ChatImageEditOp.Shape(
                            bounds = RectF(rect),
                            color = currentColor,
                            width = shapeStrokeWidth(),
                            shape = activeShapeForTool()
                        )
                    )
                    redoOperations.clear()
                    onHistoryChanged?.invoke()
                }
                activeShapeStart = null
                activeShapeRect = null
                lastBitmapPoint = null
                invalidate()
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
        activeShapeRect?.let { rect ->
            if (rect.width() >= minShapeSize()) {
                drawOperation(
                    canvas,
                    ChatImageEditOp.Shape(
                        bounds = RectF(rect),
                        color = currentColor,
                        width = shapeStrokeWidth(),
                        shape = activeShapeForTool()
                    ),
                    mosaic
                )
            }
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
                textPaint.setShadowLayer(op.textSize * 0.08f, 0f, 0f, Color.BLACK)
                canvas.drawText(op.value, op.x, op.y, textPaint)
                textPaint.clearShadowLayer()
            }
            is ChatImageEditOp.Shape -> {
                strokePaint.color = op.color
                strokePaint.strokeWidth = op.width
                when (op.shape) {
                    ChatImageEditShape.CIRCLE -> canvas.drawOval(op.bounds, strokePaint)
                    ChatImageEditShape.SQUARE -> canvas.drawRect(op.bounds, strokePaint)
                }
            }
        }
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

    private fun squareRectFrom(start: PointF, end: PointF): RectF {
        val bitmap = baseBitmap ?: return RectF(start.x, start.y, start.x, start.y)
        val dx = end.x - start.x
        val dy = end.y - start.y
        val dirX = if (dx < 0f) -1f else 1f
        val dirY = if (dy < 0f) -1f else 1f
        val maxX = if (dirX > 0f) bitmap.width - start.x else start.x
        val maxY = if (dirY > 0f) bitmap.height - start.y else start.y
        val side = min(max(abs(dx), abs(dy)), min(maxX, maxY)).coerceAtLeast(0f)
        val right = start.x + side * dirX
        val bottom = start.y + side * dirY
        return RectF(
            min(start.x, right),
            min(start.y, bottom),
            max(start.x, right),
            max(start.y, bottom)
        )
    }

    private fun activeShapeForTool(): ChatImageEditShape {
        return if (currentTool == ChatImageEditTool.CIRCLE) {
            ChatImageEditShape.CIRCLE
        } else {
            ChatImageEditShape.SQUARE
        }
    }

    private fun shapeStrokeWidth(): Float {
        return dp(4) / matrixScale().coerceAtLeast(0.01f)
    }

    private fun minShapeSize(): Float {
        return dp(12) / matrixScale().coerceAtLeast(0.01f)
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
