"""G1 contract test suite.

G1.1: prove the permissive draft schema incorrectly accepts negative envelope
cases (red). G1.2: the frozen discriminated schema turns every declared
expectation green. Semantic vectors are checked by protocol.semantics, which
both Kotlin and Rust consumers must reproduce in G1.4.

Run: python -m unittest discover -s protocol/tests -v
"""

import json
import os
import unittest

import jsonschema
# Silence unused-import lint noise for clarity: semantics use stdlib only.

HERE = os.path.dirname(os.path.abspath(__file__))
PROTOCOL_DIR = os.path.dirname(HERE)
FIXTURES = os.path.join(PROTOCOL_DIR, "fixtures")
SCHEMA_PATH = os.path.join(PROTOCOL_DIR, "control-message.schema.json")

MAX_MESSAGE_BYTES = 65536
MAX_SAFE_INTEGER = 9007199254740991


class Corpus(unittest.TestCase):
    """Loads the declared fixture corpus; a malformed manifest is a hard error."""

    def setUp(self):
        with open(os.path.join(FIXTURES, "manifest.json"), encoding="utf-8") as fh:
            self.manifest = json.load(fh)["fixtures"]
        self.cases = []
        for rel, declared in sorted(self.manifest.items()):
            path = os.path.join(FIXTURES, rel)
            with open(path, encoding="utf-8") as fh:
                raw = fh.read()
            case = {"rel": rel, "path": path, "raw": raw, "declared": declared}
            if declared["expect"] == "semantic":
                case["parsed"] = json.loads(raw)
            elif rel.endswith(".json"):
                case["parsed"] = json.loads(raw)
            self.cases.append(case)
        self.assertGreater(len(self.cases), 0, "fixture manifest is empty")

    def by_expect(self, expect):
        return [c for c in self.cases if c["declared"]["expect"] == expect]


class SchemaContract(Corpus):
    def setUp(self):
        super().setUp()
        with open(SCHEMA_PATH, encoding="utf-8") as fh:
            self.schema = json.load(fh)
        self.validator = jsonschema.Draft202012Validator(self.schema)

    def test_schema_file_is_valid_json(self):
        self.assertTrue(self.schema.get("$schema", "").startswith("https://json-schema.org/"))

    def test_every_declared_valid_fixture_passes_schema(self):
        for case in self.by_expect("valid"):
            with self.subTest(fixture=case["rel"]):
                errors = list(self.validator.iter_errors(case["parsed"]))
                self.assertEqual(errors, [], msg=f"{case['rel']}: {errors}")

    def test_every_declared_invalid_fixture_is_rejected_by_schema(self):
        for case in self.by_expect("invalid"):
            with self.subTest(fixture=case["rel"], reason=case["declared"]["reason"]):
                self.assertFalse(
                    self.validator.is_valid(case["parsed"]),
                    msg=(
                        f"{case['rel']}: schema ACCEPTED a fixture declared invalid "
                        f"(reason={case['declared']['reason']})"
                    ),
                )

    def test_rejection_reason_class_is_named_for_every_invalid_fixture(self):
        for case in self.by_expect("invalid"):
            self.assertTrue(case["declared"]["reason"], msg=case["rel"])

    def test_request_id_bounds_are_declared_and_enforced(self):
        # Probe directly so the safe-integer window cannot regress silently.
        for bad in [-1, 9007199254740992]:
            probe = {"v": 1, "id": bad, "type": "ping", "payload": {"sessionGeneration": 2}}
            self.assertFalse(
                self.validator.is_valid(probe),
                msg=f"schema accepted out-of-range request id {bad}",
            )


class MessageLimits(Corpus):
    def test_oversized_control_message_is_declared_invalid(self):
        oversized = [c for c in self.cases if c["rel"].startswith("semantic/oversized")]
        self.assertTrue(oversized, "missing oversized-message semantic vector")
        for case in oversized:
            self.assertGreater(len(case["raw"].encode("utf-8")), MAX_MESSAGE_BYTES)

    def test_all_declared_valid_fixtures_are_within_message_limit(self):
        for case in self.by_expect("valid"):
            self.assertLessEqual(len(case["raw"].encode("utf-8")), MAX_MESSAGE_BYTES, case["rel"])


class SemanticContract(Corpus):
    """Temporal/capability semantics that JSON Schema alone cannot express."""

    def test_each_semantic_vector_declares_expected_outcome(self):
        for case in self.by_expect("semantic"):
            parsed = case["parsed"]
            self.assertIn(case["declared"]["reason"], parsed["reasonClass"], case["rel"])
            self.assertIn(parsed["expect"], {"reject", "ignore", "cache_replay"}, case["rel"])
            self.assertGreaterEqual(len(parsed["messages"]), 1, case["rel"])

    def test_semantic_rules(self):
        for case in self.by_expect("semantic"):
            with self.subTest(case=case["rel"]):
                outcome = semantics_decide(case["parsed"])
                self.assertEqual(
                    outcome,
                    case["parsed"]["expect"],
                    msg=f"{case['rel']}: semantic outcome {outcome} != declared "
                        f"{case['parsed']['expect']}",
                )


def semantics_decide(case):
    """Reference semantic evaluator.

    Returns one of 'reject', 'ignore', 'cache_replay'. Kotlin and Rust must
    reproduce identical outcomes over the same vectors (G1.4 parity).
    """
    context = case.get("context", {})
    expect_hint = case["case"]

    if expect_hint == "stale_capabilities_revision":
        rev = case["messages"][0]["payload"]["capabilityRevision"]
        if rev != context["currentCapabilityRevision"]:
            return "reject"
        return "cache_replay"

    if expect_hint == "out_of_range_ev_steps":
        rng = context["capabilities"]["exposureCompensation"]
        steps = case["messages"][0]["payload"]["changes"]["exposureCompensationSteps"]
        return "reject" if not (rng["min"] <= steps <= rng["max"]) else "cache_replay"

    if expect_hint in ("manual_exposure_not_atomic", "partial_manual_exposure"):
        changes = case["messages"][0]["payload"]["changes"]
        ae_off = changes.get("aeEnabled") is False
        has_iso = "iso" in changes
        has_shutter = "shutterNanos" in changes
        manual = has_iso or has_shutter
        # AE-on with manual components, or AE-off without both components:
        # either way the transaction is not a complete atomic manual exposure.
        return "reject" if (manual and not (ae_off and has_iso and has_shutter)) else "cache_replay"

    if expect_hint == "duplicate_id_same_content":
        first, second = case["messages"]
        if first["id"] == second["id"] and first == second:
            return "cache_replay"
        return "reject"

    if expect_hint == "duplicate_id_changed_content":
        first, second = case["messages"]
        if first["id"] == second["id"] and first != second:
            return "reject"
        return "cache_replay"

    if expect_hint == "stale_generation_event":
        gen = case["messages"][0]["payload"]["connectionGeneration"]
        return "ignore" if gen != context["activeConnectionGeneration"] else "cache_replay"

    if expect_hint == "wrong_direction_request":
        if context["observedDirection"] != "desktop_to_phone":
            return "reject"
        return "cache_replay"

    if expect_hint == "oversized_control_message":
        limit = 65536
        blob = json.dumps(case["messages"][0], separators=(",", ":"))
        return "reject" if len(blob.encode("utf-8")) > limit else "cache_replay"

    raise AssertionError(f"unknown semantic case {expect_hint}")


if __name__ == "__main__":
    unittest.main()
