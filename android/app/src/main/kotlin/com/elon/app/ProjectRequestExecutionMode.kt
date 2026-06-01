package com.elon.app

internal enum class ProjectRequestExecutionMode(val wireValue: String) {
    Execute("execute"),
    Plan("plan");

    val isPlan: Boolean
        get() = this == Plan
}
