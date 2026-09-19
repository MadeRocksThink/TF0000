import {
  ADAPTER_API_VERSION,
  type ChatProviderAdapter,
  defineAdapter,
  type ProviderId,
} from "@tf0000/adapter-sdk";
import { ChatGptAdapter } from "./chatgpt-adapter";
import { ClaudeAdapter } from "./claude-adapter";
import { GeminiAdapter } from "./gemini-adapter";

const capabilities = {
  captureConversation: true,
  captureSelection: true,
  composerRead: true,
  composerInsert: true,
  clipboardFallback: true,
} as const;

export const adapterRegistrations = [
  defineAdapter({
    manifest: {
      apiVersion: ADAPTER_API_VERSION,
      id: "chatgpt",
      displayName: "ChatGPT",
      version: "1.0.0",
      hostnames: ["chatgpt.com"],
      capabilities,
    },
    create: (document) => new ChatGptAdapter(document),
  }),
  defineAdapter({
    manifest: {
      apiVersion: ADAPTER_API_VERSION,
      id: "claude",
      displayName: "Claude",
      version: "1.0.0",
      hostnames: ["claude.ai"],
      capabilities,
    },
    create: (document) => new ClaudeAdapter(document),
  }),
  defineAdapter({
    manifest: {
      apiVersion: ADAPTER_API_VERSION,
      id: "gemini",
      displayName: "Gemini",
      version: "1.0.0",
      hostnames: ["gemini.google.com"],
      capabilities,
    },
    create: (document) => new GeminiAdapter(document),
  }),
] as const;

export const integrations = adapterRegistrations.map(({ manifest }) => ({
  provider: manifest.id,
  name: manifest.displayName,
  hostname: manifest.hostnames[0] ?? "",
}));

export function createProviderAdapter(document: Document): ChatProviderAdapter | null {
  return (
    adapterRegistrations
      .find(({ manifest }) => manifest.hostnames.includes(document.location.hostname))
      ?.create(document) ?? null
  );
}

export function providerName(provider: ProviderId): string {
  return (
    adapterRegistrations.find(({ manifest }) => manifest.id === provider)?.manifest.displayName ??
    provider
  );
}
