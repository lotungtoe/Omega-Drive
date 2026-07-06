import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key, params = {}) => params.ext ? `${key}:${params.ext}` : key,
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => []),
}));

vi.mock("framer-motion", () => ({
  AnimatePresence: ({ children }) => <>{children}</>,
  motion: {
    div: ({ children, ...props }) => <div {...props}>{children}</div>,
  },
}));

vi.mock("./PdfPreview", () => ({
  PdfPreview: ({ file, onDownload }) => (
    <div>
      <span>{file.filename || file.name}</span>
      <button type="button" title="common.download" onClick={() => onDownload(file)}>
        download
      </button>
    </div>
  ),
}));

import { PreviewModal } from './PreviewModal';

describe("PreviewModal", () => {
  const baseProps = {
    onClose: vi.fn(),
    onDownload: vi.fn(),
    dark: false,
  };

  it("keeps metadata-only preview even if a video file reaches the modal", () => {
    render(
      <PreviewModal
        {...baseProps}
        file={{
          id: 42,
          filename: "sample.mp4",
          size: 1024,
          created_at: "2026-03-29T00:00:00.000Z",
        }}
      />
    );

    expect(screen.getAllByText("sample.mp4").length).toBeGreaterThan(0);
    expect(screen.getByText("drive.download")).toBeInTheDocument();
  });

  it("keeps metadata-only flow for non-video files", () => {
    render(
      <PreviewModal
        {...baseProps}
        file={{
          filename: "notes.pdf",
          size: 2048,
          created_at: "2026-03-29T00:00:00.000Z",
        }}
      />
    );

    expect(screen.getAllByText("notes.pdf").length).toBeGreaterThan(0);
    expect(screen.getByTitle("common.download")).toBeInTheDocument();
  });

  it("prefers backend kind over filename when rendering file type", () => {
    render(
      <PreviewModal
        {...baseProps}
        file={{
          filename: "stream.bin",
          kind: "video",
          size: 4096,
          created_at: "2026-03-29T00:00:00.000Z",
        }}
      />
    );

    expect(screen.getByText("fileType.video:BIN")).toBeInTheDocument();
  });
});
