import { describe, it, expect } from "vitest";
import { toDownloadView, toUploadView } from "./transferView";

const t = (k: string) => k;

describe("toDownloadView", () => {  const job = {
    id: 3,
    target_path: "C:/dl/b.mkv",
    state: "downloading",
    done_parts: 2,
    total_parts: 4,
  };
  it("maps active job with live stats", () => {
    const view = toDownloadView(job, { 3: { speedBps: 100, etaSecs: 5, bytesDone: 50, bytesTotal: 100 } }, t);
    expect(view.source).toBe("download");
    expect(view.filename).toBe("b.mkv");
    expect(view.status).toBe("active");
    expect(view.percent).toBe(50);
    expect(view.speedBps).toBe(100);
  });
  it("maps failed job with error code in status text", () => {
    const view = toDownloadView({ ...job, state: "failed", error_code: "E_IO" }, {}, t);
    expect(view.status).toBe("failed");
    expect(view.statusText).toContain("Failed");
    expect(view.statusText).toContain("E_IO");
  });
  it("caps completed at 100", () => {
    const view = toDownloadView({ ...job, state: "completed" }, {}, t);
    expect(view.status).toBe("done");
    expect(view.percent).toBe(100);
  });
});

describe("toUploadView", () => {
  const file = { id: 7, filename: "a.mp4", size: 1000, status: "uploading" };
  const live = {
    "id:7": {
      percent: 30, detail: "Uploading parts (3/10)", phase: "uploading",
      speedBps: 50, etaSecs: 10, bytesDone: 300, bytesTotal: 1000,
    },
  };
  it("maps live stat to active view", () => {
    const view = toUploadView(file, live, t);
    expect(view.source).toBe("upload");
    expect(view.status).toBe("active");
    expect(view.percent).toBe(30);
    expect(view.speedBps).toBe(50);
  });
  it("falls back to size text without live stat", () => {
    const view = toUploadView(file, {}, t);
    expect(view.status).toBe("active");
    expect(view.statusText).toContain("Uploading");
  });
  it("marks processing files indeterminate", () => {
    const view = toUploadView({ ...file, status: "processing" }, {}, t);
    expect(view.status).toBe("processing");
    expect(view.indeterminate).toBe(true);
  });
});
