import { describe, expect, it } from "vitest";
import { toMessage } from "@/lib/api";
import { cn } from "@/lib/utils";

describe("toMessage", () => {
  it("parses serialized ZenouError JSON strings", () => {
    const raw = JSON.stringify({
      code: "elevation_declined",
      message: "Administrator permission was declined.",
      details: "UAC cancelled by user",
    });
    expect(toMessage(raw)).toEqual({
      message: "Administrator permission was declined.",
      details: "UAC cancelled by user",
    });
  });

  it("handles plain strings", () => {
    expect(toMessage("boom")).toEqual({ message: "boom", details: null });
  });

  it("handles error objects", () => {
    expect(toMessage({ message: "nope", details: null }).message).toBe("nope");
  });

  it("falls back to a friendly message for unknown shapes", () => {
    expect(toMessage(42).message).toContain("went wrong");
  });
});

describe("cn", () => {
  it("joins truthy class names", () => {
    expect(cn("a", false && "b", "c")).toBe("a c");
  });

  it("returns empty string for no inputs", () => {
    expect(cn()).toBe("");
  });
});
