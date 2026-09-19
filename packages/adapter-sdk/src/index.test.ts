import { describe, expect, it } from "vitest";
import { ADAPTER_API_VERSION, defineAdapter, verifyAdapterFixture } from "./index";

const capture = {
  provider: "example",
  title: "Fixture",
  capturedAt: "2026-01-01T00:00:00Z",
  messages: [{ role: "user" as const, body: "Hello", ordinal: 0, sourceHash: "a".repeat(64) }],
};
const adapter = {
  detectConversation: async () => ({ provider: "example", title: "Fixture" }),
  captureVisibleConversation: async () => capture,
  captureSelection: async () => [],
  readComposer: async () => null,
  insertContext: async () => ({ inserted: true, method: "composer" as const }),
  getConversationMetadata: async () => ({ provider: "example", title: "Fixture", messageCount: 1 }),
  healthCheck: async () => ({ status: "healthy" as const, adapterVersion: "1.0.0" }),
};

describe("stable adapter API", () => {
  it("validates manifests", () => {
    const registration = defineAdapter({
      manifest: {
        apiVersion: ADAPTER_API_VERSION,
        id: "example-ai",
        displayName: "Example AI",
        version: "1.0.0",
        hostnames: ["example.test"],
        capabilities: {
          captureConversation: true,
          captureSelection: true,
          composerRead: true,
          composerInsert: true,
          clipboardFallback: true,
        },
      },
      create: () => adapter,
    });
    expect(registration.manifest.id).toBe("example-ai");
    expect(() =>
      defineAdapter({ ...registration, manifest: { ...registration.manifest, id: "Bad id" } }),
    ).toThrow("Invalid adapter id");
  });
  it("runs the reusable fixture contract", async () => {
    await expect(
      verifyAdapterFixture(adapter, {
        provider: "example",
        title: "Fixture",
        messages: [{ role: "user", body: "Hello" }],
      }),
    ).resolves.toMatchObject({ passed: true });
  });
});
