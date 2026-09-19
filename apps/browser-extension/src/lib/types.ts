import type {
  AdapterHealth,
  CapturedConversation,
  CapturedFragment,
  InsertResult,
} from "@tf0000/adapter-sdk";

export interface CapturePayload extends CapturedConversation {
  fragments: CapturedFragment[];
}

export interface MemorySummary {
  id: string;
  title: string;
  currentContent: string;
  memoryType: string;
  authority: string;
  status: string;
}

export interface DashboardSnapshot {
  memories: MemorySummary[];
  memorySpaces: MemorySpaceSummary[];
  contextPacks: ContextPackSummary[];
  memorySpaceLinks: MemorySpaceLink[];
}

export interface MemorySpaceSummary {
  id: string;
  projectId: string | null;
  parentId: string | null;
  name: string;
  defaultScope: string;
}

export interface MemorySpaceLink {
  memoryId: string;
  memorySpaceId: string;
}

export interface ContextPackSummary {
  id: string;
  projectId: string | null;
  name: string;
  description: string;
  currentVersion: number;
}

export interface ContextPackItem {
  targetId: string;
  ordering: number;
  inclusionMode: "full" | "summary" | "reference";
}

export interface ContextPackDetail {
  pack: ContextPackSummary;
  items: ContextPackItem[];
}

export interface ComposedContext {
  text: string;
  itemCount: number;
  characterCount: number;
  estimatedTokens: number;
}

export type HandoffMode = "full" | "minimal" | "custom";

export interface HandoffInput {
  sourceProvider: "chatgpt" | "claude" | "gemini";
  sourceExternalRef: string;
  destinationProvider: "chatgpt" | "claude" | "gemini";
  mode: HandoffMode;
  recentMessageCount: number | null;
  includeCurrentTask: boolean;
  includeActiveDecisions: boolean;
  memoryIds: string[];
  contextPackIds: string[];
}

export interface HandoffItem {
  itemType: string;
  itemId: string;
  label: string;
}

export interface HandoffPreview {
  sourceConversationId: string;
  sourceProvider: string;
  sourceTitle: string;
  destinationProvider: string;
  mode: HandoffMode;
  text: string;
  contentHash: string;
  items: HandoffItem[];
  itemCount: number;
  characterCount: number;
  estimatedTokens: number;
}

export interface ConversationHandoff {
  id: string;
  sourceConversationId: string;
  destinationProvider: string;
  destinationConversationId: string | null;
  mode: HandoffMode;
  contentHash: string;
  itemCount: number;
  characterCount: number;
  estimatedTokens: number;
  createdAt: string;
}

export interface OpenHandoffResult extends InsertResult {
  tabId: number;
}

export interface SecretWarning {
  kind: string;
  line: number;
  redactedExcerpt: string;
}

export interface CaptureResult {
  conversationId: string;
  savedMessages: number;
  savedFragments: number;
}

export interface CapturedSource {
  id: string;
  sourceType: "conversation" | "fragment" | "message";
  provider: string;
  conversationTitle: string;
  role: string;
  content: string;
  sourceUrl: string | null;
  capturedAt: string;
}

export interface SourceReference {
  id: string;
  sourceType: "conversation" | "fragment" | "message";
}

export interface ActiveTabInfo {
  title: string;
  url: string;
  provider: "chatgpt" | "claude" | "gemini" | "manual";
}

export interface ConversationIdentity {
  provider: string;
  externalRef?: string;
  title: string;
  url?: string;
}

export interface ContextBinding {
  targetType: "memory" | "memory_space" | "context_pack";
  targetId: string;
  enabled: boolean;
  lifetimeMode: string;
}

export interface TemporaryAttachment {
  id: string;
  conversationId: string;
  targetType: "memory" | "memory_space" | "context_pack";
  targetId: string;
  lifetimeMode: "one_prompt" | "n_prompts" | "conversation" | "manual";
  remainingPrompts: number | null;
}

export interface ConversationContextState {
  conversationId: string;
  bindings: ContextBinding[];
  temporaryAttachments: TemporaryAttachment[];
}

export type PageRequest =
  | { type: "capture"; mode: "selection" | "messages" | "chat" }
  | { type: "insert"; text: string }
  | { type: "metadata" }
  | { type: "adapterHealth" };

export type PageResponse = CapturePayload | InsertResult | AdapterHealth | ConversationIdentity;

export type ExtensionRequest =
  | { type: "page"; request: PageRequest }
  | {
      type: "native";
      action: "health" | "dashboard" | "sources" | "packs";
      payload?: Record<string, never>;
    }
  | { type: "native"; action: "capture"; payload: CapturePayload }
  | { type: "native"; action: "detectSecrets"; payload: { text: string } }
  | {
      type: "native";
      action: "compose";
      payload: {
        memoryIds: string[];
        memorySpaceIds: string[];
        contextPackIds: string[];
        sources: SourceReference[];
        conversationId: string | null;
      };
    }
  | {
      type: "native";
      action: "previewHandoff" | "recordHandoff";
      payload: HandoffInput;
    }
  | {
      type: "native";
      action: "savePack";
      payload: {
        id: string | null;
        projectId: string | null;
        name: string;
        description: string;
        items: ContextPackItem[];
      };
    }
  | {
      type: "native";
      action: "conversationContextByRef";
      payload: { provider: string; externalRef: string };
    }
  | {
      type: "native";
      action: "setBinding";
      payload: {
        conversationId: string;
        targetType: ContextBinding["targetType"];
        targetId: string;
        enabled: boolean;
        lifetimeMode: "conversation";
      };
    }
  | {
      type: "native";
      action: "attachTemporary";
      payload: {
        conversationId: string;
        targetType: TemporaryAttachment["targetType"];
        targetId: string;
        lifetimeMode: TemporaryAttachment["lifetimeMode"];
        remainingPrompts: number | null;
        expiresAt: string | null;
      };
    }
  | {
      type: "native";
      action: "clearTemporary" | "consumePrompt";
      payload: { conversationId: string };
    }
  | { type: "activeTab" }
  | {
      type: "openHandoff";
      destinationProvider: HandoffInput["destinationProvider"];
      text: string;
    }
  | { type: "openSidePanel" };
