import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AiHealthStatus } from "../App";

const POLL_INTERVAL_MS = 30_000;

export function useAiHealth() {
  const [aiHealth, setAiHealth] = useState<AiHealthStatus | null>(null);
  const cancelledRef = useRef(false);

  useEffect(() => {
    cancelledRef.current = false;
    const check = () => {
      invoke<AiHealthStatus>("check_ai_availability")
        .then((status) => {
          if (!cancelledRef.current) setAiHealth(status);
        })
        .catch(console.error);
    };
    check();
    const interval = setInterval(check, POLL_INTERVAL_MS);
    return () => {
      cancelledRef.current = true;
      clearInterval(interval);
    };
  }, []);

  return { aiHealth, setAiHealth };
}
