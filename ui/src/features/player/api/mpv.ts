import { call } from "../../../shared/api/call";

export interface MpvStatus {
  alive: boolean;
  paused: boolean;
  position: number;
  duration: number;
  volume: number;
  speed: number;
  fullscreen: boolean;
  title: string;
}

export function getMpvStatus() {
  return call("mpv_get_status", {}, { feature: "player", action: "mpv_get_status" });
}

export function toggleMpvPlayPause() {
  return call("mpv_play_pause", {}, { feature: "player", action: "mpv_play_pause" });
}

export function seekMpv(position: number) {
  return call("mpv_seek", { position }, { feature: "player", action: "mpv_seek" });
}

export function setMpvVolume(volume: number) {
  return call("mpv_set_volume", { volume }, { feature: "player", action: "mpv_set_volume" });
}

export function setMpvSpeed(speed: number) {
  return call("mpv_set_speed", { speed }, { feature: "player", action: "mpv_set_speed" });
}

export function toggleMpvFullscreen() {
  return call("mpv_toggle_fullscreen", {}, { feature: "player", action: "mpv_toggle_fullscreen" });
}
