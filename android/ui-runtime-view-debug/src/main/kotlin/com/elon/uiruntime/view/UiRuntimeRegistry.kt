package com.elon.uiruntime.view

import android.app.Activity
import android.app.Application
import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import java.lang.ref.WeakReference
import java.util.WeakHashMap
import java.util.concurrent.atomic.AtomicLong

internal class UiRuntimeRegistry(
    private val application: Application,
    private val onActivityChanged: () -> Unit,
) : Application.ActivityLifecycleCallbacks {
    private var currentActivity = WeakReference<Activity>(null)
    private val treeRevision = AtomicLong(0)
    private val viewsByRuntimeId = LinkedHashMap<String, WeakReference<View>>()
    private val runtimeIdByView = WeakHashMap<View, String>()
    private val definitionByRuntimeId = LinkedHashMap<String, String>()
    private val instanceByRuntimeId = LinkedHashMap<String, String?>()

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
        return TreeSnapshotMessage(
            treeRevision = treeRevision.incrementAndGet(),
            nodes = nodes,
        )
    }

    fun nextTreeRevision(): Long = treeRevision.incrementAndGet()

    fun resolve(target: LivePatchTarget): List<View> {
        val requestedRuntimeId = target.runtimeNodeId
        if (!requestedRuntimeId.isNullOrBlank()) {
            return listOfNotNull(viewsByRuntimeId[requestedRuntimeId]?.get())
        }
        val definitionId = target.definitionId ?: return emptyList()
        return definitionByRuntimeId.entries
            .asSequence()
            .filter { (_, definition) -> definition == definitionId }
            .filter { (runtimeId, _) ->
                target.instanceKey == null || instanceByRuntimeId[runtimeId] == target.instanceKey
            }
            .mapNotNull { (runtimeId, _) -> viewsByRuntimeId[runtimeId]?.get() }
            .toList()
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
}
