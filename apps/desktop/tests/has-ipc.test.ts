import { describe, expect, it } from "vitest";
import { hasIpc } from "../src/App";

/**
 * Unit tests for the IPC availability guard.
 *
 * `hasIpc()` decides whether the UI may call the Tauri backend at all. Every
 * command call in App.tsx is gated on it, so a wrong answer either blocks a
 * working backend or causes the UI to invoke a bridge that does not exist and
 * render a crash instead of the honest "backend unavailable" state.
 *
 * These run in a Node environment (no DOM), which is exactly the "outside
 * Tauri" case, so the default expectation is `false`.
 */

describe("hasIpc", () => {
  it("reports false when there is no window (non-browser environment)", () => {
    expect(typeof window).toBe("undefined");
    expect(hasIpc()).toBe(false);
  });

  it("is a pure predicate with no side effects", () => {
    const first = hasIpc();
    const second = hasIpc();
    expect(first).toBe(second);
  });

  it("returns a strict boolean, never a truthy value", () => {
    // The UI branches on this; returning a truthy object would still "work" in
    // an if-statement but would break `=== true` comparisons and serialization.
    expect(typeof hasIpc()).toBe("boolean");
  });
});
