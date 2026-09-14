// Runnable check for the session state machine (node --test). Imports the real
// transitions from useSession.ts so regressions in the hook's logic fail here.
import test from "node:test";
import assert from "node:assert/strict";
import { transitions, type SessionState } from "./useSession.ts";

const all: SessionState[] = ["Disconnected", "Ready", "Streaming", "Error"];

test("start only from Ready", () => {
  assert.equal(transitions.start("Ready"), "Streaming");
  for (const s of all.filter((s) => s !== "Ready")) assert.equal(transitions.start(s), s);
});

test("stop only from Streaming", () => {
  assert.equal(transitions.stop("Streaming"), "Ready");
  for (const s of all.filter((s) => s !== "Streaming")) assert.equal(transitions.stop(s), s);
});

test("disconnect lands on Disconnected from any state", () => {
  for (const s of all) assert.equal(transitions.disconnect(s), "Disconnected");
});

test("invariant D04: reconnect returns Ready, never Streaming, from any state", () => {
  for (const s of all) assert.equal(transitions.reconnect(s), "Ready");
});
