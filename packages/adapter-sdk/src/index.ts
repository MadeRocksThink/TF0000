export type ProviderId = "chatgpt" | "claude" | "gemini" | (string & {});
export type MessageRole = "user" | "assistant" | "system" | "tool" | "unknown";

export interface ConversationRef {
  provider: ProviderId;
  externalRef?: string;
  title: string;
  url?: string;
}

export interface CapturedMessage {
  externalRef?: string;
  role: MessageRole;
  speaker?: string;
  body: string;
  ordinal: number;
  sentAt?: string;
  sourceHash: string;
}

export interface CapturedFragment {
  messageExternalRef?: string;
  startOffset?: number;
  endOffset?: number;
  selectedText: string;
  sourceHash: string;
}

export interface CapturedConversation extends ConversationRef {
  capturedAt: string;
  messages: CapturedMessage[];
}

export interface ConversationMetadata extends ConversationRef {
  messageCount?: number;
}

export interface InsertResult {
  inserted: boolean;
  method: "composer" | "clipboard";
  detail?: string;
}

export interface AdapterHealth {
  status: "healthy" | "degraded" | "unavailable";
  adapterVersion: string;
  detail?: string;
}

export interface ChatProviderAdapter {
  detectConversation(): Promise<ConversationRef | null>;
  captureVisibleConversation(): Promise<CapturedConversation>;
  captureSelection(): Promise<CapturedFragment[]>;
  readComposer(): Promise<string | null>;
  insertContext(text: string): Promise<InsertResult>;
  getConversationMetadata(): Promise<ConversationMetadata>;
  healthCheck(): Promise<AdapterHealth>;
}

export const ADAPTER_API_VERSION = "1.0" as const;

export interface AdapterCapabilities {
  captureConversation: boolean;
  captureSelection: boolean;
  composerRead: boolean;
  composerInsert: boolean;
  clipboardFallback: boolean;
}

export interface AdapterManifest {
  apiVersion: typeof ADAPTER_API_VERSION;
  id: string;
  displayName: string;
  version: string;
  hostnames: readonly string[];
  capabilities: AdapterCapabilities;
}

export interface AdapterRegistration {
  manifest: AdapterManifest;
  create(document: Document): ChatProviderAdapter;
}

export function defineAdapter(registration: AdapterRegistration): AdapterRegistration {
  const { manifest } = registration;
  if (manifest.apiVersion !== ADAPTER_API_VERSION)
    throw new Error("Unsupported adapter API version");
  if (!/^[a-z0-9][a-z0-9-]{1,62}$/.test(manifest.id)) throw new Error("Invalid adapter id");
  if (!manifest.displayName.trim() || !/^\d+\.\d+\.\d+$/.test(manifest.version)) {
    throw new Error("Adapter name and semantic version are required");
  }
  if (manifest.hostnames.length === 0 || manifest.hostnames.some((host) => !host.trim())) {
    throw new Error("At least one hostname is required");
  }
  return Object.freeze(registration);
}

export interface AdapterFixtureExpectation {
  provider: ProviderId;
  title: string;
  messages: ReadonlyArray<Pick<CapturedMessage, "role" | "body">>;
}

export interface AdapterConformanceResult {
  passed: boolean;
  checks: readonly string[];
}

/** Runs deterministic, network-free checks shared by every provider fixture. */
export async function verifyAdapterFixture(
  adapter: ChatProviderAdapter,
  expected: AdapterFixtureExpectation,
): Promise<AdapterConformanceResult> {
  const reference = await adapter.detectConversation();
  if (!reference) throw new Error("Adapter did not detect the fixture conversation");
  if (reference.provider !== expected.provider)
    throw new Error("Provider id does not match fixture");
  if (reference.title !== expected.title)
    throw new Error("Conversation title does not match fixture");
  const captured = await adapter.captureVisibleConversation();
  if (captured.messages.length !== expected.messages.length) {
    throw new Error("Captured message count does not match fixture");
  }
  captured.messages.forEach((message, index) => {
    const wanted = expected.messages[index];
    if (!wanted || message.role !== wanted.role || message.body !== wanted.body) {
      throw new Error(`Message ${index} does not match fixture`);
    }
    if (!/^[a-f0-9]{64}$/.test(message.sourceHash))
      throw new Error(`Message ${index} hash is invalid`);
  });
  const health = await adapter.healthCheck();
  if (health.status === "unavailable") throw new Error("Fixture adapter reported unavailable");
  return {
    passed: true,
    checks: ["detection", "normalized capture", "source hashes", "health"],
  };
}
