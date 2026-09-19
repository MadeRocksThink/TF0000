import { memo, useEffect, useState } from "react";
import type { CapturedSource, DashboardSnapshot } from "./api";

export const views = [
  ["dashboard", "Overview", "overview"],
  ["search", "Search", "search"],
  ["memories", "Memory library", "library"],
  ["create", "Create context", "plus"],
  ["import", "Import", "import"],
  ["merge", "Merge & update", "merge"],
  ["history", "History & branches", "history"],
  ["conflicts", "Conflict inbox", "alert"],
  ["smart", "Smart features", "spark"],
  ["sync", "Encrypted sync", "sync"],
  ["safety", "Backup & health", "shield"],
] as const;

export function viewFromHash(hash: string): string {
  const id = hash.replace(/^#/, "");
  if (id === "spaces") return "memories";
  return views.some(([view]) => view === id) ? id : "dashboard";
}

// Original, dependency-free line icons. Labels are always supplied by the control.
const paths: Record<string, string> = {
  overview: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  search: "M20 20l-5-5 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
  library: "M4 4v16 M9 4v16 M14 4v16 M18 4l3 16",
  plus: "M12 5v14 M5 12h14",
  import: "M12 3v12 M7 10l5 5 5-5 M4 16v5h16v-5",
  merge: "M6 3v5c0 4 12 4 12 8v5 M18 3v5c0 4-12 4-12 8v5",
  history: "M3 10a9 9 0 1 1 2 8 M3 3v7h7 M12 7v5l4 2",
  alert: "M12 3L2 21h20L12 3z M12 9v5 M12 17v1",
  spark: "M12 2l3 7 7 3-7 3-3 7-3-7-7-3 7-3z",
  shield: "M12 3l8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3z M8 12l3 3 5-6",
  sync: "M7 7h10l-3-3 M17 17H7l3 3 M20 7a8 8 0 0 1-1 9 M4 17a8 8 0 0 1 1-9",
  sun: "M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0 M12 2v2 M12 20v2 M2 12h2 M20 12h2 M5 5l1 1 M18 18l1 1 M5 19l1-1 M18 6l1-1",
  moon: "M20 15A9 9 0 0 1 9 4a9 9 0 1 0 11 11z",
  arrow: "M5 12h14 M14 7l5 5-5 5",
};

export function Icon({ name }: { name: string }) {
  return (
    <svg
      className="ui-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name] ?? paths.overview} />
    </svg>
  );
}

type Appearance = { mode: "dark" | "light"; accent: "sage" | "iris" | "blue" };
function readAppearance(): Appearance {
  try {
    const stored = JSON.parse(localStorage.getItem("tf0000.appearance") ?? "null");
    return {
      mode: stored?.mode === "light" ? "light" : "dark",
      accent: ["sage", "iris", "blue"].includes(stored?.accent) ? stored.accent : "sage",
    };
  } catch {
    return { mode: "dark", accent: "sage" };
  }
}

export function AppearanceControls() {
  const [appearance, setAppearance] = useState(readAppearance);
  useEffect(() => {
    document.documentElement.dataset.theme = appearance.mode;
    document.documentElement.dataset.accent = appearance.accent;
    try {
      localStorage.setItem("tf0000.appearance", JSON.stringify(appearance));
    } catch {
      /* Session preferences still work without storage. */
    }
  }, [appearance]);
  return (
    <div className="appearance-controls">
      <span className="nav-caption">Make it yours</span>
      <fieldset className="theme-segment" aria-label="Color mode">
        {(["dark", "light"] as const).map((mode) => (
          <button
            key={mode}
            type="button"
            aria-pressed={appearance.mode === mode}
            onClick={() => setAppearance({ ...appearance, mode })}
          >
            <Icon name={mode === "dark" ? "moon" : "sun"} />
            {mode === "dark" ? "Dark" : "Light"}
          </button>
        ))}
      </fieldset>
      <fieldset className="accent-options" aria-label="Accent color">
        {(["sage", "iris", "blue"] as const).map((accent) => (
          <button
            key={accent}
            type="button"
            data-color={accent}
            aria-label={`${accent} accent`}
            title={`${accent} accent`}
            aria-pressed={appearance.accent === accent}
            onClick={() => setAppearance({ ...appearance, accent })}
          >
            <span aria-hidden="true">{appearance.accent === accent ? "✓" : ""}</span>
          </button>
        ))}
        <span className="quiet">{appearance.accent}</span>
      </fieldset>
    </div>
  );
}

export function memoryBreakdown(snapshot: DashboardSnapshot) {
  const counts = new Map<string, number>();
  for (const memory of snapshot.memories) {
    counts.set(memory.status, (counts.get(memory.status) ?? 0) + 1);
  }
  return ["active", "draft", "superseded", "rejected", "archived"].map((status) => ({
    label: status,
    count: counts.get(status) ?? 0,
  }));
}

export const WorkspaceInsights = memo(function WorkspaceInsights({
  snapshot,
  sources,
  loaded,
}: {
  snapshot: DashboardSnapshot;
  sources: CapturedSource[];
  loaded: boolean;
}) {
  const breakdown = memoryBreakdown(snapshot);
  const conversations = sources.filter((source) => source.sourceType === "conversation");
  const providerCounts = new Map<string, number>();
  for (const source of conversations) {
    providerCounts.set(source.provider, (providerCounts.get(source.provider) ?? 0) + 1);
  }
  const providers = [...providerCounts]
    .map(([label, count]) => ({ label, count }))
    .sort((left, right) => right.count - left.count || left.label.localeCompare(right.label));
  const confirmed = snapshot.memories.filter(
    (memory) => memory.authority === "user_confirmed" && memory.status === "active",
  ).length;
  const total = snapshot.memories.length;
  return (
    <div className="insights-grid">
      <article className="insight-card">
        <div className="insight-heading">
          <span className="eyebrow">Memory health</span>
          <Icon name="library" />
        </div>
        <h3>Your knowledge, at a glance.</h3>
        <div className="insight-value">
          {loaded ? confirmed : "—"}
          <span>active, user-confirmed memories</span>
        </div>
        <div className="composition-bar" aria-hidden="true">
          {breakdown
            .filter((item) => item.count > 0)
            .map((item) => (
              <span key={item.label} data-status={item.label} style={{ flex: item.count }} />
            ))}
        </div>
        <div className="chart-legend">
          {breakdown.map((item) => (
            <span key={item.label}>
              <i data-status={item.label} />
              {item.label}
              <strong>{loaded ? item.count : "—"}</strong>
            </span>
          ))}
        </div>
        <p className="quiet">
          {!loaded
            ? "Waiting for local data."
            : total
              ? `${total} memories across your entire workspace.`
              : "Save your first memory to start building context."}
        </p>
      </article>
      <article className="insight-card">
        <div className="insight-heading">
          <span className="eyebrow">Connected context</span>
          <Icon name="merge" />
        </div>
        <h3>Conversations you’ve saved.</h3>
        <p className="quiet">Saved conversations by provider · workspace-wide</p>
        {providers.length ? (
          <div className="provider-chart">
            {providers.map((item) => (
              <div key={item.label}>
                <div>
                  <span>{item.label}</span>
                  <strong>{item.count}</strong>
                </div>
                <meter
                  min={0}
                  max={conversations.length}
                  value={item.count}
                  aria-label={`${item.label}: ${item.count} of ${conversations.length} saved conversations`}
                />
              </div>
            ))}
          </div>
        ) : (
          <div className="insight-empty">
            <Icon name="import" />
            <p>
              {loaded
                ? "Your first conversation starts the picture."
                : "Loading saved conversations…"}
            </p>
            <a href="#import">
              Import context <Icon name="arrow" />
            </a>
          </div>
        )}
      </article>
    </div>
  );
});
