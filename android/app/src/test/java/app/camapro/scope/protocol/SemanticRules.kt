package app.camapro.scope.protocol

import org.json.JSONArray
import org.json.JSONObject

/**
 * Reference semantic evaluator for protocol/fixtures/semantic vectors.
 * Mirrors semantics_decide() in protocol/tests/test_contract.py and
 * semantic_expect() in desktop/src-tauri/tests/protocol_contract.rs.
 */
object SemanticRules {

    fun decide(case: JSONObject): String {
        val name = case.getString("case")
        val context = case.optJSONObject("context") ?: JSONObject()
        val messages = case.getJSONArray("messages")
        return when (name) {
            "stale_capabilities_revision" -> {
                val rev = messages.getJSONObject(0).getJSONObject("payload").getLong("capabilityRevision")
                val current = context.getLong("currentCapabilityRevision")
                if (rev == current) "cache_replay" else "reject"
            }

            "out_of_range_ev_steps" -> {
                val range = context.getJSONObject("capabilities")
                    .getJSONObject("exposureCompensation")
                val steps = messages.getJSONObject(0).getJSONObject("payload")
                    .getJSONObject("changes").getLong("exposureCompensationSteps")
                if (steps in range.getLong("min")..range.getLong("max")) "cache_replay" else "reject"
            }

            "manual_exposure_not_atomic", "partial_manual_exposure" -> {
                val changes = messages.getJSONObject(0).getJSONObject("payload")
                    .getJSONObject("changes")
                val aeOff = changes.has("aeEnabled") && !changes.getBoolean("aeEnabled")
                val hasIso = changes.has("iso")
                val hasShutter = changes.has("shutterNanos")
                val manualTouched = hasIso || hasShutter
                if (manualTouched && !(aeOff && hasIso && hasShutter)) "reject" else "cache_replay"
            }

            "duplicate_id_same_content", "duplicate_id_changed_content" -> {
                val first = messages.getJSONObject(0)
                val second = messages.getJSONObject(1)
                when {
                    first.getLong("id") != second.getLong("id") -> "cache_replay"
                    canonical(first) == canonical(second) -> "cache_replay"
                    else -> "reject"
                }
            }

            "stale_generation_event" -> {
                val gen = messages.getJSONObject(0).getJSONObject("payload")
                    .getLong("connectionGeneration")
                val active = context.getLong("activeConnectionGeneration")
                if (gen == active) "cache_replay" else "ignore"
            }

            "wrong_direction_request" -> {
                if (context.optString("observedDirection") == "desktop_to_phone") {
                    "cache_replay"
                } else {
                    "reject"
                }
            }

            "oversized_control_message" -> {
                val blob = canonical(messages.getJSONObject(0)).toByteArray(Charsets.UTF_8)
                if (blob.size > MAX_CONTROL_MESSAGE_BYTES) "reject" else "cache_replay"
            }

            else -> error("unknown semantic case: $name")
        }
    }

    /** Deterministic serialization so duplicate detection is content-exact. */
    private fun canonical(value: Any): String = when (value) {
        is JSONObject ->
            value.keys().asSequence().sorted()
                .joinToString(",", "{", "}") { "\"$it\":${canonical(value.get(it))}" }

        is JSONArray ->
            (0 until value.length()).joinToString(",", "[", "]") { canonical(value.get(it)) }

        is String -> "\"$value\""
        else -> value.toString()
    }

    const val MAX_CONTROL_MESSAGE_BYTES = 65_536
}
