import { useState, useEffect } from 'react';
import { listen } from "@tauri-apps/api/event";
import { getConnectionStatus } from '../../diagnostics/services/diagnosticsService';

/**
 * useStatusMonitor Hook: Giám sát trạng thái kết nối tới Discord và Telegram.
 * Sử dụng cơ chế Event-driven (Tauri events) kết hợp Polling định kỳ.
 */
export function useStatusMonitor(isLite = false) {
  const [discordOnline, setDiscordOnline] = useState(true);
  const [telegramOnline, setTelegramOnline] = useState(false);

  useEffect(() => {
    if (isLite) return;

    // 1. Lắng nghe sự kiện chủ động đẩy từ Backend (Instant feedback)
    let unlisten;
    const setupListener = async () => {
      unlisten = await listen("omega-event", (event) => {
        const payload = event.payload;
        // Xử lý variant DiscordConnectionStatusChanged(bool)
        if (payload?.type === "DiscordConnectionStatusChanged") {
          const isConnected = payload.data;
          setDiscordOnline(isConnected);
        }
        // Xử lý variant TelegramConnectionStatusChanged(bool)
        if (payload?.type === "TelegramConnectionStatusChanged") {
          const isConnected = payload.data;
          setTelegramOnline(isConnected);
        }
      });
    };
    setupListener();

    // 2. Polling định kỳ (Fallback) - Tăng interval lên 10s để giảm tải
    const check = async () => {
      try {
        const st = await getConnectionStatus();
        setDiscordOnline(st?.discord?.connected ?? false);
        setTelegramOnline(st?.telegram?.authorized ?? false);
      } catch (err) {
        console.warn("Kiểm tra trạng thái kết nối thất bại", err);
      }
    };
    
    check();

    return () => {
      if (unlisten) unlisten();
    };
  }, [isLite]);

  return { discordOnline, telegramOnline };
}
