import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check } from "@tauri-apps/plugin-updater";

interface BinaryStatus {
  name: string;
  version: string | null;
  path: string | null;
  exists: boolean;
  update_available: boolean;
}

export function UpdaterSection() {
  const [appVersion, setAppVersion] = useState<string>("");
  const [binaryStatuses, setBinaryStatuses] = useState<BinaryStatus[]>([]);
  const [checking, setChecking] = useState(false);

  useEffect(() => {
    invoke("check_app_update").then((res: any) => {
      setAppVersion(res?.current_version ?? "?");
    });
    invoke<BinaryStatus[]>("get_binary_status").then(setBinaryStatuses);
  }, []);

  const handleCheckUpdate = async () => {
    setChecking(true);
    try {
      const update = await check();
      if (update) {
        const yes = confirm(
          `New version ${update.version} available. Download and install?`
        );
        if (yes) {
          await update.downloadAndInstall();
        }
      } else {
        alert("You have the latest version.");
      }
    } catch (e) {
      alert(`Update check failed: ${e}`);
    } finally {
      setChecking(false);
    }
  };

  const handleCheckBinaryUpdates = async () => {
    setChecking(true);
    try {
      const statuses = await invoke<BinaryStatus[]>("check_binary_updates");
      setBinaryStatuses(statuses);
      const updatable = statuses.filter((s) => s.update_available);
      if (updatable.length === 0) {
        alert("All binaries are up to date.");
      } else {
        alert(`Updates available for: ${updatable.map((s) => s.name).join(", ")}`);
      }
    } catch (e) {
      alert(`Binary check failed: ${e}`);
    } finally {
      setChecking(false);
    }
  };

  return (
    <div className="gd-settings-section">
      <div style={{ marginBottom: 16 }}>
        <h3 className="gd-settings-section-title">App Update</h3>
        <p className="gd-settings-section-desc">Check for new versions of Omega Drive</p>
      </div>
      <div className="gd-settings-section-content">
        <div className="gd-settings-row">
          <div style={{ flex: 1 }}>
            <div className="gd-settings-row-label">Omega Drive</div>
            <div className="gd-settings-row-desc" style={{ fontFamily: "monospace" }}>
              {appVersion || "loading..."}
            </div>
          </div>
          <button
            onClick={handleCheckUpdate}
            disabled={checking}
            style={{
              padding: "6px 16px",
              borderRadius: 6,
              border: "1px solid var(--gd-outline)",
              background: "var(--gd-blue)",
              color: "#fff",
              cursor: checking ? "not-allowed" : "pointer",
              opacity: checking ? 0.6 : 1,
              fontSize: 13,
            }}
          >
            {checking ? "Checking..." : "Check for Updates"}
          </button>
        </div>
      </div>

      <div style={{ marginTop: 24, marginBottom: 16 }}>
        <h3 className="gd-settings-section-title">Supporting Components</h3>
        <p className="gd-settings-section-desc">ffmpeg, ffprobe, yt-dlp, deno</p>
      </div>
      <div className="gd-settings-section-content">
        {binaryStatuses.map((bin) => (
          <div key={bin.name} className="gd-settings-row">
            <div style={{ flex: 1, display: "flex", alignItems: "center", gap: 8 }}>
              <span style={{ fontFamily: "monospace", width: 60 }}>{bin.name}</span>
              <span style={{ color: bin.exists ? "#22c55e" : "#ef4444", fontSize: 16 }}>
                {bin.exists ? "\u2713" : "\u2717"}
              </span>
              {bin.update_available && (
                <span style={{ color: "#f59e0b", fontSize: 12 }}>(update available)</span>
              )}
            </div>
          </div>
        ))}
        <div className="gd-settings-row">
          <div style={{ flex: 1 }} />
          <button
            onClick={handleCheckBinaryUpdates}
            disabled={checking}
            style={{
              padding: "4px 12px",
              borderRadius: 6,
              border: "1px solid var(--gd-outline)",
              background: "transparent",
              cursor: checking ? "not-allowed" : "pointer",
              opacity: checking ? 0.6 : 1,
              fontSize: 12,
            }}
          >
            Check Binaries
          </button>
        </div>
      </div>
    </div>
  );
}
