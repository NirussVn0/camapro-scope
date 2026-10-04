import test from "node:test";
import assert from "node:assert/strict";
import { connectionLabel, endpointError, connectionError, parsePhoneEndpoint, canReplaceEndpoint, previewStatusState } from "./connection.ts";

const phone = { host: "192.168.1.20", port: 8100, token: "enrolled-token" };

test("endpoint replacement is refused throughout preview start, activity and stop", () => {
  assert.equal(canReplaceEndpoint({ active: false, busy: false }), true);
  for (const state of [{ active: false, busy: true }, { active: true, busy: false }, { active: true, busy: true }]) {
    assert.equal(canReplaceEndpoint(state), false);
  }
});

test("backend inactive status ends only local preview and permits an explicit new Connect", () => {
  assert.deepEqual(previewStatusState({ active: true, frames: 42 }), { active: true, frames: 42 });
  const ended = previewStatusState({ active: false, frames: 42 });
  assert.deepEqual(ended, { active: false, frames: 0 });
  assert.equal(connectionLabel(ended.active, phone, phone), "Paired / Ready");
  assert.equal(connectionLabel(ended.active, phone, null), "Not connected");
  assert.equal(canReplaceEndpoint({ ...ended, busy: false }), true);
});

test("Connect requires a nonblank phone endpoint, valid port and token", () => {
  assert.match(endpointError("   ", 8100, "token")!, /phone host/i);
  for (const port of [0, 65536, NaN, 1.5]) assert.match(endpointError(phone.host, port, phone.token)!, /port/i);
  assert.match(endpointError(phone.host, 8100, "  ")!, /Pair via QR/);
  assert.equal(endpointError(` ${phone.host} `, phone.port, phone.token), null);
});

test("pairing is Ready, never Connected until explicit preview start", () => {
  assert.equal(connectionLabel(false, phone, null), "Not connected");
  assert.equal(connectionLabel(false, phone, phone), "Paired / Ready");
  assert.equal(connectionLabel(true, phone, phone), "Connected");
  for (const changed of [{ ...phone, host: "127.0.0.1" }, { ...phone, port: 8101 }, { ...phone, token: "other" }]) {
    assert.equal(connectionLabel(false, changed, phone), "Not connected");
  }
});

test("phone URLs parse without guessing an address; desktop QR callback is not a phone endpoint", () => {
  assert.deepEqual(parsePhoneEndpoint("https://192.168.1.20:8100/stream?token=enrolled-token"), phone);
  assert.deepEqual(parsePhoneEndpoint("192.168.1.20:8100"), { ...phone, token: "" });
  assert.deepEqual(parsePhoneEndpoint(JSON.stringify({ ip: phone.host, port: phone.port, token: phone.token })), phone);
  for (const input of ["", "   ", "http://", "192.168.1.20:99999", '{"token":"secret"}']) {
    assert.throws(() => parsePhoneEndpoint(input));
  }
  assert.throws(() => parsePhoneEndpoint('{"endpoint_hint":"https://192.168.1.2:8100","secret":"qr-secret"}'), /Scan.*phone/);
});

test("backend enrollment and refused Debug errors become actionable messages", () => {
  for (const error of ['Tls("Phone endpoint is not enrolled; scan desktop QR first")', "Enrollment token mismatch"]) {
    assert.match(connectionError(error), /Pair via QR.*Scan.*Start.*Connect/);
  }
  assert.match(connectionError('Connect(Os { code: 111, kind: ConnectionRefused, message: "Connection refused" })'), /phone.*Start.*host.*port/i);
  assert.equal(connectionError(new Error("something internal")), "Connection failed. Check the enrolled phone endpoint and try again.");
});
