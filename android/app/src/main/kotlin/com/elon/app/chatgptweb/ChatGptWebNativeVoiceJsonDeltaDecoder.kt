package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

/** Reconstructs the bounded JSON delta stream used by ChatGPT Web voice messages. */
internal class ChatGptWebNativeVoiceJsonDeltaDecoder {
    private data class Delta(
        val channel: Int,
        val path: String,
        val operation: String,
        val value: Any?,
        val hasValue: Boolean,
    )

    private val valuesByChannel = LinkedHashMap<Int, Any?>()
    private var previous = Delta(
        channel = 0,
        path = "",
        operation = OP_ADD,
        value = null,
        hasValue = false,
    )

    fun apply(encoded: JSONObject): Any? {
        val decoded = decode(encoded) ?: return null
        previous = decoded
        if (decoded.channel !in 0 until MAX_CHANNELS) return null
        val holder = JSONObject().put(ROOT_KEY, copyJson(valuesByChannel[decoded.channel]))
        if (!applyAtPath(holder, decoded.path, decoded)) return null
        val value = holder.opt(ROOT_KEY).takeUnless { it === JSONObject.NULL }
        if (!withinResultLimit(value)) return null
        valuesByChannel[decoded.channel] = copyJson(value)
        return copyJson(value)
    }

    fun reset() {
        valuesByChannel.clear()
        previous = Delta(0, "", OP_ADD, null, false)
    }

    private fun decode(value: JSONObject): Delta? {
        if (value.length() > MAX_DELTA_KEYS) return null
        val channel = integerField(value, "channel", "c") ?: previous.channel
        val path = stringField(value, "path", "p") ?: previous.path
        val operation = stringField(value, "op", "o") ?: previous.operation
        if (path.length > MAX_PATH_CHARS || operation !in OPERATIONS) return null
        val valueKey = when {
            value.has("value") -> "value"
            value.has("v") -> "v"
            else -> null
        }
        return Delta(
            channel = channel,
            path = path,
            operation = operation,
            value = valueKey?.let(value::opt) ?: previous.value,
            hasValue = valueKey != null || previous.hasValue,
        )
    }

    private fun applyAtPath(holder: JSONObject, path: String, delta: Delta): Boolean {
        val segments = parsePath(path) ?: return false
        var parent: Any = holder
        for (index in 0 until segments.lastIndex) {
            val segment = segments[index]
            val next = segments[index + 1]
            parent = childOrCreate(parent, segment, next is Int) ?: return false
        }
        return applyOperation(parent, segments.last(), delta)
    }

    private fun applyOperation(parent: Any, key: Any, delta: Delta): Boolean = when (delta.operation) {
        OP_PATCH -> applyPatch(parent, key, delta.value)
        OP_ADD -> setValue(parent, key, delta.value, insertArray = true)
        OP_REMOVE -> removeValue(parent, key)
        OP_REPLACE -> setValue(parent, key, delta.value, insertArray = false)
        OP_APPEND -> appendValue(parent, key, delta.value)
        OP_TRUNCATE -> truncateValue(parent, key, delta.value)
        else -> false
    }

    private fun applyPatch(parent: Any, key: Any, value: Any?): Boolean {
        val patches = value as? JSONArray ?: return false
        if (patches.length() > MAX_PATCH_OPERATIONS) return false
        val target = getValue(parent, key)
        val holder = JSONObject().put(ROOT_KEY, copyJson(target))
        for (index in 0 until patches.length()) {
            val patch = patches.optJSONObject(index) ?: return false
            val decoded = decodeStandalone(patch) ?: return false
            if (!applyAtPath(holder, decoded.path, decoded)) return false
        }
        return setValue(parent, key, holder.opt(ROOT_KEY), insertArray = false)
    }

    private fun decodeStandalone(value: JSONObject): Delta? {
        val channel = integerField(value, "channel", "c") ?: 0
        val path = stringField(value, "path", "p") ?: return null
        val operation = stringField(value, "op", "o") ?: return null
        val valueKey = when {
            value.has("value") -> "value"
            value.has("v") -> "v"
            else -> null
        }
        if (path.length > MAX_PATH_CHARS || operation !in OPERATIONS) return null
        return Delta(channel, path, operation, valueKey?.let(value::opt), valueKey != null)
    }

    private fun appendValue(parent: Any, key: Any, value: Any?): Boolean {
        val current = getValue(parent, key)
        val next = when {
            current is String && value is String -> current + value
            current is JSONArray -> JSONArray(current.toString()).also { array ->
                when (value) {
                    is JSONArray -> repeat(value.length()) { array.put(copyJson(value.opt(it))) }
                    else -> array.put(copyJson(value))
                }
            }
            current is JSONObject && value is JSONObject -> JSONObject(current.toString()).also { objectValue ->
                value.keys().asSequence().take(MAX_OBJECT_KEYS).forEach { field ->
                    objectValue.put(field, copyJson(value.opt(field)))
                }
            }
            else -> value
        }
        return setValue(parent, key, next, insertArray = false)
    }

