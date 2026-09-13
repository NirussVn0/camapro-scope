package app.camapro.scope.protocol

import org.json.JSONObject
import java.math.BigDecimal

/** Result of validating one control message against the frozen G1.2 envelope. */
sealed class ValidationResult {
    object Ok : ValidationResult()
    data class Reject(val reasonClass: String) : ValidationResult()
}

/**
 * Minimal envelope validation mirroring protocol/control-message.schema.json.
 * The Rust consumer (desktop/src-tauri) validates via the same JSON Schema file;
 * Kotlin re-encodes the discriminated rules here so JVM tests run the shared
 * corpus without a JSON Schema dependency.
 */
object EnvelopeValidator {

    private val requestTypes = setOf("hello", "camera.set", "session.start", "session.stop", "ping")
    private val eventTypes = setOf("capabilities", "camera.state")
    private val correlatedTypes = setOf("pong", "response")
    private val allTypes = requestTypes + eventTypes + correlatedTypes + setOf("error")

    private val knownControls = setOf(
        "aeEnabled", "exposureCompensationSteps", "iso", "shutterNanos", "focusDistanceDiopters",
    )
    private val knownTopLevel = setOf("v", "id", "type", "payload", "ok", "result", "error")
    private val errorCodes = setOf(
        "unsupported_version", "unauthenticated", "forbidden", "busy", "invalid_payload",
        "unsupported_control", "stale_capabilities", "invalid_state", "timeout",
        "permission_denied", "media_failure", "internal",
    )
    private val knownCameraStateKeys = setOf(
        "cameraId", "revision", "streamState", "applied",
    )

    private const val MAX_SAFE_ID = 9_007_199_254_740_991L

    fun validate(message: JSONObject): ValidationResult {
        val keys = message.keys().asSequence().toSet()
        keys.subtract(knownTopLevel).firstOrNull()?.let {
            return ValidationResult.Reject("unknown_top_level_field")
        }
        if (!keys.contains("v")) return ValidationResult.Reject("missing_required_field")
        if (message.getInt("v") != 1) return ValidationResult.Reject("unsupported_version")
        if (!keys.contains("type")) return ValidationResult.Reject("missing_required_field")
        val type = message.getString("type")
        if (type !in allTypes) return ValidationResult.Reject("unknown_type")

        val hasId = keys.contains("id")
        if (hasId) {
            when (val raw = message.opt("id")) {
                is Int, is Long -> Unit
                is BigDecimal ->
                    // org.json represents non-integral JSON numbers as BigDecimal.
                    return ValidationResult.Reject(
                        if (raw.stripTrailingZeros().scale() > 0) {
                            "id_not_safe_integer"
                        } else {
                            "id_not_integer"
                        },
                    )

                else -> return ValidationResult.Reject("id_not_integer")
            }
            val id = message.getLong("id")
            if (id < 0) return ValidationResult.Reject("id_out_of_range")
            if (id > MAX_SAFE_ID) return ValidationResult.Reject("id_out_of_range")
        }

        val requiresId = type in requestTypes || type in correlatedTypes
        if (requiresId && !hasId) return ValidationResult.Reject("request_requires_id")
        if (type in eventTypes && hasId) return ValidationResult.Reject("event_forbids_id")

        if (type == "error") {
            if (keys.contains("payload")) return ValidationResult.Reject("payload_must_be_object")
            val error = message.optJSONObject("error")
                ?: return ValidationResult.Reject("error_requires_code")
            if (!error.has("code")) return ValidationResult.Reject("error_requires_code")
            val code = error.getString("code")
            if (code !in errorCodes) return ValidationResult.Reject("unknown_error_code")
            return ValidationResult.Ok
        }

        if (type == "response") {
            val ok = message.opt("ok")
            if (ok !is Boolean) return ValidationResult.Reject("response_requires_ok")
            if (ok != true) return ValidationResult.Reject("response_conflicting_fields")
            if (!keys.contains("result")) return ValidationResult.Reject("response_requires_ok")
            if (keys.contains("error")) return ValidationResult.Reject("response_conflicting_fields")
            return ValidationResult.Ok
        }

        val payload = message.optJSONObject("payload")
            ?: return ValidationResult.Reject("payload_must_be_object")

        if (type == "camera.set") {
            if (!payload.has("cameraId") || !payload.has("capabilityRevision")) {
                return ValidationResult.Reject("payload_must_be_object")
            }
            val changes = payload.optJSONObject("changes")
                ?: return ValidationResult.Reject("payload_must_be_object")
            if (changes.length() == 0) return ValidationResult.Reject("changes_must_be_non_empty")
            changes.keys().asSequence().forEach { key ->
                if (key !in knownControls) return ValidationResult.Reject("unknown_control_key")
                if (key == "exposureCompensationSteps" &&
                    changes.get(key) !is Int &&
                    changes.get(key) !is Long
                ) {
                    return ValidationResult.Reject("ev_steps_must_be_integer")
                }
            }
            return ValidationResult.Ok
        }

        if (type == "camera.state") {
            payload.keys().asSequence().forEach { key ->
                if (key !in knownCameraStateKeys) return ValidationResult.Reject("payload_must_be_object")
            }
            return ValidationResult.Ok
        }

        if (type == "hello") {
            val client = payload.optJSONObject("client")
                ?: return ValidationResult.Reject("payload_must_be_object")
            if (payload.optString("majorVersion").isEmpty() &&
                !payload.has("majorVersion")
            ) {
                return ValidationResult.Reject("payload_must_be_object")
            }
            if (client.optString("name").isEmpty()) {
                return ValidationResult.Reject("payload_must_be_object")
            }
            return ValidationResult.Ok
        }

        return ValidationResult.Ok
    }
}
