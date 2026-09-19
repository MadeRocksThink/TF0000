import type { MessageRole } from "@tf0000/adapter-sdk";

import { attributeRole, type ClipboardWriter, DomProviderAdapter } from "./dom-provider-adapter";

export class ChatGptAdapter extends DomProviderAdapter {
  constructor(document: Document, clipboard?: ClipboardWriter) {
    super(
      document,
      {
        provider: "chatgpt",
        displayName: "ChatGPT",
        hostnames: ["chatgpt.com"],
        titleSuffix: /\s*[|–-]\s*ChatGPT\s*$/i,
        conversationId: ({ pathname }) => pathname.match(/(?:\/g\/[^/]+)?\/c\/([^/?#]+)/)?.[1],
        turnSelectors: [
          "[data-message-author-role]",
          "article[data-testid^='conversation-turn-']",
          "[data-testid^='conversation-turn-']",
        ],
        bodySelectors: ["[data-message-content]", ".markdown", "[class*='markdown']"],
        composerSelectors: [
          "#prompt-textarea",
          "[data-testid='prompt-textarea']",
          "textarea[name='prompt-textarea']",
        ],
        role: (element): MessageRole => attributeRole(element),
      },
      clipboard,
    );
  }
}
