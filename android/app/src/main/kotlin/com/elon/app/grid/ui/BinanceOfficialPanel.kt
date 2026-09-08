package com.elon.app.grid.ui

import android.app.Activity
import android.app.Dialog
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import com.elon.app.grid.host.BinanceHostRuntime

/** A temporary full-screen attachment restores the exact original WebView parent on return. */
internal fun showBinanceOfficialPanel(activity: Activity, runtime: BinanceHostRuntime?) {
    val host = runtime ?: return
    if (!host.begin()) return
    val page = host.view ?: return
    val parent = page.parent as? ViewGroup
    val index = parent?.indexOfChild(page) ?: 0
    val parameters = page.layoutParams
    parent?.removeView(page)
    val dialog = Dialog(activity)
    val ui = BinanceGridAppearance(activity)
    val root = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background) }
    val frame = FrameLayout(activity)
    root.addView(ui.label("币安官网 · 当前登录账户", 18f))
    root.addView(frame, LinearLayout.LayoutParams(-1, 0, 1f))
    root.addView(ui.button("返回原生网格页面", "binance-official-return") { dialog.dismiss() })
    frame.addView(page, FrameLayout.LayoutParams(-1, -1))
    dialog.setContentView(root)
    dialog.setOnDismissListener {
        if (page.parent === frame) frame.removeView(page)
        if (!activity.isDestroyed && host.view === page && parent != null && page.parent == null)
            parent.addView(page, index.coerceAtMost(parent.childCount), parameters)
    }
    dialog.window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
    dialog.show()
    dialog.window?.setLayout(-1, -1)
}
