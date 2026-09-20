package com.elon.app.chatgptweb

import java.util.UUID

/** Both native and page bridge validators require this command correlation format. */
internal object GroupWebAiCommandIds {
    fun next(): String = "mcp_" + UUID.randomUUID().toString().replace("-", "")
}
