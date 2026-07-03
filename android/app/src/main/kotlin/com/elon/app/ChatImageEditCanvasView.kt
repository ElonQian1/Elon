package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
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
    private val basePaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG)
    private val annotationIconBitmap: Bitmap? by lazy {
        BitmapFactory.decodeResource(resources, R.drawable.ic_chat_image_tool_annotation)
    }
    private val completedAnnotationIconBitmap: Bitmap? by lazy {
        BitmapFactory.decodeResource(resources, R.drawable.ic_chat_image_tool_annotation_filled)
    }
    private val annotationBubbleRenderer = ChatImageAnnotationBubbleRenderer(context)

    private var baseBitmap: Bitmap? = null
    private var mosaicBitmap: Bitmap? = null
    private val operations = mutableListOf<ChatImageEditOp>()
    private val redoOperations = mutableListOf<ChatImageEditOp>()
    private var activePath: Path? = null
    private var activeShapeStart: PointF? = null
    private var activeShapeRect: RectF? = null
    private var activeLineStart: PointF? = null
    private var activeLineEnd: PointF? = null
    private var lastBitmapPoint: PointF? = null
    private var pressedAnnotationIndex: Int? = null
    private var expandedAnnotationIndex: Int? = null
    private var currentTool = ChatImageEditTool.ANNOTATION
    private var currentColor = Color.WHITE
    var onHistoryChanged: (() -> Unit)? = null
    var onAnnotationRequested: ((Int) -> Unit)? = null

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
        activeLineStart = null
        activeLineEnd = null
        pressedAnnotationIndex = null
        expandedAnnotationIndex = null
        updateImageMatrix()
        invalidate()
    }

    fun setTool(tool: ChatImageEditTool) {
        currentTool = tool
        activeShapeStart = null
        activeShapeRect = null
        activeLineStart = null
        activeLineEnd = null
        invalidate()
    }

    fun setBrushColor(color: Int) {
        currentColor = color
    }

    fun undo(): Boolean {
        if (operations.isEmpty()) return false
        redoOperations.add(operations.removeAt(operations.lastIndex))
        normalizeExpandedAnnotationIndex()
        invalidate()
        onHistoryChanged?.invoke()
        return true
    }

    fun redo(): Boolean {
        if (redoOperations.isEmpty()) return false
        operations.add(redoOperations.removeAt(redoOperations.lastIndex))
        normalizeExpandedAnnotationIndex()
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
        drawOperations(canvas, includeAnnotationIcons = true, respectExpandedAnnotations = false)
        return output
    }

    fun annotationPanelBounds(index: Int): RectF? {
        val annotation = annotationAt(index) ?: return null
        return RectF(annotation.bounds).also { imageMatrix.mapRect(it) }
    }

    fun annotationNote(index: Int): String {
        return annotationAt(index)?.note.orEmpty()
    }

    fun updateAnnotationNote(index: Int, note: String) {
        val annotation = annotationAt(index) ?: return
        if (annotation.note == note) {
            if (note.trim().isNotEmpty() && expandedAnnotationIndex == index) {
                expandedAnnotationIndex = null
                invalidate()
            }
            return
        }
        annotation.note = note
        expandedAnnotationIndex = if (note.trim().isEmpty()) index else null
        redoOperations.clear()
        invalidate()
        onHistoryChanged?.invoke()
    }

    fun exportAnnotations(): List<ChatImageAnnotation> {
        val bitmap = baseBitmap ?: return emptyList()
        val bitmapWidth = bitmap.width.toFloat().coerceAtLeast(1f)
        val bitmapHeight = bitmap.height.toFloat().coerceAtLeast(1f)
        return operations.mapNotNull { op ->
            val annotation = op as? ChatImageEditOp.Annotation ?: return@mapNotNull null
            val note = annotation.note.trim()
            if (note.isEmpty()) return@mapNotNull null
            val bounds = RectF(annotation.bounds).apply {
                left = left.coerceIn(0f, bitmapWidth)
                top = top.coerceIn(0f, bitmapHeight)
                right = right.coerceIn(0f, bitmapWidth)
                bottom = bottom.coerceIn(0f, bitmapHeight)
            }
            if (bounds.width() <= 0f || bounds.height() <= 0f) return@mapNotNull null
            val iconRect = annotationIconRectOnBitmap(annotation)
            ChatImageAnnotation(
                x = bounds.left / bitmapWidth,
                y = bounds.top / bitmapHeight,
                width = bounds.width() / bitmapWidth,
                height = bounds.height() / bitmapHeight,
                note = note,
                iconX = iconRect.left / bitmapWidth,
                iconY = iconRect.top / bitmapHeight,
                iconWidth = iconRect.width() / bitmapWidth,
                iconHeight = iconRect.height() / bitmapHeight
            )
        }
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
        drawOperations(canvas, includeAnnotationIcons = false, respectExpandedAnnotations = true)
        canvas.restore()
        drawExpandedAnnotationBubble(canvas)
        drawAnnotationIcons(canvas)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (handleAnnotationIconTouch(event)) return true
        val point = eventToBitmap(event.x, event.y) ?: return true
        when (currentTool) {
            ChatImageEditTool.ANNOTATION -> handleAnnotationTouch(event, point)
            ChatImageEditTool.HORIZONTAL_LINE -> handleLineTouch(event, point)
            ChatImageEditTool.CIRCLE,
            ChatImageEditTool.SQUARE -> handleShapeTouch(event, point)
            ChatImageEditTool.MOSAIC -> handleStrokeTouch(event, point)
        }
        return true
    }

    private fun handleAnnotationIconTouch(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val hitIndex = findAnnotationIconAt(event.x, event.y) ?: return false
                pressedAnnotationIndex = hitIndex
                parent?.requestDisallowInterceptTouchEvent(true)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                return pressedAnnotationIndex != null
            }
            MotionEvent.ACTION_UP -> {
                val hitIndex = pressedAnnotationIndex ?: return false
                pressedAnnotationIndex = null
                if (findAnnotationIconAt(event.x, event.y) == hitIndex) {
                    handleAnnotationIconClick(hitIndex)
                }
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                if (pressedAnnotationIndex == null) return false
                pressedAnnotationIndex = null
                return true
            }
        }
        return false
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

    private fun handleLineTouch(event: MotionEvent, point: PointF) {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activeLineStart = point
                activeLineEnd = PointF(point.x, point.y)
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val start = activeLineStart ?: return
                activeLineEnd = PointF(point.x, start.y)
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                val start = activeLineStart
                val end = if (start == null) null else PointF(point.x, start.y)
                if (event.actionMasked == MotionEvent.ACTION_UP && start != null && end != null && isLineLargeEnough(start, end)) {
                    operations.add(
                        ChatImageEditOp.HorizontalLine(
                            start = PointF(start.x, start.y),
                            end = end,
                            color = currentColor,
                            width = shapeStrokeWidth()
                        )
                    )
                    redoOperations.clear()
                    onHistoryChanged?.invoke()
                }
                activeLineStart = null
                activeLineEnd = null
                lastBitmapPoint = null
                invalidate()
            }
        }
    }

    private fun handleAnnotationTouch(event: MotionEvent, point: PointF) {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activeShapeStart = point
                activeShapeRect = shapeRectFrom(point, point)
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val start = activeShapeStart ?: return
                activeShapeRect = shapeRectFrom(start, point)
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                val start = activeShapeStart
                val rect = if (start == null) null else shapeRectFrom(start, point)
                if (event.actionMasked == MotionEvent.ACTION_UP && rect != null && isShapeLargeEnough(rect)) {
                    operations.add(
                        ChatImageEditOp.Annotation(
                            bounds = RectF(rect),
                            color = currentColor,
                            width = shapeStrokeWidth()
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

    private fun handleShapeTouch(event: MotionEvent, point: PointF) {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activeShapeStart = point
                activeShapeRect = shapeRectFrom(point, point)
                lastBitmapPoint = point
                parent?.requestDisallowInterceptTouchEvent(true)
            }
            MotionEvent.ACTION_MOVE -> {
                val start = activeShapeStart ?: return
                activeShapeRect = shapeRectFrom(start, point)
                lastBitmapPoint = point
                invalidate()
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL -> {
                val start = activeShapeStart
                val rect = if (start == null) null else shapeRectFrom(start, point)
                if (event.actionMasked == MotionEvent.ACTION_UP && rect != null && isShapeLargeEnough(rect)) {
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

    private fun drawOperations(
        canvas: Canvas,
        includeAnnotationIcons: Boolean,
        respectExpandedAnnotations: Boolean
    ) {
        val mosaic = ensureMosaicBitmap()
        operations.forEachIndexed { index, op ->
            drawOperation(
                canvas,
                op,
                mosaic,
                annotationBoundsExpanded = respectExpandedAnnotations && expandedAnnotationIndex == index
            )
        }
        if (includeAnnotationIcons) {
            operations.forEach { op ->
                (op as? ChatImageEditOp.Annotation)?.let { drawAnnotationIconOnBitmap(canvas, it) }
            }
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
                mosaic,
                annotationBoundsExpanded = true
            )
        }
        val lineStart = activeLineStart
        val lineEnd = activeLineEnd
        if (lineStart != null && lineEnd != null && isLineLargeEnough(lineStart, lineEnd)) {
            drawOperation(
                canvas,
                ChatImageEditOp.HorizontalLine(
                    start = lineStart,
                    end = lineEnd,
                    color = currentColor,
                    width = shapeStrokeWidth()
                ),
                mosaic
            )
        }
        activeShapeRect?.let { rect ->
            if (isShapeLargeEnough(rect)) {
                val op = if (currentTool == ChatImageEditTool.ANNOTATION) {
                    ChatImageEditOp.Annotation(
                        bounds = RectF(rect),
                        color = currentColor,
                        width = shapeStrokeWidth()
                    )
                } else {
                    ChatImageEditOp.Shape(
                        bounds = RectF(rect),
                        color = currentColor,
                        width = shapeStrokeWidth(),
                        shape = activeShapeForTool()
                    )
                }
                drawOperation(canvas, op, mosaic, annotationBoundsExpanded = true)
            }
        }
    }

    private fun drawOperation(
        canvas: Canvas,
        op: ChatImageEditOp,
        mosaic: Bitmap?,
        annotationBoundsExpanded: Boolean = false
    ) {
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
            is ChatImageEditOp.Shape -> {
                strokePaint.color = op.color
                strokePaint.strokeWidth = op.width
                when (op.shape) {
                    ChatImageEditShape.CIRCLE -> canvas.drawOval(op.bounds, strokePaint)
                    ChatImageEditShape.SQUARE -> canvas.drawRect(op.bounds, strokePaint)
                }
            }
            is ChatImageEditOp.HorizontalLine -> {
                strokePaint.color = op.color
                strokePaint.strokeWidth = op.width
                canvas.drawLine(op.start.x, op.start.y, op.end.x, op.end.y, strokePaint)
            }
            is ChatImageEditOp.Annotation -> {
                if (op.note.trim().isEmpty() || annotationBoundsExpanded) {
                    strokePaint.color = op.color
                    strokePaint.strokeWidth = op.width
                    canvas.drawRect(op.bounds, strokePaint)
                }
            }
        }
    }

    private fun drawExpandedAnnotationBubble(canvas: Canvas) {
        val index = expandedAnnotationIndex ?: return
        val annotation = annotationAt(index) ?: return
        if (annotation.note.trim().isEmpty()) return
        val anchor = RectF(annotation.bounds).also { imageMatrix.mapRect(it) }
        annotationBubbleRenderer.draw(canvas, annotation.note, anchor, width, height)
    }

    private fun drawAnnotationIcons(canvas: Canvas) {
        operations.forEach { op ->
            val annotation = op as? ChatImageEditOp.Annotation ?: return@forEach
            val rect = annotationIconRectOnView(annotation) ?: return@forEach
            val icon = annotationIconFor(annotation) ?: return@forEach
            canvas.drawBitmap(icon, null, rect, basePaint)
        }
    }

    private fun drawAnnotationIconOnBitmap(canvas: Canvas, annotation: ChatImageEditOp.Annotation) {
        val icon = annotationIconFor(annotation) ?: return
        val rect = annotationIconRectOnBitmap(annotation)
        canvas.drawBitmap(icon, null, rect, basePaint)
    }

    private fun annotationIconFor(annotation: ChatImageEditOp.Annotation): Bitmap? {
        return if (annotation.note.trim().isNotEmpty()) {
            completedAnnotationIconBitmap ?: annotationIconBitmap
        } else {
            annotationIconBitmap
        }
    }

    private fun handleAnnotationIconClick(index: Int) {
        val annotation = annotationAt(index) ?: return
        if (annotation.note.trim().isEmpty()) {
            expandedAnnotationIndex = index
            invalidate()
            onAnnotationRequested?.invoke(index)
            return
        }
        if (expandedAnnotationIndex == index) {
            onAnnotationRequested?.invoke(index)
        } else {
            expandedAnnotationIndex = index
            invalidate()
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

    private fun shapeRectFrom(start: PointF, end: PointF): RectF {
        return RectF(
            min(start.x, end.x),
            min(start.y, end.y),
            max(start.x, end.x),
            max(start.y, end.y)
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

    private fun isShapeLargeEnough(rect: RectF): Boolean {
        val minSize = minShapeSize()
        return rect.width() >= minSize && rect.height() >= minSize
    }

    private fun isLineLargeEnough(start: PointF, end: PointF): Boolean {
        return kotlin.math.abs(end.x - start.x) >= dp(18) / matrixScale().coerceAtLeast(0.01f)
    }

    private fun findAnnotationIconAt(x: Float, y: Float): Int? {
        for (index in operations.lastIndex downTo 0) {
            val annotation = operations[index] as? ChatImageEditOp.Annotation ?: continue
            val rect = annotationIconRectOnView(annotation) ?: continue
            val hitRect = RectF(rect).apply {
                val extraX = max(0f, (dp(48).toFloat() - width()) / 2f)
                val extraY = max(0f, (dp(48).toFloat() - height()) / 2f)
                inset(-extraX, -extraY)
            }
            if (hitRect.contains(x, y)) return index
        }
        return null
    }

    private fun annotationIconRectOnView(annotation: ChatImageEditOp.Annotation): RectF? {
        if (baseBitmap == null) return null
        val points = floatArrayOf(
            annotation.bounds.left,
            annotation.bounds.bottom,
            annotation.bounds.right,
            annotation.bounds.bottom
        )
        imageMatrix.mapPoints(points)
        val size = annotationIconSize()
        val pad = dp(5).toFloat()
        val edgePad = dp(8).toFloat()
        val rightLeft = points[2] + pad
        val leftLeft = points[0] - pad - size
        val rawLeft = if (rightLeft + size <= width - edgePad) rightLeft else leftLeft
        val left = rawLeft.coerceIn(edgePad, max(edgePad, width - size - edgePad))
        val top = points[1] - size * 0.9f
        return RectF(left, top, left + size, top + size)
    }

    private fun annotationIconRectOnBitmap(annotation: ChatImageEditOp.Annotation, iconSize: Float, pad: Float): RectF {
        val bitmapWidth = baseBitmap?.width?.toFloat() ?: 0f
        val edgePad = dp(8) / matrixScale().coerceAtLeast(0.01f)
        val rightLeft = annotation.bounds.right + pad
        val leftLeft = annotation.bounds.left - pad - iconSize
        val rawLeft = if (rightLeft + iconSize <= bitmapWidth - edgePad) rightLeft else leftLeft
        val left = rawLeft.coerceIn(edgePad, max(edgePad, bitmapWidth - iconSize - edgePad))
        val top = annotation.bounds.bottom - iconSize * 0.9f
        return RectF(left, top, left + iconSize, top + iconSize)
    }

    private fun annotationIconRectOnBitmap(annotation: ChatImageEditOp.Annotation): RectF {
        val scale = matrixScale().coerceAtLeast(0.01f)
        val iconSize = annotationIconSize() / scale
        val pad = dp(5) / scale
        return annotationIconRectOnBitmap(annotation, iconSize, pad)
    }

    private fun annotationAt(index: Int): ChatImageEditOp.Annotation? {
        return operations.getOrNull(index) as? ChatImageEditOp.Annotation
    }

    private fun normalizeExpandedAnnotationIndex() {
        val index = expandedAnnotationIndex ?: return
        if (annotationAt(index) == null) {
            expandedAnnotationIndex = null
        }
    }

    private fun annotationIconSize(): Float {
        return dp(36).toFloat()
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }

}
