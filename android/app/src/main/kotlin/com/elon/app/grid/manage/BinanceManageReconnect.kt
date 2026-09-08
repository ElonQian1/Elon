package com.elon.app.grid.manage

/** Rebuilds a lost page through the existing authenticated host, without replaying any operation. */
internal fun <T : Any> reconnectBinanceManagePage(
    submitting: Boolean, cancelPreparation: () -> Unit, page: () -> T?,
    begin: () -> Boolean, attach: (T) -> Unit, reload: (T) -> Unit
): Boolean {
    if (submitting) return false
    cancelPreparation()
    val previous = page()
    if (!begin()) return false
    val current = page() ?: return false
    attach(current)
    // begin already starts the initial navigation for a newly created page.
    if (current === previous) reload(current)
    return true
}
