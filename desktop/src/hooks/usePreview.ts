import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { canReplaceEndpoint, connectionError, endpointError, previewStatusState } from "../lib/connection";

// T3: wire the real preview_start/stop/status commands (stream client ->
// waylandsink pipeline). Only explicit Connect starts preview.
export function usePreview() {
  const [active, setActiveState] = useState(false);
  const [frames, setFrames] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusyState] = useState(false);
  // Synchronous guards also protect event/clipboard callbacks before React renders.
  const state = useRef({ active: false, busy: false });
  const generation = useRef(0);
  const canEditEndpoint = useCallback(() => canReplaceEndpoint(state.current), []);
  const setActive = (value: boolean) => {
    state.current.active = value;
    setActiveState(value);
  };
  const setBusy = (value: boolean) => {
    state.current.busy = value;
    setBusyState(value);
  };

  // Teardown on unmount: stop the stream + gst pipeline, no orphans.
  useEffect(
    () => () => {
      void invoke("preview_stop").catch(() => {});
    },
    [],
  );

  useEffect(() => {
    if (!active) return;
    let cancelled = false;
    let polling = false;
    const t = setInterval(async () => {
      if (polling || !state.current.active || state.current.busy) return;
      polling = true;
      const requestedGeneration = generation.current;
      try {
        const status = previewStatusState(await invoke<{ active: boolean; frames: number }>("preview_status"));
        if (cancelled || requestedGeneration !== generation.current) return;
        setFrames(status.frames);
        setActive(status.active);
      } catch {
        /* window closing, etc. */
      } finally {
        polling = false;
      }
    }, 1000);
    return () => {
      cancelled = true;
      clearInterval(t);
    };
  }, [active]);

  const start = async (host: string, port: number, token: string) => {
    if (!canEditEndpoint()) return;
    const invalid = endpointError(host, port, token);
    if (invalid) {
      setError(invalid);
      return;
    }
    setBusy(true);
    generation.current += 1;
    setError(null);
    try {
      await invoke("preview_start", {
        host: host.trim(),
        port,
        token: token.trim(),
      });
      setActive(true);
    } catch (e) {
      setError(connectionError(e));
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    if (state.current.busy) return;
    setBusy(true);
    generation.current += 1;
    try {
      await invoke("preview_stop");
    } finally {
      setActive(false);
      setFrames(0);
      setBusy(false);
    }
  };

  return { active, frames, error, busy, start, stop, canEditEndpoint };
}
