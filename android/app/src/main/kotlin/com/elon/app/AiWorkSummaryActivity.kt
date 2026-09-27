package com.elon.app

import android.app.DatePickerDialog
import android.content.Intent
import android.os.Bundle
import android.widget.ScrollView
import androidx.appcompat.app.AppCompatActivity
import com.google.gson.Gson
import okhttp3.OkHttpClient
import java.util.Calendar
import kotlin.concurrent.thread

class AiWorkSummaryActivity : AppCompatActivity() {
    private val http = OkHttpClient()
    private var selectedDayMillis = System.currentTimeMillis()
    private var projects: List<AppProject> = emptyList()
    private lateinit var root: ScrollView
    private fun storedProjects(): List<AppProject> {
        return loadStoredProjects(AuthManager.userDataPrefs(this), Gson(), {}, null).projects
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = elonColor(R.color.elon_bg_app)
        window.navigationBarColor = elonColor(R.color.elon_bg_app)
        projects = storedProjects()
        root = createContent()
        setContentView(root)
        refreshFromServer()
    }

    private fun createContent(): ScrollView = AiWorkSummaryContent(this, ::finish,
        ::showDatePicker, ::openProject, ::runAction).create(generateWorkSummary(projects, selectedDayMillis), formatSelectedDay())

    private fun showDatePicker() {
        val selected = Calendar.getInstance().apply { timeInMillis = selectedDayMillis }
        DatePickerDialog(this, { _, year, month, day ->
            selectedDayMillis = Calendar.getInstance().apply { set(year, month, day, 12, 0, 0); set(Calendar.MILLISECOND, 0) }.timeInMillis
            rerender()
        }, selected.get(Calendar.YEAR), selected.get(Calendar.MONTH), selected.get(Calendar.DAY_OF_MONTH)).show()
    }

    private fun openProject(project: String) {
        startActivity(Intent(this, MainActivity::class.java).apply {
            putExtra(EXTRA_OPEN_WORK_SUMMARY_PROJECT_TITLE, project)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        })
    }

    private fun runAction(item: GeneratedWorkSummaryItem, action: String) {
        if (action == "进入项目" || action == "查看项目" || action == "查看详情") {
            openProject(item.project)
            return
        }
        val prompt = if (action == "进入测试") {
            "请为${item.project}进入测试阶段，并先检查 APK 构建结果。"
        } else {
            "请处理${item.project}的事项：${item.title}。${item.suggestion}。"
        }
        startActivity(Intent(this, MainActivity::class.java).apply {
            putExtra(EXTRA_WORK_SUMMARY_AI_PROMPT, prompt)
            putExtra(EXTRA_WORK_SUMMARY_AUTO_SEND, true)
            addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP)
        })
    }
    private fun formatSelectedDay(): String {
        val selected = Calendar.getInstance().apply { timeInMillis = selectedDayMillis }
        val today = Calendar.getInstance()
        return if (selected.get(Calendar.YEAR) == today.get(Calendar.YEAR) && selected.get(Calendar.DAY_OF_YEAR) == today.get(Calendar.DAY_OF_YEAR)) "今天"
        else "${selected.get(Calendar.MONTH) + 1}月${selected.get(Calendar.DAY_OF_MONTH)}日"
    }
    private fun rerender() {
        val replacement = createContent()
        root = replacement
        setContentView(replacement)
    }
    private fun refreshFromServer() {
        if (!AuthManager.isLoggedIn(this)) return
        thread(name = "work-summary-refresh") {
            val remote = runCatching { fetchMyProjectArchive(http, ElonApplication.activeServerUrl(this), this).allProjects.map { it.toAppProject() } }.getOrNull()
            if (!remote.isNullOrEmpty()) runOnUiThread {
                val remoteIds = remote.mapTo(mutableSetOf()) { it.id }
                projects = remote + projects.filterNot { it.id in remoteIds }
                rerender()
            }
        }
    }
}
