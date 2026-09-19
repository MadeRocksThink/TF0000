import { describe, expect, it } from "vitest";
import { createMemorySchema } from "./index";

describe("createMemorySchema", () => {
  it("accepts a valid memory", () => {
    expect(
      createMemorySchema.safeParse({
        projectId: null,
        memorySpaceId: null,
        type: "decision",
        authority: "user_confirmed",
        status: "active",
        title: "Use SQLite",
        content: "SQLite is the canonical local store.",
      }).success,
    ).toBe(true);
  });

  it("rejects empty content", () => {
    expect(
      createMemorySchema.safeParse({
        projectId: null,
        memorySpaceId: null,
        type: "decision",
        authority: "user_confirmed",
        status: "active",
        title: "Invalid",
        content: " ",
      }).success,
    ).toBe(false);
  });
});
