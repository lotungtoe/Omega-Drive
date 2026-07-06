import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../../diagnostics/services/diagnosticsService", () => ({
  getConnectionStatus: vi.fn(),
}));

import { getConnectionStatus } from '../../diagnostics/services/diagnosticsService';
import { useStatusMonitor } from './useStatusMonitor';

describe("useStatusMonitor", () => {
  beforeEach(() => {
    getConnectionStatus.mockReset();
  });

  it("falls back to false when nested status fields are missing", async () => {
    getConnectionStatus.mockResolvedValueOnce({});

    const { result, unmount } = renderHook(() => useStatusMonitor(false));

    await waitFor(() => {
      expect(getConnectionStatus).toHaveBeenCalledTimes(1);
      expect(result.current.discordOnline).toBe(false);
      expect(result.current.telegramOnline).toBe(false);
    });

    unmount();
  });
});
