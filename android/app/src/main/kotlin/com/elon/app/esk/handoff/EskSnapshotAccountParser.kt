package com.elon.app.esk.handoff

import com.elon.eskcontract.EskSnapshotContract
import com.google.gson.stream.JsonReader
import com.google.gson.stream.JsonToken
import java.io.StringReader
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction

/** Strict source-schema boundary; server text and account identity never leave this parser. */
internal object EskSnapshotAccountParser {
    const val MAX_BYTES = 16 * 1024
    private data class NumberToken(val value: String)
    private val rootKeys = setOf("schema", "mode", "enabled", "simulated", "funds_moved",
        "asset", "balance", "sellback", "status_message")
    private val assetKeys = setOf("asset_id", "symbol", "name", "decimals", "issuance_mode",
        "chain_status", "contract_address")
    private val amounts = linkedMapOf("total" to "total_base_units", "available" to "available_base_units",
        "reserved_for_sellback" to "sellback_reserved_base_units",
        "reserved_for_quant" to "quant_reserved_base_units", "reserved_total" to "reserved_base_units")

    fun parse(bytes: ByteArray): Map<String, String> {
        require(bytes.size in 1..MAX_BYTES)
        val text = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
        val root = JsonReader(StringReader(text)).use { reader ->
            reader.isLenient = false
            val value = readObject(reader, 0, intArrayOf(0))
            require(reader.peek() == JsonToken.END_DOCUMENT)
            value
        }
        require(root.keys == rootKeys && root["schema"] == "yilong.esk.asset_account.v2")
        val mode = root.string("mode")
        require(mode == "paper" || mode == "disabled")
        require(root["enabled"] == (mode == "paper"))
        require(root["simulated"] == true && root["funds_moved"] == false)
        root.string("status_message") // Never forward or display dynamic server messages.
        val asset = root.objectAt("asset")
        require(asset.keys == assetKeys && asset["asset_id"] == "esk" && asset["symbol"] == "ESK")
        asset.string("name")
        require(asset["decimals"] == NumberToken("6") && asset["issuance_mode"] == "paper_recorded")
        require(asset["chain_status"] == "not_deployed" && asset["contract_address"] == null)
        val balance = root.objectAt("balance")
        require(balance.keys == amounts.keys + amounts.values + setOf("revision", "updated_at"))
        require(balance["updated_at"] == null || balance["updated_at"] is String)
        val result = linkedMapOf("asset_id" to "esk", "symbol" to "ESK", "mode" to mode,
            "issuance_mode" to "paper_recorded", "chain_status" to "not_deployed",
            "simulated" to "true", "funds_moved" to "false")
        for ((amountKey, unitsKey) in amounts) {
            val amount = balance.string(amountKey)
            val units = EskSnapshotContract.units(amount) ?: error("Invalid amount")
            val rawUnits = balance.string(unitsKey)
            require(EskSnapshotContract.integer(rawUnits) != null && units.toString() == rawUnits)
            result[amountKey] = amount
        }
        require(EskSnapshotContract.validBalances(result))
        val revision = (balance["revision"] as? NumberToken)?.value ?: error("Invalid revision type")
        require(EskSnapshotContract.integer(revision) != null)
        result["revision"] = revision
        val sellback = root.objectAt("sellback")
        require(sellback.keys == setOf("application_only", "request_enabled", "settlement_enabled", "pricing_status"))
        require(sellback["application_only"] == true && sellback["settlement_enabled"] == false)
        require(sellback["pricing_status"] == "not_defined")
        require(sellback["request_enabled"] == (mode == "paper" &&
            EskSnapshotContract.units(result.getValue("available"))!!.signum() > 0))
        return result
    }

    private fun readObject(reader: JsonReader, depth: Int, count: IntArray): Map<String, Any?> {
        require(depth <= 6 && reader.peek() == JsonToken.BEGIN_OBJECT)
        reader.beginObject()
        val values = linkedMapOf<String, Any?>()
        while (reader.hasNext()) {
            require(++count[0] <= 128)
            val key = reader.nextName()
            require(key.length <= 64 && !values.containsKey(key))
            values[key] = when (reader.peek()) {
                JsonToken.STRING -> reader.nextString().also { require(it.length <= 2048) }
                JsonToken.NUMBER -> NumberToken(reader.nextString().also { require(it.length <= 32) })
                JsonToken.BOOLEAN -> reader.nextBoolean()
                JsonToken.NULL -> { reader.nextNull(); null }
                JsonToken.BEGIN_OBJECT -> readObject(reader, depth + 1, count)
                else -> error("Unsupported JSON type")
            }
        }
        reader.endObject()
        return values
    }

    private fun Map<String, Any?>.string(key: String): String = this[key] as? String ?: error("Expected string")

    @Suppress("UNCHECKED_CAST")
    private fun Map<String, Any?>.objectAt(key: String): Map<String, Any?> =
        this[key] as? Map<String, Any?> ?: error("Expected object")
}
