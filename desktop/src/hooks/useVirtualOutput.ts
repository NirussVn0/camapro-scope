import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Rust serializes OutputState as "Idle" | "Running" | "Stopped".
export type VirtualOutputState = "Idle" | "Running" | "Stopped";

// G3.2: the rest of the app fakes session state in React (useSession), so this
// is the first real Tauri invoke path. Wraps virtual_output_start/stop/status;
// commands error gracefully (e.g. browser dev, missing /dev/video0) and the
// message surfaces in the UI instead of throwing.
export function useVirtualOutput() {
  const [outputState, setOutputState] = useState<VirtualOutputState>("Idle");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const run = async (cmd: "virtual_output_start" | "virtual_output_stop") => {
    setBusy(true);
    setError(null);
    try {
      setOutputState(await invoke<VirtualOutputState>(cmd));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return {
    outputState,
    error,
    busy,
    toggle: () =>
      run(outputState === "Running" ? "virtual_output_stop" : "virtual_output_start"),
  };
}
