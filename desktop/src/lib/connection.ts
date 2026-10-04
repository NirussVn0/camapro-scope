export type PhoneEndpoint = { host: string; port: number; token: string };

export function canReplaceEndpoint(state: { active: boolean; busy: boolean }): boolean {
  return !state.active && !state.busy;
}

export function previewStatusState(status: { active: boolean; frames: number }) {
  return { active: status.active, frames: status.active ? status.frames : 0 };
}

export function endpointError(host: string, port: number, token?: string): string | null {
  if (!host.trim()) return "Enter the phone host, or Pair via QR to fill its enrolled endpoint.";
  if (!Number.isInteger(port) || port < 1 || port > 65535) return "Enter a port between 1 and 65535.";
  if (token !== undefined && !token.trim()) return "Pair via QR to enroll the phone and obtain its stream token.";
  return null;
}

export function connectionLabel(active: boolean, endpoint: PhoneEndpoint, paired: PhoneEndpoint | null) {
  if (active) return "Connected";
  return paired && endpoint.host.trim() === paired.host && endpoint.port === paired.port && endpoint.token.trim() === paired.token
    ? "Paired / Ready"
    : "Not connected";
}

export function parsePhoneEndpoint(input: string): PhoneEndpoint {
  const trimmed = input.trim();
  if (!trimmed) throw new Error("Paste a phone URL or phone IP:port.");
  let host: string;
  let port: number;
  let token: string;
  if (trimmed.startsWith("{")) {
    const parsed = JSON.parse(trimmed);
    if (parsed.endpoint_hint || parsed.secret) throw new Error("Scan the desktop QR on your phone; its callback address is not the phone host.");
    host = typeof parsed.ip === "string" ? parsed.ip.trim() : "";
    port = Number(parsed.port ?? 8100);
    token = typeof parsed.token === "string" ? parsed.token : "";
  } else {
    const url = new URL(trimmed.includes("://") ? trimmed : `https://${trimmed}`);
    if (!["http:", "https:"].includes(url.protocol) || url.username || url.password) throw new Error("Paste a phone HTTP(S) URL or phone IP:port.");
    host = url.hostname;
    port = Number(url.port || 8100);
    token = url.searchParams.get("token") ?? "";
  }
  const error = endpointError(host, port);
  if (error) throw new Error(error);
  return { host, port, token };
}

export function connectionError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (/not enrolled|pairing required|enrollment|unpaired/i.test(message)) {
    return "Pairing required for this endpoint. Choose Pair via QR, Scan on the phone, then tap Start on the phone. Once Paired / Ready appears, click Connect.";
  }
  if (/connection.?refused|os error 111|code: 111/i.test(message)) {
    return "Connection refused by the phone. Tap Start on the phone, check its host and port, and confirm both devices are on the same LAN. For USB, check ADB forwarding.";
  }
  if (/timed?.?out|unreachable|no route/i.test(message)) return "Phone unreachable. Check the phone is running, both devices share a LAN, and VPN or firewall settings allow local traffic.";
  if (/certificate|tls|pin mismatch/i.test(message)) return "Secure connection failed. Pair via QR again to enroll the phone's current endpoint.";
  return "Connection failed. Check the enrolled phone endpoint and try again.";
}