    private fun truncateValue(parent: Any, key: Any, value: Any?): Boolean {
        val size = (value as? Number)?.toInt()?.takeIf { it in 0..MAX_COLLECTION_ITEMS }
            ?: return false
        val current = getValue(parent, key)
        val next = when (current) {
            is String -> current.take(size)
            is JSONArray -> JSONArray().also { result ->
                repeat(minOf(size, current.length())) { result.put(copyJson(current.opt(it))) }
            }
            else -> return true
        }
        return setValue(parent, key, next, insertArray = false)
    }

    private fun childOrCreate(parent: Any, key: Any, nextIsIndex: Boolean): Any? {
        val current = getValue(parent, key)
        if (current is JSONObject || current is JSONArray) return current
        val created: Any = if (nextIsIndex) JSONArray() else JSONObject()
        return created.takeIf { setValue(parent, key, it, insertArray = false) }
    }

    private fun getValue(parent: Any, key: Any): Any? = when {
        parent is JSONObject && key is String -> parent.opt(key).takeUnless { it === JSONObject.NULL }
        parent is JSONArray && key is Int && key in 0 until parent.length() ->
            parent.opt(key).takeUnless { it === JSONObject.NULL }
        else -> null
    }

    private fun setValue(parent: Any, key: Any, value: Any?, insertArray: Boolean): Boolean = runCatching {
        when {
            parent is JSONObject && key is String -> parent.put(key, copyJson(value))
            parent is JSONArray && key is Int && key in 0..MAX_COLLECTION_ITEMS -> {
                if (insertArray && key < parent.length()) {
                    val tail = (key until parent.length()).map(parent::opt)
                    while (parent.length() > key) parent.remove(parent.length() - 1)
                    parent.put(copyJson(value))
                    tail.forEach { parent.put(copyJson(it)) }
                } else {
                    while (parent.length() < key) parent.put(JSONObject.NULL)
                    parent.put(key, copyJson(value))
                }
            }
            else -> return false
        }
        true
    }.getOrDefault(false)

    private fun removeValue(parent: Any, key: Any): Boolean = when {
        parent is JSONObject && key is String -> {
            parent.remove(key)
            true
        }
        parent is JSONArray && key is Int && key in 0 until parent.length() -> {
            parent.remove(key)
            true
        }
        else -> false
    }

    private fun parsePath(path: String): List<Any>? {
        if (path.isEmpty()) return listOf(ROOT_KEY)
        val normalized = path.removePrefix("/")
        val raw = normalized.split('/')
        if (raw.size > MAX_PATH_DEPTH) return null
        return buildList {
            add(ROOT_KEY)
            raw.forEach { segment ->
                val decoded = segment.replace("~1", "/").replace("~0", "~")
                add(decoded.toIntOrNull()?.takeIf { decoded == it.toString() } ?: decoded)
            }
        }
    }

    private fun integerField(value: JSONObject, longKey: String, shortKey: String): Int? =
        sequenceOf(longKey, shortKey)
            .firstOrNull(value::has)
            ?.let(value::opt)
            ?.let { it as? Number }
            ?.toInt()

    private fun stringField(value: JSONObject, longKey: String, shortKey: String): String? =
        sequenceOf(longKey, shortKey)
            .firstOrNull(value::has)
            ?.let(value::opt)
            ?.let { it as? String }

    private fun withinResultLimit(value: Any?): Boolean = runCatching {
        when (value) {
            is JSONObject, is JSONArray -> value.toString().length <= MAX_RESULT_CHARS
            is String -> value.length <= MAX_RESULT_CHARS
            else -> true
        }
    }.getOrDefault(false)

    private fun copyJson(value: Any?): Any? = when (value) {
        null, JSONObject.NULL -> JSONObject.NULL
        is JSONObject -> JSONObject(value.toString())
        is JSONArray -> JSONArray(value.toString())
        else -> value
    }

    private companion object {
        const val ROOT_KEY = "__root"
        const val OP_PATCH = "patch"
        const val OP_ADD = "add"
        const val OP_REMOVE = "remove"
        const val OP_REPLACE = "replace"
        const val OP_APPEND = "append"
        const val OP_TRUNCATE = "truncate"
        val OPERATIONS = setOf(OP_PATCH, OP_ADD, OP_REMOVE, OP_REPLACE, OP_APPEND, OP_TRUNCATE)
        const val MAX_CHANNELS = 16
        const val MAX_DELTA_KEYS = 8
        const val MAX_PATH_CHARS = 512
        const val MAX_PATH_DEPTH = 32
        const val MAX_PATCH_OPERATIONS = 64
        const val MAX_COLLECTION_ITEMS = 4096
        const val MAX_OBJECT_KEYS = 256
        const val MAX_RESULT_CHARS = 128 * 1024
    }
}
