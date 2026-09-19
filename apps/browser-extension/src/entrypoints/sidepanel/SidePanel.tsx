import type { AdapterHealth, InsertResult } from "@tf0000/adapter-sdk";
import { useCallback, useEffect, useMemo, useState } from "react";
import { browser } from "wxt/browser";

import { sha256 } from "../../lib/dom-provider-adapter";
import type {
  ActiveTabInfo,
  CapturedSource,
  CapturePayload,
  CaptureResult,
  ComposedContext,
  ContextBinding,
  ContextPackDetail,
  ContextPackItem,
  ConversationContextState,
  ConversationIdentity,
  DashboardSnapshot,
  ExtensionRequest,
  HandoffInput,
  HandoffMode,
  HandoffPreview,
  OpenHandoffResult,
  SecretWarning,
  SourceReference,
  TemporaryAttachment,
} from "../../lib/types";

type CaptureMode = "selection" | "messages" | "chat";
type Provider = "chatgpt" | "claude" | "gemini";

const providers: Array<{ id: Provider; name: string }> = [
  { id: "chatgpt", name: "ChatGPT" },
  { id: "claude", name: "Claude" },
  { id: "gemini", name: "Gemini" },
];

interface ContextPreset {
  name: string;
  memoryIds: string[];
  memorySpaceIds: string[];
  contextPackIds: string[];
  sources: SourceReference[];
}

const emptyDashboard: DashboardSnapshot = {
  memories: [],
  memorySpaces: [],
  contextPacks: [],
  memorySpaceLinks: [],
};

