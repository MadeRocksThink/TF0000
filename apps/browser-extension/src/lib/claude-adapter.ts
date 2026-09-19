import type { MessageRole } from "@tf0000/adapter-sdk";

import {
  attributeRole,
  type ClipboardWriter,
  DomProviderAdapter,
  normalizeRole,
} from "./dom-provider-adapter";

export class ClaudeAdapter extends DomProviderAdapter {
  constructor(document: Document, clipboard?: ClipboardWriter) {
    super(
      document,
      {
        provider: "claude",
        displayName: "Claude",
        hostnames: ["claude.ai"],
        titleSuffix: /\s*[|–-]\s*Claude\s*$/i,
        conversationId: ({ pathname }) => pathname.match(/\/chat\/([^/?#]+)/)?.[1],
        turnSelectors: [
          "[data-message-author-role]",
          "[data-testid='user-message'], [data-testid='assistant-message']",
          ".font-user-message, .font-claude-message",
        ],
        bodySelectors: [
          "[data-message-content]",
          ".font-user-message",
          ".font-claude-message",
          ".markdown",
        ],
        composerSelectors: [
          "[contenteditable='true'][data-testid='chat-input']",
          ".ProseMirror[contenteditable='true']",
          "fieldset [contenteditable='true']",
        ],
        role: (element): MessageRole => {
          const attributed = attributeRole(element);
          if (attributed !== "unknown") return attributed;
          const marker = `${element.dataset.testid ?? ""} ${element.className}`;
          return normalizeRole(
            marker.includes("user")
              ? "user"
              : marker.includes("claude") || marker.includes("assistant")
                ? "assistant"
                : undefined,
          );
        },
      },
      clipboard,
    );
  }
}
