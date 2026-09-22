package com.elon.app

import android.os.Handler
import android.os.Looper
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import okhttp3.OkHttpClient
import org.json.JSONObject

internal class GroupAssistantFeature(
    private val activity: AppCompatActivity, http: OkHttpClient, server: String,
    private val read: suspend (JSONObject) -> JSONObject,
) : DefaultLifecycleObserver {
    private val api = GroupAssistantApi(activity, http, server)
    private val sync = GroupAssistantSync(api, read)
    private val ui = GroupAssistantUi(activity, api, sync, read)
    private val handler = Handler(Looper.getMainLooper())
    private var job: Job? = null
    private var resumed = false
    private var lastRun = 0L
    private var owner = ""
    private val tick = Runnable { refresh() }
    init { activity.lifecycle.addObserver(this) }
    fun show(group: AppGroup) { job?.cancel(); ui.show(group) }
    fun refresh() {
        if (!resumed) return
        handler.removeCallbacks(tick); handler.postDelayed(tick, 300_000)
        if (job?.isActive == true || ui.open || !AuthManager.isLoggedIn(activity)) return
        if (owner != socialSession(activity)) { owner = socialSession(activity); lastRun = 0 }
        val elapsed = android.os.SystemClock.elapsedRealtime()
        if (lastRun != 0L && elapsed - lastRun < 300_000) return
        lastRun = elapsed
        val expected = owner
        job = activity.lifecycleScope.launch {
            try {
                val items = api.call().getJSONArray("items")
                // Server sorts least recently checked first; bounded work rotates across subscriptions.
                for (i in 0 until minOf(10, items.length())) {
                    if (!resumed || ui.open || socialSession(activity) != expected) break
                    val item = items.getJSONObject(i)
                    sync.one(item.getString("group_id"), item)
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { /* Retry next foreground interval, never clear shared results. */ }
            finally { if (resumed) { handler.removeCallbacks(tick); handler.postDelayed(tick, 300_000) } }
        }
    }
    override fun onResume(owner: LifecycleOwner) { resumed = true; ui.resume(); refresh() }
    override fun onPause(owner: LifecycleOwner) { resumed = false; job?.cancel(); handler.removeCallbacks(tick); ui.pause() }
    override fun onDestroy(owner: LifecycleOwner) { job?.cancel(); handler.removeCallbacks(tick); ui.close(); activity.lifecycle.removeObserver(this) }
}