export function SidePanel() {
  const [dashboard, setDashboard] = useState<DashboardSnapshot>(emptyDashboard);
  const [sources, setSources] = useState<CapturedSource[]>([]);
  const [packs, setPacks] = useState<ContextPackDetail[]>([]);
  const [selectedMemoryIds, setSelectedMemoryIds] = useState<string[]>([]);
  const [selectedSpaceIds, setSelectedSpaceIds] = useState<string[]>([]);
  const [selectedPackIds, setSelectedPackIds] = useState<string[]>([]);
  const [selectedSources, setSelectedSources] = useState<SourceReference[]>([]);
  const [conversation, setConversation] = useState<ConversationIdentity | null>(null);
  const [conversationContext, setConversationContext] = useState<ConversationContextState | null>(
    null,
  );
  const [temporaryTarget, setTemporaryTarget] = useState("");
  const [temporaryLifetime, setTemporaryLifetime] =
    useState<TemporaryAttachment["lifetimeMode"]>("one_prompt");
  const [temporaryCount, setTemporaryCount] = useState(3);
  const [packName, setPackName] = useState("");
  const [packMemoryId, setPackMemoryId] = useState("");
  const [packItems, setPackItems] = useState<ContextPackItem[]>([]);
  const [presets, setPresets] = useState<ContextPreset[]>([]);
  const [presetName, setPresetName] = useState("");
  const [preview, setPreview] = useState<ComposedContext | null>(null);
  const [handoffDestination, setHandoffDestination] = useState<Provider>("claude");
  const [handoffMode, setHandoffMode] = useState<HandoffMode>("minimal");
  const [handoffRecentCount, setHandoffRecentCount] = useState(8);
  const [handoffIncludeTask, setHandoffIncludeTask] = useState(true);
  const [handoffIncludeDecisions, setHandoffIncludeDecisions] = useState(true);
  const [handoffPreview, setHandoffPreview] = useState<HandoffPreview | null>(null);
  const [handoffRequest, setHandoffRequest] = useState<HandoffInput | null>(null);
  const [nativeStatus, setNativeStatus] = useState("Checking local bridge…");
  const [activeTab, setActiveTab] = useState<ActiveTabInfo | null>(null);
  const [adapterHealth, setAdapterHealth] = useState<AdapterHealth | null>(null);
  const [manualTitle, setManualTitle] = useState("");
  const [manualUrl, setManualUrl] = useState("");
  const [manualText, setManualText] = useState("");
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);

  const selectedMemorySet = useMemo(() => new Set(selectedMemoryIds), [selectedMemoryIds]);
  const selectedSourceSet = useMemo(
    () => new Set(selectedSources.map((source) => sourceKey(source))),
    [selectedSources],
  );
  const selectedSpaceSet = useMemo(() => new Set(selectedSpaceIds), [selectedSpaceIds]);
  const selectedPackSet = useMemo(() => new Set(selectedPackIds), [selectedPackIds]);
  const activeProvider = providers.find((provider) => provider.id === activeTab?.provider);

  const refresh = useCallback(async () => {
    try {
      const [health, snapshot, capturedSources, packDetails, tab, stored] = await Promise.all([
        send<unknown>({ type: "native", action: "health" }),
        send<DashboardSnapshot>({ type: "native", action: "dashboard" }),
        send<CapturedSource[]>({ type: "native", action: "sources" }),
        send<ContextPackDetail[]>({ type: "native", action: "packs" }),
        send<ActiveTabInfo>({ type: "activeTab" }),
        browser.storage.local.get("tf0000Presets"),
      ]);
      setNativeStatus(health ? "Local bridge connected" : "Local bridge unavailable");
      setDashboard(snapshot);
      setSources(capturedSources);
      setPacks(packDetails);
      setPresets(Array.isArray(stored.tf0000Presets) ? stored.tf0000Presets : []);
      setActiveTab(tab);
      setManualTitle((current) => current || tab.title);
      setManualUrl((current) => current || tab.url);
      if (tab.provider === "manual") {
        setAdapterHealth(null);
        setConversation(null);
        setConversationContext(null);
      } else {
        try {
          const [nextHealth, metadata] = await Promise.all([
            send<AdapterHealth>({ type: "page", request: { type: "adapterHealth" } }),
            send<ConversationIdentity>({ type: "page", request: { type: "metadata" } }),
          ]);
          setAdapterHealth(nextHealth);
          setConversation(metadata);
          if (metadata.externalRef) {
            setConversationContext(
              await send<ConversationContextState | null>({
                type: "native",
                action: "conversationContextByRef",
                payload: { provider: metadata.provider, externalRef: metadata.externalRef },
              }),
            );
          }
        } catch (error) {
          setAdapterHealth({
            status: "degraded",
            adapterVersion: "0.2.0",
            detail: `Reload the ${providerLabel(tab.provider)} page: ${errorMessage(error)}`,
          });
        }
      }
    } catch (error) {
      setNativeStatus(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    setHandoffPreview(null);
    setHandoffRequest(null);
    if (activeProvider?.id === handoffDestination) {
      const destination = providers.find((provider) => provider.id !== activeProvider.id);
      if (destination) setHandoffDestination(destination.id);
    }
  }, [activeProvider, handoffDestination]);

  function invalidateHandoff(): void {
    setHandoffPreview(null);
    setHandoffRequest(null);
  }

  async function capture(mode: CaptureMode): Promise<void> {
    await run(async () => {
      if (!activeProvider) throw new Error("Open ChatGPT, Claude, or Gemini to capture a chat");
      const payload = await send<CapturePayload>({
        type: "page",
        request: { type: "capture", mode },
      });
      if (mode === "selection" && payload.fragments.length === 0) {
        throw new Error(`Select text inside a ${activeProvider.name} message first`);
      }
      if (payload.messages.length === 0) throw new Error("No visible messages were detected");
      if (
        !(await confirmSecretText(
          [
            ...payload.messages.map((message) => message.body),
            ...payload.fragments.map((fragment) => fragment.selectedText),
          ].join("\n"),
          "Save this captured context",
        ))
      ) {
        return;
      }
      const result = await send<CaptureResult>({ type: "native", action: "capture", payload });
      setSources(await send<CapturedSource[]>({ type: "native", action: "sources" }));
      setConversationContext(
        (current) =>
          current ?? {
            conversationId: result.conversationId,
            bindings: [],
            temporaryAttachments: [],
          },
      );
      setNotice(captureNotice(result, payload.provider));
    });
  }

  async function ensureConversationContext(): Promise<ConversationContextState> {
    if (conversationContext) return conversationContext;
    if (!activeProvider) throw new Error("Open a supported AI conversation first");
    const payload = await send<CapturePayload>({
      type: "page",
      request: { type: "capture", mode: "chat" },
    });
    if (!(await confirmSecretText(captureText(payload), "Save this conversation context"))) {
      throw new Error("Conversation capture cancelled");
    }
    const result = await send<CaptureResult>({ type: "native", action: "capture", payload });
    const state: ConversationContextState = {
      conversationId: result.conversationId,
      bindings: [],
      temporaryAttachments: [],
    };
    setConversationContext(state);
    setConversation(payload);
    return state;
  }

  async function toggleBinding(
    targetType: ContextBinding["targetType"],
    targetId: string,
    enabled: boolean,
  ): Promise<void> {
    await run(async () => {
      const state = await ensureConversationContext();
      setConversationContext(
        await send<ConversationContextState>({
          type: "native",
          action: "setBinding",
          payload: {
            conversationId: state.conversationId,
            targetType,
            targetId,
            enabled,
            lifetimeMode: "conversation",
          },
        }),
      );
      setNotice(`${enabled ? "Enabled" : "Disabled"} conversation context`);
    });
  }

  async function attachTemporary(): Promise<void> {
    await run(async () => {
      const [targetType, targetId] = temporaryTarget.split(":", 2) as [
        TemporaryAttachment["targetType"],
        string,
      ];
      if (!targetType || !targetId) throw new Error("Choose context to attach temporarily");
      const state = await ensureConversationContext();
      setConversationContext(
        await send<ConversationContextState>({
          type: "native",
          action: "attachTemporary",
          payload: {
            conversationId: state.conversationId,
            targetType,
            targetId,
            lifetimeMode: temporaryLifetime,
            remainingPrompts: temporaryLifetime === "n_prompts" ? temporaryCount : null,
            expiresAt: null,
          },
        }),
      );
      setNotice("Temporary context attached");
    });
  }

  async function clearTemporary(): Promise<void> {
    if (!conversationContext) return;
    await run(async () => {
      setConversationContext(
        await send<ConversationContextState>({
          type: "native",
          action: "clearTemporary",
          payload: { conversationId: conversationContext.conversationId },
        }),
      );
      setNotice("Temporary context cleared");
    });
  }

  async function importManual(): Promise<void> {
    await run(async () => {
      const title = manualTitle.trim();
      const body = manualText.trim();
      if (!title || !body) throw new Error("Manual title and text are required");
      if (!(await confirmSecretText(body, "Save this manual context"))) return;
      const sourceHash = await sha256(`unknown\n${body}`);
      const payload: CapturePayload = {
        provider: "manual",
        externalRef: manualUrl.trim() || crypto.randomUUID(),
        title,
        ...(manualUrl.trim() ? { url: manualUrl.trim() } : {}),
        capturedAt: new Date().toISOString(),
        messages: [
          {
            externalRef: `manual-${sourceHash.slice(0, 16)}`,
            role: "unknown",
            body,
            ordinal: 0,
            sourceHash,
          },
        ],
        fragments: [],
      };
      await send<CaptureResult>({ type: "native", action: "capture", payload });
      setSources(await send<CapturedSource[]>({ type: "native", action: "sources" }));
      setManualText("");
      setNotice("Manual source saved locally");
    });
  }

  async function buildPreview(): Promise<void> {
    await run(async () => {
      const selectedCount =
        selectedMemoryIds.length +
        selectedSpaceIds.length +
        selectedPackIds.length +
        selectedSources.length;
      const conversationCount = conversationContext
        ? conversationContext.bindings.filter((binding) => binding.enabled).length +
          conversationContext.temporaryAttachments.length
        : 0;
      if (selectedCount + conversationCount === 0) {
        throw new Error("Select at least one memory or captured source");
      }
      const result = await send<ComposedContext>({
        type: "native",
        action: "compose",
        payload: {
          memoryIds: selectedMemoryIds,
          memorySpaceIds: selectedSpaceIds,
          contextPackIds: selectedPackIds,
          sources: selectedSources,
          conversationId: conversationContext?.conversationId ?? null,
        },
      });
      setPreview(result);
      setNotice("Cross-provider context preview is ready");
    });
  }

  async function insert(): Promise<void> {
    await run(async () => {
      if (!preview?.text) throw new Error("Build a context preview first");
      if (!(await confirmSecretText(preview.text, "Attach this context"))) return;
      if (!activeProvider) {
        await navigator.clipboard.writeText(preview.text);
        setNotice("Unsupported page: context copied to clipboard");
        return;
      }
      const result = await send<InsertResult>({
        type: "page",
        request: { type: "insert", text: preview.text },
      });
      setNotice(
        result.inserted
          ? `Inserted into ${activeProvider.name}. Review it, then send when ready.`
          : (result.detail ?? "Copied to clipboard"),
      );
      if (result.inserted && conversationContext) {
        setConversationContext(
          await send<ConversationContextState>({
            type: "native",
            action: "consumePrompt",
            payload: { conversationId: conversationContext.conversationId },
          }),
        );
      }
    });
  }

  async function buildHandoff(): Promise<void> {
    await run(async () => {
      if (!activeProvider) throw new Error("Open ChatGPT, Claude, or Gemini to continue a chat");
      const payload = await send<CapturePayload>({
        type: "page",
        request: { type: "capture", mode: "chat" },
      });
      if (!payload.externalRef) throw new Error("The current conversation has no stable reference");
      if (payload.messages.length === 0) throw new Error("No visible messages were detected");
      if (
        !(await confirmSecretText(captureText(payload), "Save this conversation handoff source"))
      ) {
        return;
      }
      const result = await send<CaptureResult>({ type: "native", action: "capture", payload });
      setConversation(payload);
      setConversationContext(
        (current) =>
          current ?? {
            conversationId: result.conversationId,
            bindings: [],
            temporaryAttachments: [],
          },
      );
      setSources(await send<CapturedSource[]>({ type: "native", action: "sources" }));
      const request: HandoffInput = {
        sourceProvider: activeProvider.id,
        sourceExternalRef: payload.externalRef,
        destinationProvider: handoffDestination,
        mode: handoffMode,
        recentMessageCount: handoffMode === "custom" ? handoffRecentCount : null,
        includeCurrentTask: handoffMode === "custom" ? handoffIncludeTask : true,
        includeActiveDecisions: handoffMode === "custom" ? handoffIncludeDecisions : true,
        memoryIds: selectedMemoryIds,
        contextPackIds: selectedPackIds,
      };
      setHandoffRequest(request);
      setHandoffPreview(
        await send<HandoffPreview>({
          type: "native",
          action: "previewHandoff",
          payload: request,
        }),
      );
      setNotice("Exact handoff preview is ready");
    });
  }

  async function continueHandoff(): Promise<void> {
    await run(async () => {
      if (!handoffPreview?.text || !handoffRequest) {
        throw new Error("Build and review a handoff preview first");
      }
      if (!(await confirmSecretText(handoffPreview.text, "Open this handoff"))) return;
      try {
        await navigator.clipboard.writeText(handoffPreview.text);
      } catch {
        // Direct insertion can still succeed when clipboard permission is unavailable.
      }
      const result = await send<OpenHandoffResult>({
        type: "openHandoff",
        destinationProvider: handoffRequest.destinationProvider,
        text: handoffPreview.text,
      });
      await send({ type: "native", action: "recordHandoff", payload: handoffRequest });
      const destination = providerLabel(handoffRequest.destinationProvider);
      setNotice(
        result.inserted
          ? `Opened ${destination} and inserted the reviewed handoff. Nothing was sent.`
          : `Opened ${destination}. Paste the handoff from your clipboard; nothing was sent.`,
      );
    });
  }

  function addPackItem(): void {
    if (!packMemoryId || packItems.some((item) => item.targetId === packMemoryId)) return;
    setPackItems((current) => [
      ...current,
      { targetId: packMemoryId, ordering: current.length, inclusionMode: "full" },
    ]);
    setPackMemoryId("");
  }

  function movePackItem(index: number, direction: -1 | 1): void {
    setPackItems((current) => {
      const destination = index + direction;
      if (destination < 0 || destination >= current.length) return current;
      const next = [...current];
      const [item] = next.splice(index, 1);
      if (!item) return current;
      next.splice(destination, 0, item);
      return next.map((entry, ordering) => ({ ...entry, ordering }));
    });
  }

  async function savePack(): Promise<void> {
    await run(async () => {
      if (!packName.trim() || packItems.length === 0) {
        throw new Error("Pack name and at least one memory are required");
      }
      await send<ContextPackDetail>({
        type: "native",
        action: "savePack",
        payload: {
          id: null,
          projectId: null,
          name: packName.trim(),
          description: "Created in the TF0000 browser panel",
          items: packItems,
        },
      });
      const [nextPacks, nextDashboard] = await Promise.all([
        send<ContextPackDetail[]>({ type: "native", action: "packs" }),
        send<DashboardSnapshot>({ type: "native", action: "dashboard" }),
      ]);
      setPacks(nextPacks);
      setDashboard(nextDashboard);
      setPackName("");
      setPackItems([]);
      setNotice("Reusable context pack saved");
    });
  }

  async function savePreset(): Promise<void> {
    const name = presetName.trim();
    if (!name) {
      setNotice("Preset name is required");
      return;
    }
    const preset: ContextPreset = {
      name,
      memoryIds: selectedMemoryIds,
      memorySpaceIds: selectedSpaceIds,
      contextPackIds: selectedPackIds,
      sources: selectedSources,
    };
    const next = [...presets.filter((item) => item.name !== name), preset];
    await browser.storage.local.set({ tf0000Presets: next });
    setPresets(next);
    setPresetName("");
    setNotice("Context preset saved");
  }

  function applyPreset(preset: ContextPreset): void {
    invalidateHandoff();
    setSelectedMemoryIds(preset.memoryIds);
    setSelectedSpaceIds(preset.memorySpaceIds);
    setSelectedPackIds(preset.contextPackIds);
    setSelectedSources(preset.sources);
    setNotice(`Applied preset: ${preset.name}`);
  }

  async function copyPreview(): Promise<void> {
    if (!preview?.text) return;
    await navigator.clipboard.writeText(preview.text);
    setNotice("Context copied to clipboard");
  }

  async function confirmSecretText(text: string, action: string): Promise<boolean> {
    const warnings = await send<SecretWarning[]>({
      type: "native",
      action: "detectSecrets",
      payload: { text },
    });
    if (warnings.length === 0) return true;
    const kinds = [...new Set(warnings.map((warning) => warning.kind))].join(", ");
    return window.confirm(
      `TF0000 detected ${warnings.length} possible secret${warnings.length === 1 ? "" : "s"} (${kinds}). ${action} anyway?`,
    );
  }

  async function run(operation: () => Promise<void>): Promise<void> {
    setBusy(true);
    setNotice("");
    try {
      await operation();
    } catch (error) {
      setNotice(errorMessage(error));
    } finally {
      setBusy(false);
    }
  }

  return (
    <main>
      <header>
        <p className="eyebrow">LOCAL-FIRST MEMORY</p>
        <h1>TF0000</h1>
        <p className="subtle">Capture in one AI. Attach deliberately in another.</p>
      </header>

      <section className="health" aria-label="Integration health">
        <div className="section-heading">
          <div>
            <p className="eyebrow">INTEGRATIONS</p>
            <h2>Health</h2>
          </div>
          <button className="quiet" type="button" onClick={() => void refresh()} disabled={busy}>
            Run diagnostics
          </button>
        </div>
        <Status label="Storage" value={nativeStatus} status="healthy" />
        {providers.map((provider) => {
          const isActive = provider.id === activeTab?.provider;
          return (
            <Status
              key={provider.id}
              label={provider.name}
              value={
                isActive
                  ? (adapterHealth?.detail ?? "Checking active integration…")
                  : `Not checked — open ${provider.name} to diagnose DOM`
              }
              status={isActive ? (adapterHealth?.status ?? "degraded") : "idle"}
            />
          );
        })}
      </section>

      {activeProvider && (
        <section>
          <div className="section-heading">
            <div>
              <p className="eyebrow">PER-CHAT CONTEXT</p>
              <h2>{conversation?.title ?? "Current conversation"}</h2>
            </div>
            <span>{conversationContext ? "Ready" : "Not initialized"}</span>
          </div>
          <details>
            <summary>ON / OFF memory spaces</summary>
            <div className="toggle-list">
              {dashboard.memorySpaces.map((space) => (
                <label className="toggle-row" key={space.id}>
                  <span>{space.name}</span>
                  <input
                    type="checkbox"
                    checked={isBindingEnabled(conversationContext, "memory_space", space.id)}
                    onChange={(event) =>
                      void toggleBinding("memory_space", space.id, event.target.checked)
                    }
                  />
                </label>
              ))}
            </div>
          </details>
          <details>
            <summary>ON / OFF individual memories</summary>
            <div className="toggle-list">
              {dashboard.memories
                .filter((memory) => memory.status === "active")
                .map((memory) => (
                  <label className="toggle-row" key={memory.id}>
                    <span>{memory.title}</span>
                    <input
                      type="checkbox"
                      checked={isBindingEnabled(conversationContext, "memory", memory.id)}
                      onChange={(event) =>
                        void toggleBinding("memory", memory.id, event.target.checked)
                      }
                    />
                  </label>
                ))}
            </div>
          </details>
          <div className="temporary-builder">
            <select
              value={temporaryTarget}
              onChange={(event) => setTemporaryTarget(event.target.value)}
              aria-label="Temporary context target"
            >
              <option value="">Choose temporary context…</option>
              {dashboard.memorySpaces.map((space) => (
                <option key={`space:${space.id}`} value={`memory_space:${space.id}`}>
                  Space · {space.name}
                </option>
              ))}
              {dashboard.memories
                .filter((memory) => memory.status === "active")
                .map((memory) => (
                  <option key={`memory:${memory.id}`} value={`memory:${memory.id}`}>
                    Memory · {memory.title}
                  </option>
                ))}
              {packs.map(({ pack }) => (
                <option key={`pack:${pack.id}`} value={`context_pack:${pack.id}`}>
                  Pack · {pack.name}
                </option>
              ))}
            </select>
            <select
              value={temporaryLifetime}
              onChange={(event) =>
                setTemporaryLifetime(event.target.value as TemporaryAttachment["lifetimeMode"])
              }
              aria-label="Temporary context lifetime"
            >
              <option value="one_prompt">One prompt</option>
              <option value="n_prompts">N prompts</option>
              <option value="conversation">Conversation</option>
              <option value="manual">Until manually cleared</option>
            </select>
            {temporaryLifetime === "n_prompts" && (
              <input
                type="number"
                min={1}
                max={100}
                value={temporaryCount}
                onChange={(event) => setTemporaryCount(Math.max(1, Number(event.target.value)))}
                aria-label="Remaining prompts"
              />
            )}
            <button type="button" onClick={() => void attachTemporary()} disabled={busy}>
              Attach temporarily
            </button>
          </div>
          <div className="temporary-list">
            {conversationContext?.temporaryAttachments.map((attachment) => (
              <div className="temporary-item" key={attachment.id}>
                <span>{targetLabel(attachment, dashboard)}</span>
                <strong>
                  {attachment.remainingPrompts === null
                    ? lifetimeLabel(attachment.lifetimeMode)
                    : `${attachment.remainingPrompts} prompt${attachment.remainingPrompts === 1 ? "" : "s"} left`}
                </strong>
              </div>
            ))}
          </div>
          {Boolean(conversationContext?.temporaryAttachments.length) && (
            <button className="secondary" type="button" onClick={() => void clearTemporary()}>
              Clear temporary context
            </button>
          )}
        </section>
      )}

      <section>
        <div className="section-heading">
          <div>
            <p className="eyebrow">CAPTURE</p>
            <h2>{activeProvider?.name ?? "Manual fallback"}</h2>
          </div>
          {activeProvider && (
            <span className={`badge ${activeProvider.id}`}>{activeProvider.name}</span>
          )}
        </div>
        {activeProvider ? (
          <div className="capture-grid">
            <button type="button" onClick={() => void capture("selection")} disabled={busy}>
              Save selection
            </button>
            <button type="button" onClick={() => void capture("messages")} disabled={busy}>
              Save messages
            </button>
            <button type="button" onClick={() => void capture("chat")} disabled={busy}>
              Save visible chat
            </button>
          </div>
        ) : (
          <p className="empty">This page has no provider adapter. Paste exact material below.</p>
        )}
        <details className="manual" open={!activeProvider}>
          <summary>Manual capture / import</summary>
          <input
            value={manualTitle}
            onChange={(event) => setManualTitle(event.target.value)}
            placeholder="Source title"
            aria-label="Manual source title"
          />
          <input
            value={manualUrl}
            onChange={(event) => setManualUrl(event.target.value)}
            placeholder="Source URL (optional)"
            aria-label="Manual source URL"
          />
          <textarea
            value={manualText}
            onChange={(event) => setManualText(event.target.value)}
            placeholder="Paste exact text to save"
            aria-label="Manual source text"
          />
          <button type="button" onClick={() => void importManual()} disabled={busy}>
            Save manual source
          </button>
        </details>
      </section>

      <section>
        <div className="section-heading">
          <div>
            <p className="eyebrow">SOURCE LIBRARY</p>
            <h2>Captured context</h2>
          </div>
          <span>{selectedSources.length} selected</span>
        </div>
        <div className="source-list">
          {sources.length === 0 ? (
            <p className="empty">Saved selections and chats will appear here.</p>
          ) : (
            sources.map((source) => (
              <label className="source" key={sourceKey(source)}>
                <input
                  type="checkbox"
                  checked={selectedSourceSet.has(sourceKey(source))}
                  onChange={(event) =>
                    setSelectedSources((current) =>
                      event.target.checked
                        ? [...current, { id: source.id, sourceType: source.sourceType }]
                        : current.filter((item) => sourceKey(item) !== sourceKey(source)),
                    )
                  }
                />
                <span className="source-body">
                  <span className="source-meta">
                    <span className={`badge ${source.provider}`}>
                      {providerLabel(source.provider)}
                    </span>
                    <small>{source.role}</small>
                    {source.sourceUrl && (
                      <a href={source.sourceUrl} target="_blank" rel="noreferrer">
                        source ↗
                      </a>
                    )}
                  </span>
                  <strong>{source.conversationTitle}</strong>
                  <small className="excerpt">{source.content}</small>
                </span>
              </label>
            ))
          )}
        </div>
      </section>

      <section>
        <div className="section-heading">
          <div>
            <p className="eyebrow">MEMORIES</p>
            <h2>Active memories</h2>
          </div>
          <span>{selectedMemoryIds.length} selected</span>
        </div>
        <div className="memory-list">
          {dashboard.memories.filter((memory) => memory.status === "active").length === 0 ? (
            <p className="empty">Create memories in the desktop app, then refresh.</p>
          ) : (
            dashboard.memories
              .filter((memory) => memory.status === "active")
              .map((memory) => (
                <label className="memory" key={memory.id}>
                  <input
                    type="checkbox"
                    checked={selectedMemorySet.has(memory.id)}
                    onChange={(event) => {
                      invalidateHandoff();
                      setSelectedMemoryIds((current) =>
                        event.target.checked
                          ? [...current, memory.id]
                          : current.filter((id) => id !== memory.id),
                      );
                    }}
                  />
                  <span>
                    <strong>{memory.title}</strong>
                    <small>{memory.memoryType.replaceAll("_", " ")}</small>
                  </span>
                </label>
              ))
          )}
        </div>
        <details>
          <summary>Attach memory spaces</summary>
          <div className="toggle-list">
            {dashboard.memorySpaces.map((space) => (
              <label className="toggle-row" key={space.id}>
                <span>{space.name}</span>
                <input
                  type="checkbox"
                  checked={selectedSpaceSet.has(space.id)}
                  onChange={(event) =>
                    setSelectedSpaceIds((current) =>
                      event.target.checked
                        ? [...current, space.id]
                        : current.filter((id) => id !== space.id),
                    )
                  }
                />
              </label>
            ))}
          </div>
        </details>
        <details>
          <summary>Attach reusable packs</summary>
          <div className="toggle-list">
            {packs.map(({ pack, items }) => (
              <label className="toggle-row" key={pack.id}>
                <span>
                  {pack.name} <small>({items.length} items)</small>
                </span>
                <input
                  type="checkbox"
                  checked={selectedPackSet.has(pack.id)}
                  onChange={(event) => {
                    invalidateHandoff();
                    setSelectedPackIds((current) =>
                      event.target.checked
                        ? [...current, pack.id]
                        : current.filter((id) => id !== pack.id),
                    );
                  }}
                />
              </label>
            ))}
          </div>
        </details>
        <div className="preset-row">
          <select
            defaultValue=""
            onChange={(event) => {
              const preset = presets.find((item) => item.name === event.target.value);
              if (preset) applyPreset(preset);
              event.target.value = "";
            }}
            aria-label="Apply context preset"
          >
            <option value="">Apply preset…</option>
            {presets.map((preset) => (
              <option key={preset.name} value={preset.name}>
                {preset.name}
              </option>
            ))}
          </select>
          <input
            value={presetName}
            onChange={(event) => setPresetName(event.target.value)}
            placeholder="New preset name"
            aria-label="New preset name"
          />
          <button className="secondary" type="button" onClick={() => void savePreset()}>
            Save preset
          </button>
        </div>
        <button
          type="button"
          onClick={() => void buildPreview()}
          disabled={
            busy ||
            (selectedMemoryIds.length +
              selectedSpaceIds.length +
              selectedPackIds.length +
              selectedSources.length ===
              0 &&
              !conversationContext)
          }
        >
          Build cross-provider preview
        </button>
      </section>

      {activeProvider && (
        <section>
          <div className="section-heading">
            <div>
              <p className="eyebrow">CONTINUE IN…</p>
              <h2>Conversation handoff</h2>
            </div>
            <span>{selectedMemoryIds.length + selectedPackIds.length} selected extras</span>
          </div>
          <div className="handoff-grid">
            <label>
              Destination
              <select
                value={handoffDestination}
                onChange={(event) => {
                  invalidateHandoff();
                  setHandoffDestination(event.target.value as Provider);
                }}
                aria-label="Handoff destination"
              >
                {providers
                  .filter((provider) => provider.id !== activeProvider.id)
                  .map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name}
                    </option>
                  ))}
              </select>
            </label>
            <label>
              Mode
              <select
                value={handoffMode}
                onChange={(event) => {
                  invalidateHandoff();
                  setHandoffMode(event.target.value as HandoffMode);
                }}
                aria-label="Handoff mode"
              >
                <option value="full">Full</option>
                <option value="minimal">Minimal</option>
                <option value="custom">Custom</option>
              </select>
            </label>
          </div>
          <p className="safety">
            {handoffMode === "full"
              ? "All captured messages, current tasks, active decisions, and selected extras."
              : handoffMode === "minimal"
                ? "The latest four messages, current tasks, and selected extras."
                : "Choose the exact recent-message window and memory groups below."}
          </p>
          {handoffMode === "custom" && (
            <div className="handoff-options">
              <label>
                Recent messages
                <input
                  type="number"
                  min={1}
                  max={100}
                  value={handoffRecentCount}
                  onChange={(event) => {
                    invalidateHandoff();
                    setHandoffRecentCount(
                      Math.min(100, Math.max(1, Number(event.target.value) || 1)),
                    );
                  }}
                />
              </label>
              <label className="toggle-row">
                <span>Current tasks</span>
                <input
                  type="checkbox"
                  checked={handoffIncludeTask}
                  onChange={(event) => {
                    invalidateHandoff();
                    setHandoffIncludeTask(event.target.checked);
                  }}
                />
              </label>
              <label className="toggle-row">
                <span>Active decisions</span>
                <input
                  type="checkbox"
                  checked={handoffIncludeDecisions}
                  onChange={(event) => {
                    invalidateHandoff();
                    setHandoffIncludeDecisions(event.target.checked);
                  }}
                />
              </label>
            </div>
          )}
          <button type="button" onClick={() => void buildHandoff()} disabled={busy}>
            Build exact handoff preview
          </button>
          {handoffPreview && (
            <div className="handoff-preview">
              <div className="section-heading">
                <strong>Exact outgoing text</strong>
                <span>
                  {handoffPreview.characterCount.toLocaleString()} chars · ≈{" "}
                  {handoffPreview.estimatedTokens.toLocaleString()} tokens
                </span>
              </div>
              <textarea
                className="preview"
                value={handoffPreview.text}
                readOnly
                aria-label="Handoff preview"
              />
              <button type="button" onClick={() => void continueHandoff()} disabled={busy}>
                Continue in {providerLabel(handoffPreview.destinationProvider)}
              </button>
              <p className="safety">
                Opens a new tab and inserts only this reviewed text. TF0000 never presses Send.
              </p>
            </div>
          )}
        </section>
      )}

      <section>
        <div className="section-heading">
          <div>
            <p className="eyebrow">PACK BUILDER</p>
            <h2>Reusable ordered context</h2>
          </div>
          <span>{packItems.length} items</span>
        </div>
        <input
          className="pack-name"
          value={packName}
          onChange={(event) => setPackName(event.target.value)}
          placeholder="Pack name"
          aria-label="Context pack name"
        />
        <div className="pack-add">
          <select
            value={packMemoryId}
            onChange={(event) => setPackMemoryId(event.target.value)}
            aria-label="Memory to add"
          >
            <option value="">Choose a memory…</option>
            {dashboard.memories
              .filter(
                (memory) =>
                  memory.status === "active" &&
                  !packItems.some((item) => item.targetId === memory.id),
              )
              .map((memory) => (
                <option key={memory.id} value={memory.id}>
                  {memory.title}
                </option>
              ))}
          </select>
          <button className="secondary" type="button" onClick={addPackItem}>
            Add
          </button>
        </div>
        <div className="pack-items">
          {packItems.map((item, index) => (
            <div className="pack-item" key={item.targetId}>
              <span>{memoryTitle(item.targetId, dashboard)}</span>
              <select
                value={item.inclusionMode}
                onChange={(event) =>
                  setPackItems((current) =>
                    current.map((entry) =>
                      entry.targetId === item.targetId
                        ? {
                            ...entry,
                            inclusionMode: event.target.value as ContextPackItem["inclusionMode"],
                          }
                        : entry,
                    ),
                  )
                }
                aria-label={`Inclusion mode for ${memoryTitle(item.targetId, dashboard)}`}
              >
                <option value="full">Full</option>
                <option value="summary">Summary</option>
                <option value="reference">Reference</option>
              </select>
              <button
                className="icon-button"
                type="button"
                onClick={() => movePackItem(index, -1)}
                disabled={index === 0}
                aria-label="Move up"
              >
                ↑
              </button>
              <button
                className="icon-button"
                type="button"
                onClick={() => movePackItem(index, 1)}
                disabled={index === packItems.length - 1}
                aria-label="Move down"
              >
                ↓
              </button>
              <button
                className="icon-button danger"
                type="button"
                onClick={() =>
                  setPackItems((current) =>
                    current
                      .filter((entry) => entry.targetId !== item.targetId)
                      .map((entry, ordering) => ({ ...entry, ordering })),
                  )
                }
                aria-label="Remove item"
              >
                ×
              </button>
            </div>
          ))}
        </div>
        <button type="button" onClick={() => void savePack()} disabled={busy}>
          Save context pack
        </button>
      </section>

      {preview && (
        <section>
          <div className="section-heading">
            <div>
              <p className="eyebrow">PREVIEW</p>
              <h2>Review exact outgoing context</h2>
            </div>
            <span>≈ {preview.estimatedTokens} tokens</span>
          </div>
          <textarea
            className="preview"
            value={preview.text}
            readOnly
            aria-label="Context preview"
          />
          <div className="actions">
            <button type="button" onClick={() => void insert()} disabled={busy}>
              {activeProvider ? `Insert in ${activeProvider.name}` : "Copy for this page"}
            </button>
            <button className="secondary" type="button" onClick={() => void copyPreview()}>
              Copy
            </button>
          </div>
          <p className="safety">Nothing is sent automatically. You always review and send.</p>
        </section>
      )}

      {notice && <output className="notice">{notice}</output>}
    </main>
  );
}

