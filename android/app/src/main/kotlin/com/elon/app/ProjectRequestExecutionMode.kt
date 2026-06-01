package com.elon.app

enum class ProjectRequestExecutionMode(val wireValue: String) {
    Execute("execute"),
    Plan("plan");

    val isPlan: Boolean
        get() = this == Plan
}
