import { describe, it, expect } from "vitest";
import {
  uploadStatKey,
  uploadBytes,
  resolveUploadStat,
  nextSpeed,
} from "./uploadProgress";

const payload = (over = {}) => ({
  sessionId: "s1",
  fileName: "a.mp4",
  phase: "uploading",
  doneParts: 3,
  totalParts: 10,
  detail: "Uploading parts (3/10)",
  overallProgress: 30,
  platforms: [{ name: "Discord", done: 300, total: 1000 }],
  fileId: 7,
  ...over,
});

describe("uploadStatKey", () => {
  it("prefers file id, falls back to file name", () => {
    expect(uploadStatKey(payload({ fileId: 7 }))).toBe("id:7");
    expect(uploadStatKey(payload({ fileId: null }))).toBe("name:a.mp4");
    expect(uploadStatKey(payload({ fileId: null, fileName: "" }))).toBeNull();
  });
});

describe("uploadBytes", () => {
  it("sums platform bytes", () => {
    expect(
      uploadBytes(payload({ platforms: [{ name: "Discord", done: 300, total: 1000 }] }))
    ).toEqual({ done: 300, total: 1000 });
  });
});

describe("resolveUploadStat", () => {
  const file = { id: 7, filename: "a.mp4", size: 1000, status: "uploading" };
  const stat = (percent: number) => ({
    percent, detail: "", phase: "uploading",
    speedBps: 0, etaSecs: null, bytesDone: 0, bytesTotal: 0,
  });
  it("matches live stat by id first, then by name", () => {
    expect(resolveUploadStat(file, { "id:7": stat(30) })?.percent).toBe(30);
    expect(resolveUploadStat(file, { "name:a.mp4": stat(55) })?.percent).toBe(55);
    expect(resolveUploadStat(file, {})).toBeNull();
  });
});

describe("nextSpeed", () => {
  it("computes bytes-per-second from samples, ignores clock skew", () => {
    const first = nextSpeed(undefined, 1000, 1000);
    expect(first.speed).toBe(0);
    const second = nextSpeed(first.sample, 3000, 3000);
    expect(second.speed).toBe(1000);
    expect(nextSpeed(first.sample, 500, 1000).speed).toBe(0);
  });
});
