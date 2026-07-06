import { useEffect, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { reportUiVisibility } from "../../features/diagnostics/services/diagnosticsService";

/**
 * Hook tracking window state (Visible, Focused, Minimized).
 * Only sends info to Backend when state CHANGES to avoid request spam.
 */
export function useWindowStateTracker() {
  const lastState = useRef({ visible: null, focused: null });
  const windowRef = useRef(null);

  useEffect(() => {
    // Get window instance from Tauri
    try {
      windowRef.current = getCurrentWindow();
    } catch (e) {
      console.warn("[WindowStateTracker] Failed to get current window:", e);
    }
    
    const label = windowRef.current?.label || "main";

    const checkAndReport = async () => {
      // 1. Check Focused (Based on DOM focus)
      const focused = document.hasFocus();
      
      // 2. Check Visibility (Based on Tab Visibility API)
      const isHidden = document.visibilityState === "hidden";
      
      // 3. Check Minimized (Based on Tauri API)
      // Use promise chain instead of try/catch to avoid empty catch block
      let isMinimized = false;
      if (windowRef.current) {
        isMinimized = await windowRef.current.isMinimized().catch(() => false);
      }

      // A window is considered "Visible" to the user if it is not tab-hidden and not minimized
      const visible = !isHidden && !isMinimized;

      // ONLY send request if there's an actual change from previous state
      if (visible !== lastState.current.visible || focused !== lastState.current.focused) {
        lastState.current = { visible, focused };
        
        reportUiVisibility({ 
          windowLabel: label, 
          visible, 
          focused 
        }).catch(() => {
          // Silent fail to avoid polluting user console/file logs on temporary disconnect
        });
      }
    };

    // Register event listeners
    const handleVisibilityChange = () => checkAndReport();
    const handleFocus = () => checkAndReport();
    const handleBlur = () => checkAndReport();
    const handleResize = () => checkAndReport(); // Resize event often fires on Minimize/Restore on Windows

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", handleFocus);
    window.addEventListener("blur", handleBlur);
    window.addEventListener("resize", handleResize);

    // Send initial state report on mount
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
