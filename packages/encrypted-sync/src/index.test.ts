import { PORTABLE_FORMAT, type PortableWorkspace } from "@tf0000/context-format";
import { describe, expect, it } from "vitest";
import {
  buildMemoryMergePlan,
  decryptWorkspace,
  encryptWorkspace,
  parseEncryptedEnvelope,
  serializeEncryptedEnvelope,
} from "./index";

const record = (current: string, versions: string[]) => ({
  memory: {
    id: "m1",
    projectId: "p1",
    memoryType: "decision",
    authority: "user_confirmed",
    status: "active",
    title: "Choice",
    currentVersionId: current,
    currentContent: current,
    createdAt: "2026-01-01",
  },
  versions: versions.map((id, index) => ({
    id,
    memoryId: "m1",
    content: id,
    changeType: index ? "replace" : "create",
    createdAt: `2026-01-0${index + 1}`,
    supersedesVersionId: index ? (versions[index - 1] ?? null) : null,
  })),
  sources: [],
  memorySpaceIds: [],
});
const workspace = (memory = record("v1", ["v1"])): PortableWorkspace => ({
  format: PORTABLE_FORMAT,
  schemaVersion: 1,
  exportedAt: "2026-01-01",
  sourceDeviceId: "laptop",
  projects: [
    { id: "p1", name: "Project", description: "", createdAt: "2026-01-01", archivedAt: null },
  ],
  memorySpaces: [],
  memories: [memory],
  contextPacks: [],
});
describe("encrypted selective sync", () => {
  it("encrypts and authenticates without exposing plaintext", async () => {
    const envelope = await encryptWorkspace(workspace(), "correct horse battery staple");
    const json = serializeEncryptedEnvelope(envelope);
    expect(json).not.toContain("Choice");
    await expect(
      decryptWorkspace(parseEncryptedEnvelope(json), "correct horse battery staple"),
    ).resolves.toEqual(workspace());
  });
  it("rejects wrong passphrases and modified metadata", async () => {
    const envelope = await encryptWorkspace(workspace(), "correct horse battery staple");
    await expect(decryptWorkspace(envelope, "different horse battery staple")).rejects.toThrow(
      "Unable to decrypt",
    );
    await expect(
      decryptWorkspace({ ...envelope, deviceId: "attacker" }, "correct horse battery staple"),
    ).rejects.toThrow("Unable to decrypt");
  });
  it("classifies fast-forward and concurrent offline edits", () => {
    expect(
      buildMemoryMergePlan(
        workspace(record("v1", ["v1"])),
        workspace(record("v2", ["v1", "v2"])),
      )[0]?.disposition,
    ).toBe("remote-ahead");
    expect(
      buildMemoryMergePlan(
        workspace(record("v2a", ["v1", "v2a"])),
        workspace(record("v2b", ["v1", "v2b"])),
      )[0]?.disposition,
    ).toBe("conflict");
  });
});
