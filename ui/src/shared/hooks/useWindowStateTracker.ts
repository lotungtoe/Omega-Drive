import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { reportUiVisibility } from "../../features/diagnostics/services/diagnosticsService";

/**
 * Hook theo dõi trạng thái của cửa sổ (Visible, Focused, Minimized).
 * Chỉ gửi thông tin cho Backend khi trạng thái THAY ĐỔI để tránh spam request.
 */
export function useWindowStateTracker() {
  const lastState = useRef({ visible: null, focused: null });
  const windowRef = useRef(null);

  useEffect(() => {
    // Lấy window instance từ Tauri
    try {
      windowRef.current = getCurrentWindow();
    } catch (e) {
      console.warn("[WindowStateTracker] Failed to get current window:", e);
    }
    
    const label = windowRef.current?.label || "main";

    const checkAndReport = async () => {
      // 1. Kiểm tra Focused (Dựa trên DOM focus)
      const focused = document.hasFocus();
      
      // 2. Kiểm tra Visibility (Dựa trên Tab Visibility API)
      const isHidden = document.visibilityState === "hidden";
      
      // 3. Kiểm tra Minimized (Dựa trên Tauri API)
      // Dùng promise chain thay vì try/catch để tránh empty catch block
      let isMinimized = false;
      if (windowRef.current) {
        isMinimized = await windowRef.current.isMinimized().catch(() => false);
      }

      // Một cửa sổ được coi là "Visible" đối với người dùng nếu nó không bị ẩn tab và không bị thu nhỏ
      const visible = !isHidden && !isMinimized;

      // CHỈ gửi request nếu có sự thay đổi thực sự so với trạng thái trước đó
      if (visible !== lastState.current.visible || focused !== lastState.current.focused) {
        lastState.current = { visible, focused };
        
        reportUiVisibility({ 
          windowLabel: label, 
          visible, 
          focused 
        }).catch(() => {
          // Silent fail để tránh làm bẩn log console/file của user khi mất kết nối tạm thời
        });
      }
    };

    // Đăng ký các event listeners
    const handleVisibilityChange = () => checkAndReport();
    const handleFocus = () => checkAndReport();
    const handleBlur = () => checkAndReport();
    const handleResize = () => checkAndReport(); // Resize event thường bắn khi Minimize/Restore trên Windows

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);
    window.addEventListener("resize", handleResize);

    // Gửi báo cáo trạng thái ban đầu ngay khi mount
    checkAndReport();

    return () => {
      // Cleanup listeners
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", handleFocus);
      window.removeEventListener("blur", handleBlur);
      window.removeEventListener("resize", handleResize);
    };
  }, []);
}
