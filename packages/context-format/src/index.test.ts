import { describe, expect, it } from "vitest";
import {
  formatContextPack,
  PORTABLE_FORMAT,
  parsePortableWorkspace,
  serializePortableWorkspace,
  validatePortableWorkspace,
} from "./index";

const workspace = {
  format: PORTABLE_FORMAT,
  schemaVersion: 1 as const,
  exportedAt: "2026-09-19T00:00:00Z",
  sourceDeviceId: "laptop",
  projects: [
    { id: "p1", name: "Demo", description: "", createdAt: "2026-01-01", archivedAt: null },
  ],
  memorySpaces: [
    {
      id: "s1",
      projectId: "p1",
      parentId: null,
      name: "Decisions",
      description: "",
      defaultScope: "project",
      createdAt: "2026-01-01",
      archivedAt: null,
    },
  ],
  memories: [
    {
      memory: {
        id: "m1",
        projectId: "p1",
        memoryType: "decision",
        authority: "user_confirmed",
        status: "active",
        title: "Choice",
        currentVersionId: "v1",
        currentContent: "Use SQLite",
        createdAt: "2026-01-01",
      },
      versions: [
        {
          id: "v1",
          memoryId: "m1",
          content: "Use SQLite",
          changeType: "create",
          createdAt: "2026-01-01",
          supersedesVersionId: null,
        },
      ],
      sources: [],
      memorySpaceIds: ["s1"],
    },
  ],
  contextPacks: [],
};
describe("portable project format", () => {
  it("round trips a valid workspace", () =>
    expect(parsePortableWorkspace(serializePortableWorkspace(workspace))).toEqual(workspace));
  it("rejects missing current versions and dangling spaces", () => {
    expect(() =>
      validatePortableWorkspace({
        ...workspace,
        memories: [{ ...workspace.memories[0], versions: [] }],
      }),
    ).toThrow("current version is missing");
    expect(() =>
      validatePortableWorkspace({
        ...workspace,
        memories: [{ ...workspace.memories[0], memorySpaceIds: ["missing"] }],
      }),
    ).toThrow("unknown space");
  });
  it("keeps the readable Context Pack representation", () =>
    expect(
      formatContextPack("Demo", [{ id: "1", title: "Decision", content: "Use SQLite" }]),
    ).toContain("# TF0000 Context Pack: Demo"));
});
