import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// T3: wire the real preview_start/stop/status commands (stream client ->
// waylandsink pipeline). Host stays 127.0.0.1: LAN discovery/pairing is G4.
export function usePreview() {
  const [active, setActive] = useState(false);
  const [frames, setFrames] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!active) return;
    const t = setInterval(async () => {
      try {
        setFrames(await invoke<{ frames: number }>("preview_status").then((s) => s.frames));
      } catch {
        /* window closing, etc. */
      }
    }, 1000);
    return () => clearInterval(t);
  }, [active]);

  const start = async (port: number, token: string) => {
    setBusy(true);
    setError(null);
    try {
      await invoke("preview_start", { host: "127.0.0.1", port, token });
      setActive(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    try {
      await invoke("preview_stop");
    } finally {
      setActive(false);
      setFrames(0);
      setBusy(false);
    }
  };

  return { active, frames, error, busy, start, stop };
}
