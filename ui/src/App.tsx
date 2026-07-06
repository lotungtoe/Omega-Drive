import { useEffect, Suspense, lazy } from "react";
import MainApp from "./pages/MainApp";
import { OverlayLoader } from './shared/components/OverlayLoader';
import NativePlayerOverlay from './features/player/components/NativePlayerOverlay';

const UploadPlanSandbox = lazy(() => import("./features/upload/components/UploadPlanSandbox"));

function useParam(name) {
  const params = new URLSearchParams(globalThis.location.search);
  return params.get(name);
}

export default function App() {
  const overlay = useParam("overlay");
  const debug = useParam("debug");

  useEffect(() => {
    const handleContextMenu = (e) => {
      e.preventDefault();
    };
    document.addEventListener("contextmenu", handleContextMenu);
    return () => document.removeEventListener("contextmenu", handleContextMenu);
  }, []);

  if (debug === "upload-plan") {
    return (
      <Suspense fallback={<OverlayLoader message="Loading Sandbox..." />}>
        <UploadPlanSandbox />
      </Suspense>
    );
  }

  if (overlay === "native-player") {
    // Transparent body cho overlay window
    document.documentElement.classList.add("native-overlay-mode");
    return <NativePlayerOverlay />;
  }

  return <MainApp />;
}

export { MainApp };
