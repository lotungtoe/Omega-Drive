import { useState, useEffect } from 'react';
import { listen } from "@tauri-apps/api/event";
import { getConnectionStatus } from '../../diagnostics/services/diagnosticsService';

/**
 * useStatusMonitor Hook: Monitor connection status to Discord and Telegram.
 * Uses Event-driven mechanism (Tauri events) combined with periodic Polling.
 */
export function useStatusMonitor(isLite = false) {
  const [discordOnline, setDiscordOnline] = useState(true);
  const [telegramOnline, setTelegramOnline] = useState(false);

  useEffect(() => {
    if (isLite) return;

    // 1. Listen for events pushed from Backend (Instant feedback)
    let unlisten;
    const setupListener = async () => {
      unlisten = await listen("omega-event", (event) => {
        const payload = event.payload;
        // Handle variant DiscordConnectionStatusChanged(bool)
        if (payload?.type === "DiscordConnectionStatusChanged") {
          const isConnected = payload.data;
          setDiscordOnline(isConnected);
        }
        // Handle variant TelegramConnectionStatusChanged(bool)
        if (payload?.type === "TelegramConnectionStatusChanged") {
          const isConnected = payload.data;
          setTelegramOnline(isConnected);
        }
      });
    };
    setupListener();

    // 2. Periodic Polling (Fallback) - Increased interval to 10s to reduce load
    const check = async () => {
      try {
        const st = await getConnectionStatus();
        setDiscordOnline(st?.discord?.connected ?? false);
        setTelegramOnline(st?.telegram?.authorized ?? false);
      } catch (err) {
        console.warn("Failed to check connection status", err);
      }
    };
    
    check();

    return () => {
      if (unlisten) unlisten();
    };
  }, [isLite]);

  return { discordOnline, telegramOnline };
}
