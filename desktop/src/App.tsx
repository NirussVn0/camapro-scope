import { useState } from "react";

type SessionState = "Disconnected" | "Ready" | "Streaming" | "Error";

export function App() {
  const [sessionState, setSessionState] = useState<SessionState>("Ready");
  const [generation, setGeneration] = useState<number>(1);
  const [selectedMode] = useState<string>("720p30");

  const handleStart = () => {
    if (sessionState === "Ready") {
      setSessionState("Streaming");
    }
  };

  const handleStop = () => {
    if (sessionState === "Streaming") {
      setSessionState("Ready");
    }
  };

  const handleDisconnect = () => {
    setSessionState("Disconnected");
    setGeneration((g) => g + 1);
  };

  const handleReconnect = () => {
    // Invariant D04: Reconnect returns Ready, never silently reactivates Streaming
    setSessionState("Ready");
  };

  return (
    <main style={{ padding: "1.5rem", fontFamily: "sans-serif" }}>
      <header>
        <h1>Camapro Scope</h1>
        <p style={{ color: "#666" }}>
          Target: Linux Native Preview (Wayland) | Status: <strong>{sessionState}</strong> | Gen: {generation} | Mode: {selectedMode}
        </p>
      </header>

      <section style={{ margin: "1rem 0" }}>
        {/* Invariant D06: Native preview renders directly to container surface; never passing raw frames to React */}
        <div
          id="camapro-native-preview-surface"
          style={{
            width: "640px",
            height: "360px",
            backgroundColor: "#111",
            color: "#eee",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            borderRadius: "8px",
            border: "2px solid #333",
          }}
        >
          {sessionState === "Streaming" ? (
            <span>[Native Video Surface Active — GStreamer waylandsink]</span>
          ) : (
            <span>[Preview Inactive — {sessionState}]</span>
          )}
        </div>
      </section>

      <section style={{ display: "flex", gap: "0.5rem" }}>
        <button
          onClick={handleStart}
          disabled={sessionState !== "Ready"}
          style={{ padding: "0.5rem 1rem", cursor: sessionState === "Ready" ? "pointer" : "not-allowed" }}
        >
          Start Camera
        </button>

        <button
          onClick={handleStop}
          disabled={sessionState !== "Streaming"}
          style={{ padding: "0.5rem 1rem", cursor: sessionState === "Streaming" ? "pointer" : "not-allowed" }}
        >
          Stop Camera
        </button>

        {sessionState === "Disconnected" ? (
          <button onClick={handleReconnect} style={{ padding: "0.5rem 1rem", cursor: "pointer" }}>
            Reconnect Phone
          </button>
        ) : (
          <button onClick={handleDisconnect} style={{ padding: "0.5rem 1rem", cursor: "pointer" }}>
            Disconnect
          </button>
        )}
      </section>
    </main>
  );
}
