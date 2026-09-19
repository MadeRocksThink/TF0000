import { browser } from "wxt/browser";

import { createProviderAdapter } from "../lib/adapter-registry";
import type { CapturePayload, PageRequest } from "../lib/types";

export default defineContentScript({
  matches: ["https://chatgpt.com/*", "https://claude.ai/*", "https://gemini.google.com/*"],
  main() {
    installLauncher();
    browser.runtime.onMessage.addListener(async (request: PageRequest) => {
      const adapter = createProviderAdapter(document);
      if (!adapter) throw new Error("No supported provider adapter is available on this page");
      if (request.type === "adapterHealth") return adapter.healthCheck();
      if (request.type === "metadata") return adapter.getConversationMetadata();
      if (request.type === "insert") return adapter.insertContext(request.text);
      const conversation = await adapter.captureVisibleConversation();
      let fragments = request.mode === "selection" ? await adapter.captureSelection() : [];
      if (request.mode === "messages") {
        fragments = conversation.messages.map((message) => ({
          ...(message.externalRef ? { messageExternalRef: message.externalRef } : {}),
          startOffset: 0,
          endOffset: message.body.length,
          selectedText: message.body,
          sourceHash: message.sourceHash,
        }));
      }
      return { ...conversation, fragments } satisfies CapturePayload;
    });
  },
});

function installLauncher(): void {
  if (document.querySelector("#tf0000-launcher")) return;
  const button = document.createElement("button");
  button.id = "tf0000-launcher";
  button.type = "button";
  button.textContent = "Context";
  button.title = "Open TF0000";
  Object.assign(button.style, {
    position: "fixed",
    right: "18px",
    bottom: "82px",
    zIndex: "2147483647",
    border: "1px solid rgba(255,255,255,.18)",
    borderRadius: "999px",
    padding: "8px 13px",
    background: "#17211d",
    color: "#e7f7ef",
    font: "600 12px system-ui, sans-serif",
    boxShadow: "0 6px 20px rgba(0,0,0,.25)",
    cursor: "pointer",
  });
  button.addEventListener("click", () => {
    void browser.runtime.sendMessage({ type: "openSidePanel" });
  });
  document.body.append(button);
}
