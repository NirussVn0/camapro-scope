# Camapro Scope control protocol — G1 contract

## Status

`control-message.schema.json` is the **frozen G1.2** discriminated-variant schema
for wire protocol v1. Directions, ID requirements, payload shapes, units and the
error taxonomy follow [docs/PROTOCOL.md](../docs/PROTOCOL.md). Device capability
ranges, duplicate-request caching, connection generations and lease timing are
semantic rules each runtime must implement identically; JSON Schema alone does
not express them.

## Running the contract suite

From repository root, in an isolated environment:

```bash
python -m venv protocol/.venv
protocol/.venv/bin/pip install -r protocol/requirements-test.txt
python -m unittest discover -s protocol/tests -v
```

Exit status 0 means every valid fixture validates, every invalid fixture is
rejected for its declared reason, and every semantic vector matches its
declared behavioral outcome under the reference evaluator in
`tests/test_contract.py`.

## Layout

| Path | Purpose |
|---|---|
| `control-message.schema.json` | Frozen message schema (draft 2020-12) |
| `fixtures/valid/` | One well-formed message per type |
| `fixtures/invalid/` | Envelope negatives that must be rejected |
| `fixtures/semantic/` | Temporal/capability vectors with declared outcomes |
| `fixtures/manifest.json` | Declared expectation for every fixture |
| `tests/test_contract.py` | Cross-language reference semantics + gates |
| `requirements-test.txt` | Pinned validator (`jsonschema==4.26.0`) |

Consumers (Android Kotlin, desktop Rust) run the same corpus in G1.4; a change
to schema or fixtures requires updating both consumers in the same slice.
