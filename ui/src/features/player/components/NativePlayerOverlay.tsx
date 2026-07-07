import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  getMpvStatus,
  seekMpv,
  setMpvSpeed,
  setMpvVolume,
  toggleMpvFullscreen,
  toggleMpvPlayPause,
} from "../api/mpv";
import "./NativePlayerOverlay.css";

function formatTime(seconds) {
  if (!Number.isFinite(seconds) || seconds <= 0) {
    return "0:00";
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }

  return `${minutes}:${String(secs).padStart(2, "0")}`;
}

function toErrorMessage(error) {
  if (error instanceof Error && error.message) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  if (error && typeof error === "object" && typeof error.message === "string") {
    return error.message.trim() || "Unknown native player error";
  }
  return "Unknown native player error";
}

const SPEEDS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2, 3, 4];
const POLL_INTERVAL_MS = 200;
const STARTUP_ERROR_GRACE_MS = 1500;

export default function NativePlayerOverlay() {
  const [status, setStatus] = useState({
    position: 0,
    duration: 0,
    paused: true,
    volume: 100,
    speed: 1,
    title: "",
    alive: false,
    fullscreen: false,
  });
  const [showVolume, setShowVolume] = useState(false);
  const [showSpeed, setShowSpeed] = useState(false);
  const [seeking, setSeeking] = useState(false);
  const [seekPos, setSeekPos] = useState(0);
  const [visible, setVisible] = useState(true);
  const [lastError, setLastError] = useState("");
  const hideTimer = useRef(null);
  const seekBarRef = useRef(null);
  const lastPollErrorRef = useRef("");
  const pollStartedAtRef = useRef(0);

  const params = new URLSearchParams(globalThis.location.search);
  const titleFromUrl = decodeURIComponent(params.get("title") || "");

  const armHideTimer = useCallback(() => {
    clearTimeout(hideTimer.current);
    hideTimer.current = setTimeout(() => {
      if (!showVolume && !showSpeed) {
        setVisible(false);
      }
    }, 3000);
  }, [showSpeed, showVolume]);

  const resetHide = useCallback(() => {
    setVisible(true);
    armHideTimer();
  }, [armHideTimer]);

  const reportOverlayError = useCallback((source, error) => {
    console.error(`[NativePlayerOverlay] ${source}`, error);
    setLastError(`${source}: ${toErrorMessage(error)}`);
  }, []);

  const runAction = useCallback(async (source, action) => {
    try {
      const result = await action();
      setLastError("");
      return result;
    } catch (error) {
      reportOverlayError(source, error);
      return undefined;
    }
  }, [reportOverlayError]);

  useEffect(() => {
    armHideTimer();
    return () => {
      clearTimeout(hideTimer.current);
    };
  }, [armHideTimer]);

  useEffect(() => {
    let unlisten;

    const setup = async () => {
      try {
        unlisten = await listen("playback-state-changed", (event) => {
          if (!event.payload) return;
          pollStartedAtRef.current = Date.now();
          lastPollErrorRef.current = "";
          setVisible(true);
          setLastError("");
          resetHide();
        });
      } catch (error) {
        console.error("[NativePlayerOverlay] listen(playback-state-changed)", error);
      }
    };

    void setup();
    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [resetHide]);

  useEffect(() => {
    let active = true;
    pollStartedAtRef.current = Date.now();

    const poll = async () => {
      while (active) {
        try {
          const nextStatus = await getMpvStatus();
          if (!active) break;

          setStatus(nextStatus as any);
          setLastError("");
          lastPollErrorRef.current = "";

          if (!(nextStatus as any).alive) {
            try {
              await getCurrentWindow().close();
            } catch {
              // Ignore close failures: the overlay can disappear with the session.
            }
            break;
          }
        } catch (error) {
          const message = toErrorMessage(error);
          const inStartupGrace = Date.now() - pollStartedAtRef.current < STARTUP_ERROR_GRACE_MS;
          if (!inStartupGrace && lastPollErrorRef.current !== message) {
            lastPollErrorRef.current = message;
            reportOverlayError("mpv_get_status", error);
          }
        }

        await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
      }
    };

    void poll();
    return () => {
      active = false;
    };
  }, [reportOverlayError]);

  const updateSeekFromEvent = useCallback((event) => {
    const bar = seekBarRef.current;
    if (!bar || status.duration <= 0) return;

    const rect = bar.getBoundingClientRect();
    const ratio = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    setSeekPos(ratio * status.duration);
  }, [status.duration]);

  useEffect(() => {
    if (!seeking) {
      return undefined;
    }

    const handleMouseUp = () => {
      const nextSeekPos = seekPos;
      setSeeking(false);
      void runAction("mpv_seek", () => seekMpv(nextSeekPos));
    };
    const handleMouseMove = (event) => updateSeekFromEvent(event);

    globalThis.addEventListener("mouseup", handleMouseUp);
    globalThis.addEventListener("mousemove", handleMouseMove);

    return () => {
      globalThis.removeEventListener("mouseup", handleMouseUp);
      globalThis.removeEventListener("mousemove", handleMouseMove);
    };
  }, [runAction, seekPos, seeking, updateSeekFromEvent]);

  const displayPos = seeking ? seekPos : status.position;
  const seekRatio = status.duration > 0 ? (displayPos / status.duration) * 100 : 0;
  const title = status.title || titleFromUrl || "Video";

  return (
    <div
      className={`mpv-overlay ${visible ? "mpv-visible" : "mpv-hidden"}`}
      onMouseMove={resetHide}
      onMouseEnter={() => setVisible(true)}
      data-tauri-drag-region
    >
      <div
        ref={seekBarRef}
        className="mpv-seek-bar"
        onMouseDown={(event) => {
          setSeeking(true);
          updateSeekFromEvent(event);
        }}
      >
        <div className="mpv-seek-track">
          <div className="mpv-seek-fill" style={{ width: `${seekRatio}%` }} />
          <div className="mpv-seek-thumb" style={{ left: `${seekRatio}%` }} />
        </div>
      </div>

      <div className="mpv-controls">
        <div className="mpv-left">
          <button type="button"
            className="mpv-btn"
            onClick={() => void runAction("mpv_play_pause", () => toggleMpvPlayPause())}
            title={status.paused ? "Phat" : "Tam dung"}
          >
            {status.paused ? "\u25B6" : "\u23F8"}
          </button>

          <div
            className="mpv-volume-group"
            onMouseEnter={() => setShowVolume(true)}
            onMouseLeave={() => setShowVolume(false)}
          >
            <button type="button"
              className="mpv-btn"
              onClick={() => void runAction("mpv_set_volume", () => setMpvVolume(status.volume > 0 ? 0 : 100))}
            >
              {status.volume === 0 ? "\uD83D\uDD07" : status.volume < 50 ? "\uD83D\uDD09" : "\uD83D\uDD0A"}
            </button>
            {showVolume ? (
              <input
                type="range"
                className="mpv-volume-slider"
                min="0"
                max="100"
                step="1"
                value={status.volume}
                onChange={(event) => void runAction("mpv_set_volume", () => setMpvVolume(Number(event.target.value)))}
              />
            ) : null}
          </div>

          <span className="mpv-time">
            {formatTime(displayPos)} / {formatTime(status.duration)}
          </span>
        </div>

        <div className="mpv-center" title={title}>
          <span className="mpv-title">{title}</span>
        </div>

        <div className="mpv-right">
          <div
            className="mpv-speed-group"
            onMouseEnter={() => setShowSpeed(true)}
            onMouseLeave={() => setShowSpeed(false)}
          >
            <button type="button" className="mpv-btn mpv-speed-btn">{status.speed}x</button>
            {showSpeed ? (
              <div className="mpv-speed-popup">
                {SPEEDS.map((speed) => (
                  <button type="button"
                    key={speed}
                    className={`mpv-speed-option ${status.speed === speed ? "active" : ""}`}
                    onClick={() => {
                      void runAction("mpv_set_speed", () => setMpvSpeed(speed));
                      setShowSpeed(false);
                    }}
                  >
                    {speed}x
                  </button>
                ))}
              </div>
            ) : null}
          </div>

          <button type="button"
            className="mpv-btn"
            onClick={() => void runAction("mpv_toggle_fullscreen", () => toggleMpvFullscreen())}
            title={status.fullscreen ? "Thoat toan man hinh" : "Toan man hinh"}
          >
            ⛶
          </button>
        </div>
      </div>

      {lastError ? (
        <div className="mpv-error-banner" role="status">
          {lastError}
        </div>
      ) : null}
    </div>
  );
}
