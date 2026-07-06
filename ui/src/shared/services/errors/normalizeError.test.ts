import { describe, it, expect } from "vitest";
import { normalizeError } from './normalizeError';
import { ERROR_CODES } from './types';

describe("normalizeError", () => {
  it("handles string error", () => {
    const err = normalizeError("Boom");
    expect(err.code).toBe(ERROR_CODES.UNKNOWN);
    expect(err.message).toBe("Boom");
  });

  it("handles Error instance", () => {
    const base = new Error("Failed");
    const err = normalizeError(base);
    expect(err.message).toBe("Failed");
    expect(err.code).toBe(ERROR_CODES.UNKNOWN);
    expect(err.stack).toBeTruthy();
  });

  it("handles object error with code/message", () => {
    const err = normalizeError({ code: "E_DB", message: "DB error" });
    expect(err.code).toBe("E_DB");
    expect(err.message).toBe("DB error");
  });

  it("handles JSON string error", () => {
    const err = normalizeError(JSON.stringify({ code: "E_IO", message: "IO error" }));
    expect(err.code).toBe("E_IO");
    expect(err.message).toBe("IO error");
  });
});
