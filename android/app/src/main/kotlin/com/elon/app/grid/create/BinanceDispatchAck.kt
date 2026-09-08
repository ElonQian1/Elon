package com.elon.app.grid.create

/** A missing WebView callback value is not proof that the network side effect did not start. */
internal fun binanceDispatchStarted(raw: String?): Boolean? = when(raw) {
    "true" -> true
    "false" -> false
    else -> null
}
