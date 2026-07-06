import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

if (typeof globalThis.DOMMatrix === "undefined") {
  globalThis.DOMMatrix = class DOMMatrix {};
}

if (typeof globalThis.Path2D === "undefined") {
  globalThis.Path2D = class Path2D {};
}

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));
