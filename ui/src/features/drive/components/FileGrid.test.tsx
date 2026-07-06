import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key) => key,
  }),
}));

vi.mock("./FileCard/FileCard", () => ({
  FileCard: ({ file, onDelete, onDownload, onPlay, onPreview, onToggleStar }) => (
    <div data-testid={`file-card-${file.id}`}>
      <button type="button" onClick={onDelete}>delete</button>
      <button type="button" onClick={onDownload}>download</button>
      <button type="button" onClick={onPlay}>play</button>
      <button type="button" onClick={onPreview}>preview</button>
      <button type="button" onClick={onToggleStar}>star</button>
    </div>
  ),
}));

vi.mock("../../../shared/components/Common", () => ({
  EmptyState: () => <div>empty</div>,
}));

vi.mock("./Toolbar/SortBar", () => ({
  SortBar: () => null,
}));

vi.mock("./Toolbar/ListHeader", () => ({
  ListHeader: () => null,
}));

import { FileGrid } from './FileGrid';
import {
  DriveControllerContext,
  MainAppUiActionsContext,
  MainAppUiStateContext,
} from '../pages/useMainAppContext';

describe("FileGrid", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
    };

    globalThis.requestAnimationFrame = (callback) => callback();
    globalThis.innerHeight = 900;
    globalThis.innerWidth = 1440;
  });

  it("falls back to context handlers when file action props are omitted", () => {
    const handleDownload = vi.fn();
    const handlePlay = vi.fn();
    const handlePreview = vi.fn();
    const deleteItem = vi.fn();
    const toggleStar = vi.fn();

    const file = {
      id: 1,
      filename: "clip.mp4",
      status: "ready",
      starred: false,
      size: 1024,
      isFolder: false,
      created_at: "2026-03-29T00:00:00.000Z",
    };

    render(
      <MainAppUiStateContext.Provider
        value={{
          progressMap: {},
          view: "list",
          dark: false,
          isDragOver: false,
          sort: { field: "name", dir: "asc" },
        }}
      >
        <MainAppUiActionsContext.Provider
          value={{
            handleDownload,
            handlePlay,
            handlePreview,
            setSort: vi.fn(),
          }}
        >
          <DriveControllerContext.Provider
            value={{
              files: [file],
              deleteItem,
              toggleStar,
              setCurrentFolderId: vi.fn(),
              filesHasMore: false,
              loadingMore: false,
            }}
          >
            <FileGrid files={[file]} />
          </DriveControllerContext.Provider>
        </MainAppUiActionsContext.Provider>
      </MainAppUiStateContext.Provider>
    );

    fireEvent.click(screen.getByText("delete"));
    fireEvent.click(screen.getByText("download"));
    fireEvent.click(screen.getByText("play"));
    fireEvent.click(screen.getByText("preview"));
    fireEvent.click(screen.getByText("star"));

    expect(deleteItem).toHaveBeenCalledWith(expect.objectContaining(file));
    expect(handleDownload).toHaveBeenCalledWith(expect.objectContaining(file));
    expect(handlePlay).toHaveBeenCalledWith(expect.objectContaining(file));
    expect(handlePreview).toHaveBeenCalledWith(expect.objectContaining(file));
    expect(toggleStar).toHaveBeenCalledWith(expect.objectContaining(file));
  });
});
