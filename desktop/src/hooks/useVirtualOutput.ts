import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

// Rust serializes OutputState as "Idle" | "Running" | "Stopped".
export type VirtualOutputState = "Idle" | "Running" | "Stopped";

export interface VideoDeviceInfo {
  path: string;
  name: string;
  is_loopback: boolean;
}

export function useVirtualOutput() {
  const [outputState, setOutputState] = useState<VirtualOutputState>("Idle");
  const [devices, setDevices] = useState<VideoDeviceInfo[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refreshDevices = async () => {
    try {
      const devs = await invoke<VideoDeviceInfo[]>("get_video_devices");
      setDevices(devs);
      if (devs.length > 0 && !selectedDevice) {
        setSelectedDevice(devs[0].path);
      }
    } catch (e) {
      console.error("Failed to load video devices:", e);
    }
  };

  useEffect(() => {
    void refreshDevices();
  }, []);

  const run = async (cmd: "virtual_output_start" | "virtual_output_stop", devicePath?: string) => {
    setBusy(true);
    setError(null);
    try {
      if (cmd === "virtual_output_start") {
        const path = devicePath || selectedDevice || null;
        setOutputState(await invoke<VirtualOutputState>(cmd, { devicePath: path }));
      } else {
        setOutputState(await invoke<VirtualOutputState>(cmd));
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return {
    outputState,
    devices,
    selectedDevice,
    setSelectedDevice,
    refreshDevices,
    error,
    busy,
    toggle: (devicePath?: string) =>
      run(
        outputState === "Running" ? "virtual_output_stop" : "virtual_output_start",
        devicePath || selectedDevice,
      ),
  };
}
