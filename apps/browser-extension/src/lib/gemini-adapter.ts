import type { MessageRole } from "@tf0000/adapter-sdk";

import {
  attributeRole,
  type ClipboardWriter,
  DomProviderAdapter,
  normalizeRole,
} from "./dom-provider-adapter";

export class GeminiAdapter extends DomProviderAdapter {
  constructor(document: Document, clipboard?: ClipboardWriter) {
    super(
      document,
      {
        provider: "gemini",
        displayName: "Gemini",
        hostnames: ["gemini.google.com"],
        titleSuffix: /\s*[|–-]\s*Gemini\s*$/i,
        conversationId: ({ pathname }) => pathname.match(/\/app\/([^/?#]+)/)?.[1],
        turnSelectors: [
          "[data-message-author-role]",
          "user-query, model-response",
          "[data-test-id='user-query'], [data-test-id='model-response']",
        ],
        bodySelectors: [
          ".query-text",
          ".model-response-text",
          "[data-message-content]",
          ".markdown",
        ],
        composerSelectors: [
          "rich-textarea [contenteditable='true']",
          ".ql-editor[contenteditable='true']",
          "textarea[aria-label*='prompt' i]",
        ],
        role: (element): MessageRole => {
          const attributed = attributeRole(element);
          if (attributed !== "unknown") return attributed;
          const marker = `${element.tagName} ${element.dataset.testId ?? ""}`.toLowerCase();
          return normalizeRole(
            marker.includes("user") ? "user" : marker.includes("model") ? "assistant" : undefined,
          );
        },
      },
      clipboard,
    );
  }
}
