import { useState } from "react";

export type SessionState = "Disconnected" | "Ready" | "Streaming" | "Error";

// Pure transitions, exported for testing. Invariant D04: reconnect lands on Ready, never Streaming.
export const transitions = {
  start: (s: SessionState): SessionState => (s === "Ready" ? "Streaming" : s),
  stop: (s: SessionState): SessionState => (s === "Streaming" ? "Ready" : s),
  disconnect: (_s: SessionState): SessionState => "Disconnected",
  reconnect: (_s: SessionState): SessionState => "Ready",
};

export function useSession() {
  const [sessionState, setSessionState] = useState<SessionState>("Ready");
  const [generation, setGeneration] = useState<number>(1);
  const [selectedMode] = useState<string>("720p30");

  const advance = (t: (s: SessionState) => SessionState) => setSessionState((s) => t(s));

  return {
    sessionState,
    generation,
    selectedMode,
    handleStart: () => advance(transitions.start),
    handleStop: () => advance(transitions.stop),
    // Each new connection is a new session generation; stale async completions must be rejected.
    handleDisconnect: () => {
      advance(transitions.disconnect);
      setGeneration((g) => g + 1);
    },
    handleReconnect: () => {
      advance(transitions.reconnect);
      setGeneration((g) => g + 1);
    },
  };
}
