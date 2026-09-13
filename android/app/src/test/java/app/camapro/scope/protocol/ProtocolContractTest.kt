package app.camapro.scope.protocol

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Runs the shared protocol/fixtures corpus against the frozen envelope rules.
 * Semantic vectors carry their own expected outcome; schema fixtures are
 * validated by [EnvelopeValidator] so both runtimes agree on rejection reasons.
 */
class ProtocolContractTest {

    private val fixturesRoot: File by lazy { locateFixtures() }

    private fun locateFixtures(): File {
        // Gradle runs JVM unit tests with the module directory as working
        // directory; direct JUnit runs may start from the repository root.
        val candidates = listOf(
            File("../.."),
            File("."),
            File("../../.."),
        ).map { File(it, "protocol/fixtures") }
        return candidates.firstOrNull { it.isDirectory }
            ?: throw AssertionError(
                "shared fixture corpus not found; tried: " +
                    candidates.joinToString { it.invariantSeparatorsPath },
            )
    }

    private fun json(file: File): JSONObject = JSONObject(file.readText())

    @Test
    fun `every valid fixture passes envelope validation`() {
        val files = File(fixturesRoot, "valid").listFiles { f -> f.extension == "json" }!!
        assertTrue("expected at least 10 valid fixtures", files.size >= 10)
        files.forEach { file ->
            val result = EnvelopeValidator.validate(json(file))
            assertTrue("${file.name}: $result", result is ValidationResult.Ok)
        }
    }

    @Test
    fun `every invalid fixture is rejected for its declared reason`() {
        val manifest = json(File(fixturesRoot, "manifest.json")).getJSONObject("fixtures")
        val files = File(fixturesRoot, "invalid").listFiles { f -> f.extension == "json" }!!
        assertTrue("expected at least 15 invalid fixtures", files.size >= 15)
        files.forEach { file ->
            val rel = "invalid/${file.name}"
            val declared = manifest.getJSONObject(rel).getString("reason")
            val result = EnvelopeValidator.validate(json(file))
            assertTrue("$rel must be rejected", result is ValidationResult.Reject)
            assertEquals(rel, declared, (result as ValidationResult.Reject).reasonClass)
        }
    }

    @Test
    fun `safe integer id bounds match other consumers`() {
        val maxSafe = 9_007_199_254_740_991L
        assertTrue(
            EnvelopeValidator.validate(
                JSONObject()
                    .put("v", 1)
                    .put("id", maxSafe)
                    .put("type", "ping")
                    .put("payload", JSONObject().put("sessionGeneration", 0)),
            ) is ValidationResult.Ok,
        )
        assertFalse(
            EnvelopeValidator.validate(
                JSONObject()
                    .put("v", 1)
                    .put("id", maxSafe + 1)
                    .put("type", "ping")
                    .put("payload", JSONObject().put("sessionGeneration", 0)),
            ) is ValidationResult.Ok,
        )
    }

    @Test
    fun `semantic vectors produce declared outcomes`() {
        val files = File(fixturesRoot, "semantic").listFiles { f -> f.extension == "json" }!!
        assertTrue("expected at least 8 semantic vectors", files.size >= 8)
        files.forEach { file ->
            val case = json(file)
            val expected = case.getString("expect")
            val actual = SemanticRules.decide(case)
            assertEquals("${file.name}: semantic outcome", expected, actual)
        }
    }

    @Test
    fun `units are exact integers across runtimes`() {
        val cameraSet = json(File(fixturesRoot, "valid/camera.set.json"))
        assertEquals(
            2L,
            cameraSet.getJSONObject("payload")
                .getJSONObject("changes")
                .getLong("exposureCompensationSteps"),
        )
        val capabilities = json(File(fixturesRoot, "valid/capabilities.json"))
        val cameras: JSONArray = capabilities.getJSONObject("payload").getJSONArray("cameras")
        assertEquals(
            33_333_333L,
            cameras.getJSONObject(0)
                .getJSONObject("controls")
                .getJSONObject("shutterNanos")
                .getLong("max"),
        )
    }
}