function isBindingEnabled(
  state: ConversationContextState | null,
  targetType: ContextBinding["targetType"],
  targetId: string,
): boolean {
  return Boolean(
    state?.bindings.some(
      (binding) =>
        binding.targetType === targetType && binding.targetId === targetId && binding.enabled,
    ),
  );
}

function targetLabel(attachment: TemporaryAttachment, dashboard: DashboardSnapshot): string {
  if (attachment.targetType === "memory") {
    return memoryTitle(attachment.targetId, dashboard);
  }
  if (attachment.targetType === "memory_space") {
    return (
      dashboard.memorySpaces.find((space) => space.id === attachment.targetId)?.name ??
      "Memory space"
    );
  }
  return (
    dashboard.contextPacks.find((pack) => pack.id === attachment.targetId)?.name ?? "Context pack"
  );
}

function memoryTitle(id: string, dashboard: DashboardSnapshot): string {
  return dashboard.memories.find((memory) => memory.id === id)?.title ?? "Unknown memory";
}

function lifetimeLabel(lifetime: TemporaryAttachment["lifetimeMode"]): string {
  return lifetime === "conversation"
    ? "this conversation"
    : lifetime === "manual"
      ? "until cleared"
      : lifetime.replaceAll("_", " ");
}

function Status({
  label,
  value,
  status,
}: {
  label: string;
  value: string;
  status: "healthy" | "degraded" | "unavailable" | "idle";
}) {
  return (
    <div className="status">
      <i className={status} aria-hidden="true" />
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function sourceKey(source: SourceReference): string {
  return `${source.sourceType}:${source.id}`;
}

function captureText(payload: CapturePayload): string {
  return [
    ...payload.messages.map((message) => message.body),
    ...payload.fragments.map((fragment) => fragment.selectedText),
  ].join("\n");
}

function providerLabel(provider: string): string {
  return (
    providers.find((item) => item.id === provider)?.name ??
    (provider === "manual" ? "Manual" : provider)
  );
}

function captureNotice(result: CaptureResult, provider: string): string {
  return `Saved ${result.savedMessages} ${providerLabel(provider)} message${result.savedMessages === 1 ? "" : "s"}${result.savedFragments ? ` and ${result.savedFragments} fragment${result.savedFragments === 1 ? "" : "s"}` : ""}`;
}

async function send<T>(message: ExtensionRequest): Promise<T> {
  return browser.runtime.sendMessage(message) as Promise<T>;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
