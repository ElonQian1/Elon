package com.elon.uiruntime.view

import android.app.Activity
import android.app.Application
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import com.google.gson.Gson
import java.lang.ref.WeakReference
import java.util.WeakHashMap
import java.util.concurrent.atomic.AtomicLong

internal class UiRuntimeRegistry(
    private val application: Application,
    private val onActivityChanged: () -> Unit,
) : Application.ActivityLifecycleCallbacks {
    internal data class ResolvedTargets(
        val views: List<View>,
        val externalNodes: List<UiRuntimeExternalNode>,
    )

    private val gson = Gson()
    private var currentActivity = WeakReference<Activity>(null)
    private val treeRevision = AtomicLong(0)
    private val viewsByRuntimeId = LinkedHashMap<String, WeakReference<View>>()
    private val runtimeIdByView = WeakHashMap<View, String>()
    private val definitionByRuntimeId = LinkedHashMap<String, String>()
    private val instanceByRuntimeId = LinkedHashMap<String, String?>()
    private val externalNodes = LinkedHashMap<String, UiRuntimeExternalNode>()

    fun start() {
        application.registerActivityLifecycleCallbacks(this)
    }

    fun currentActivity(): Activity? = currentActivity.get()

    fun snapshot(): TreeSnapshotMessage {
        val activity = currentActivity.get()
            ?: return TreeSnapshotMessage(
                treeRevision = treeRevision.incrementAndGet(),
                nodes = emptyList(),
            )
        val root = activity.window?.decorView
            ?: return TreeSnapshotMessage(
                treeRevision = treeRevision.incrementAndGet(),
                nodes = emptyList(),
            )
        viewsByRuntimeId.clear()
        definitionByRuntimeId.clear()
        instanceByRuntimeId.clear()
        val screenId = activity.componentName.className
        val nodes = ArrayList<LiveUiNode>()
        visit(
            view = root,
            parentRuntimeId = null,
            screenId = screenId,
            path = "root",
            nodes = nodes,
        )
        nodes += externalNodes.values.map(::externalSnapshot)
        return TreeSnapshotMessage(
            treeRevision = treeRevision.incrementAndGet(),
            nodes = nodes,
        )
    }

    fun nextTreeRevision(): Long = treeRevision.incrementAndGet()

    fun resolve(target: LivePatchTarget): ResolvedTargets {
        val requestedRuntimeId = target.runtimeNodeId
        if (!requestedRuntimeId.isNullOrBlank()) {
            val requestedView = viewsByRuntimeId[requestedRuntimeId]?.get()
                ?.takeIf(View::isAttachedToWindow)
            val requestedExternal = externalNodes[requestedRuntimeId]
            if (requestedView != null || requestedExternal != null) {
                return ResolvedTargets(
                    views = listOfNotNull(requestedView),
                    externalNodes = listOfNotNull(requestedExternal),
                )
            }
        }
        val definitionId = target.definitionId ?: return ResolvedTargets(emptyList(), emptyList())
        val views = definitionByRuntimeId.entries
            .asSequence()
            .filter { (_, definition) -> definition == definitionId }
            .filter { (runtimeId, _) ->
                target.instanceKey == null || instanceByRuntimeId[runtimeId] == target.instanceKey
            }
            .mapNotNull { (runtimeId, _) ->
                viewsByRuntimeId[runtimeId]?.get()?.takeIf(View::isAttachedToWindow)
            }
            .toList()
        val external = externalNodes.values.filter { node ->
            node.definitionId == definitionId &&
                (target.instanceKey == null || node.instanceKey == target.instanceKey)
        }
        return ResolvedTargets(views, external)
    }

    fun persistentTarget(target: LivePatchTarget): LivePatchTarget {
        val definitionId = target.definitionId?.takeIf { it.isNotBlank() } ?: return target
        val matchingViews = definitionByRuntimeId.entries.count { (runtimeId, definition) ->
            definition == definitionId &&
                viewsByRuntimeId[runtimeId]?.get()?.isAttachedToWindow == true &&
                (target.instanceKey == null || instanceByRuntimeId[runtimeId] == target.instanceKey)
        }
        val matchingExternal = externalNodes.values.count { node ->
            node.definitionId == definitionId &&
                (target.instanceKey == null || node.instanceKey == target.instanceKey)
        }
        val canAddressStably = target.scope != "INSTANCE" ||
            target.instanceKey != null ||
            matchingViews + matchingExternal == 1
        return if (canAddressStably) target.copy(runtimeNodeId = null) else target
    }

    fun upsertExternalNode(node: UiRuntimeExternalNode) {
        externalNodes[node.runtimeNodeId] = node
        onActivityChanged()
    }

    fun removeExternalNode(runtimeNodeId: String) {
        if (externalNodes.remove(runtimeNodeId) != null) onActivityChanged()
    }

    fun clearExternalNodes() {
        if (externalNodes.isNotEmpty()) {
            externalNodes.clear()
            onActivityChanged()
        }
    }

    private fun visit(
        view: View,
        parentRuntimeId: String?,
        screenId: String,
        path: String,
        nodes: MutableList<LiveUiNode>,
    ) {
        val runtimeId = runtimeId(view)
        val resourceId = resourceName(view)
        val taggedDefinition = view.getTag(R.id.yilong_ui_node_id) as? String
        val definitionId = taggedDefinition?.takeIf { it.isNotBlank() }
            ?: resourceId?.let { "$screenId#$it" }
            ?: "$screenId@$path"
        val instanceKey = (view.getTag(R.id.yilong_ui_instance_key) as? String)
            ?.takeIf { it.isNotBlank() }
        viewsByRuntimeId[runtimeId] = WeakReference(view)
        definitionByRuntimeId[runtimeId] = definitionId
        instanceByRuntimeId[runtimeId] = instanceKey
        nodes += UiRuntimeViewAdapter.nodeSnapshot(
            view = view,
            runtimeNodeId = runtimeId,
            definitionId = definitionId,
            instanceKey = instanceKey,
            parentRuntimeNodeId = parentRuntimeId,
            screenId = screenId,
            resourceId = resourceId,
        )
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                val child = view.getChildAt(index)
                visit(
                    view = child,
                    parentRuntimeId = runtimeId,
                    screenId = screenId,
                    path = "$path/${kind(child)}[$index]",
                    nodes = nodes,
                )
            }
        }
    }

    private fun runtimeId(view: View): String = runtimeIdByView.getOrPut(view) {
        "rn_${System.identityHashCode(view).toUInt().toString(16)}"
    }

    private fun resourceName(view: View): String? {
        if (view.id == View.NO_ID) return null
        return runCatching { view.resources.getResourceName(view.id) }.getOrNull()
    }

    override fun onActivityResumed(activity: Activity) {
        currentActivity = WeakReference(activity)
        activity.window?.decorView?.post(onActivityChanged)
    }

    override fun onActivityPaused(activity: Activity) {
        if (currentActivity.get() === activity) onActivityChanged()
    }

    override fun onActivityDestroyed(activity: Activity) {
        if (currentActivity.get() === activity) {
            currentActivity.clear()
            viewsByRuntimeId.clear()
            definitionByRuntimeId.clear()
            instanceByRuntimeId.clear()
            runtimeIdByView.clear()
            externalNodes.clear()
            onActivityChanged()
        }
    }

    override fun onActivityCreated(activity: Activity, savedInstanceState: Bundle?) = Unit
    override fun onActivityStarted(activity: Activity) = Unit
    override fun onActivityStopped(activity: Activity) = Unit
    override fun onActivitySaveInstanceState(activity: Activity, outState: Bundle) = Unit

    companion object {
        fun kind(view: View): String = kindByClass(view) ?: when (view) {
            is TextView -> "android.text"
            is ImageView -> "android.image"
            is ViewGroup -> "android.container"
            else -> "android.view"
        }

        private fun kindByClass(view: View): String? = when (view.javaClass.name) {
            "com.google.android.material.button.MaterialButton" -> "material.button"
            "com.google.android.material.card.MaterialCardView" -> "material.card"
            else -> null
        }
    }

    private fun externalSnapshot(node: UiRuntimeExternalNode): LiveUiNode {
        val bounds = node.geometry
        val rect = LiveRect(
            left = bounds.leftPx,
            top = bounds.topPx,
            right = bounds.rightPx,
            bottom = bounds.bottomPx,
            width = (bounds.rightPx - bounds.leftPx).coerceAtLeast(0),
            height = (bounds.bottomPx - bounds.topPx).coerceAtLeast(0),
        )
        return LiveUiNode(
            runtimeNodeId = node.runtimeNodeId,
            definitionId = node.definitionId,
            instanceKey = node.instanceKey,
            parentRuntimeNodeId = node.parentRuntimeNodeId,
            screenId = node.screenId,
            kind = node.kind,
            text = node.text?.take(2_000),
            resourceId = null,
            className = node.className,
            source = node.source?.let(gson::toJsonTree),
            geometry = LiveGeometry(
                boundsInDisplayPx = rect,
                density = bounds.density,
                fontScale = bounds.fontScale,
                rotation = bounds.rotation,
                visible = bounds.visible,
            ),
            properties = node.properties.mapValues { (_, property) ->
                LivePropertySnapshot(
                    effective = property.effective?.toProtocolValue(),
                    measured = property.measured?.toProtocolValue(),
                    changeLevel = property.changeLevel,
                    commitMode = property.commitMode,
                    binding = property.binding?.let(gson::toJsonTree),
                    constraints = property.constraints?.let(gson::toJsonTree),
                )
            },
            capabilities = node.capabilities,
        )
    }

    private fun UiRuntimeValue.toProtocolValue(): LivePropertyValue = LivePropertyValue(
        valueType = type,
        value = gson.toJsonTree(value),
    )
}
