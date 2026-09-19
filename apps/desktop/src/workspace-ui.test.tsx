import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { DashboardSnapshot, Memory } from "./api";
import { memoryBreakdown, viewFromHash, views, WorkspaceInsights } from "./workspace-ui";

const empty: DashboardSnapshot = {
  projects: [],
  memorySpaces: [],
  memories: [],
  contextPacks: [],
  memorySpaceLinks: [],
};
const memories = ["active", "active", "draft", "superseded", "archived", "rejected"].map(
  (status, index): Memory => ({
    id: String(index),
    projectId: null,
    memoryType: "decision",
    status,
    authority: index === 0 ? "user_confirmed" : "ai_suggestion",
    title: `Memory ${index}`,
    currentVersionId: `v${index}`,
    currentContent: "Example",
    createdAt: "2026-01-01T00:00:00Z",
  }),
);

describe("workspace navigation", () => {
  it.each(views)("resolves the %s screen", (id) => {
    expect(viewFromHash(`#${id}`)).toBe(id);
  });
  it("preserves the old spaces link and safely handles unknown links", () => {
    expect(viewFromHash("#spaces")).toBe("memories");
    expect(viewFromHash("#unknown")).toBe("dashboard");
    expect(viewFromHash("")).toBe("dashboard");
  });
});

describe("workspace insights", () => {
  it("counts each lifecycle state without modifying the snapshot", () => {
    const snapshot = { ...empty, memories };
    expect(memoryBreakdown(snapshot).map((item) => item.count)).toEqual([2, 1, 1, 1, 1]);
    expect(snapshot.memories).toBe(memories);
    expect(memoryBreakdown(empty).every((item) => item.count === 0)).toBe(true);
  });
  it("distinguishes loading from an empty workspace", () => {
    const render = (loaded: boolean) =>
      renderToStaticMarkup(
        createElement(WorkspaceInsights, {
          snapshot: empty,
          sources: [],
          loaded,
        }),
      );
    expect(render(false)).toContain("Waiting for local data.");
    expect(render(false)).toContain("Loading saved conversations");
    expect(render(true)).toContain("Save your first memory");
    expect(render(true)).toContain('href="#import"');
  });
  it("counts whole conversations only, not each message or fragment", () => {
    const source = {
      provider: "Example AI",
      conversationTitle: "Example",
      role: "user",
      content: "Hello",
      sourceUrl: null,
      capturedAt: "2026-01-01T00:00:00Z",
    };
    const html = renderToStaticMarkup(
      createElement(WorkspaceInsights, {
        snapshot: { ...empty, memories },
        loaded: true,
        sources: [
          { ...source, id: "c1", sourceType: "conversation" },
          { ...source, id: "c2", sourceType: "conversation" },
          { ...source, id: "m1", sourceType: "message" },
          { ...source, id: "f1", sourceType: "fragment" },
        ],
      }),
    );
    expect(html).toContain("Example AI: 2 of 2 saved conversations");
    expect(html).toContain('class="insight-value">1<span>active, user-confirmed');
    expect(html).toContain("6 memories across your entire workspace.");
  });
});
