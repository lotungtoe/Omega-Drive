import { describe, it, expect } from "vitest";
import { toUserMessage } from './toUserMessage';

describe("toUserMessage", () => {
  it("maps known code to friendly message", () => {
    const msg = toUserMessage({ code: "E_DB", message: "db fail" });
    expect(msg.message).toContain("Hệ thống lưu trữ");
    expect(msg.details.code).toBe("E_DB");
  });

  it("falls back to error message", () => {
    const msg = toUserMessage({ code: "E_UNKNOWN", message: "Boom" });
    expect(msg.message).toBe("Boom");
  });
});
