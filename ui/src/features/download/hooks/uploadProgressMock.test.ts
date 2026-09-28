import { describe, it, expect } from "vitest";
import { nextMockProgress, mockPayload } from "./uploadProgressMock";

describe("nextMockProgress", () => {
  it("advances and holds at 95 until backend completes it", () => {
    expect(nextMockProgress(0)).toBeGreaterThan(0);
    expect(nextMockProgress(90)).toBe(95);
    expect(nextMockProgress(95)).toBe(95);
  });
});

describe("mockPayload", () => {
  it("builds bytes consistent with percent", () => {
    const p = mockPayload({ id: 7, filename: "a.mp4", size: 1000 }, 30);
    expect(p.fileId).toBe(7);
    expect(p.phase).toBe("uploading");
    expect(p.overallProgress).toBe(30);
    expect(p.platforms[0].done).toBe(300);
    expect(p.platforms[0].total).toBe(1000);
  });
});
