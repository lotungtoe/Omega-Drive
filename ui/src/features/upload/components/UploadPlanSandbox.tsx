import { Suspense, lazy } from "react";
import { OverlayLoader } from "../../../shared/components/OverlayLoader";

const LazyUploadPlanModal = lazy(() => 
  import("./UploadPlanModal").then(m => ({ default: m.UploadPlanModal }))
);

/**
 * Isolated environment for debugging the UploadPlanModal
 */
export function UploadPlanSandbox() {
  const mockEntries = [
    { name: "Test_Project_Alpha.mp4", size: 1024 * 1024 * 500 },
    { name: "Backup_Document.zip", size: 1024 * 1024 * 25 }
  ];

  const handleClose = () => {
    console.log("[Sandbox] Modal Closed");
    // globalThis.location.href = "/"; // Uncomment to redirect back
  };

  const handleProceed = (results) => {
    console.log("[Sandbox] Proceeded with results:", results);
    alert("Proceeded! Check console for data.");
  };

  const mockToast = {
    show: (msg, type) => console.log(`[Mock Toast] ${type}: ${msg}`),
    remove: () => {}
  };

  return (
    <div className="gd-app dark min-h-screen bg-[#0d1117] flex items-center justify-center p-10">
      <div className="w-full max-w-4xl p-6 bg-[#161b22] border border-[#30363d] rounded-2xl shadow-2xl relative overflow-hidden">
        <h2 className="text-white text-xl font-bold mb-6 opacity-40">Sandbox: Upload Plan Debug</h2>
        
        <Suspense fallback={<OverlayLoader message="Loading Sandbox Component..." />}>
          <LazyUploadPlanModal 
            isOpen={true} 
            entries={mockEntries} 
            onClose={handleClose}
            onProceed={handleProceed}
            toast={mockToast}
            dark={true}
          />
        </Suspense>

        <div className="mt-8 pt-6 border-t border-[#30363d] text-[#8b949e] text-sm">
          <p>Dùng URL <code>?debug=upload-plan</code> để truy cập trang này.</p>
          <p className="mt-2 text-xs italic opacity-80">
            * Backend được giả lập qua <code>uploadPlanMocks.js</code>.
          </p>
        </div>
      </div>
    </div>
  );
}

export default UploadPlanSandbox;
