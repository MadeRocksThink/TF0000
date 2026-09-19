import { browser } from "wxt/browser";

import { NativeClient } from "../lib/native-client";
import type { ExtensionRequest, OpenHandoffResult, PageResponse } from "../lib/types";

const providerUrls = {
  chatgpt: "https://chatgpt.com/",
  claude: "https://claude.ai/new",
  gemini: "https://gemini.google.com/app",
} as const;

export default defineBackground(() => {
  const nativeClient = new NativeClient();

  browser.runtime.onInstalled.addListener(() => {
    void browser.sidePanel.setPanelBehavior({ openPanelOnActionClick: true });
  });

  browser.runtime.onMessage.addListener(async (message: ExtensionRequest, sender) => {
    if (sender.id && sender.id !== browser.runtime.id) {
      throw new Error("Rejected a message from an untrusted extension sender");
    }
    if (message.type === "native") {
      return nativeClient.request(message.action, message.payload ?? {});
    }
    if (message.type === "openSidePanel") {
      const tabId = sender.tab?.id;
      if (tabId === undefined) throw new Error("Unable to identify the active tab");
      await browser.sidePanel.open({ tabId });
      return { opened: true };
    }
    if (message.type === "openHandoff") {
      const tab = await browser.tabs.create({
        active: true,
        url: providerUrls[message.destinationProvider],
      });
      if (tab.id === undefined) throw new Error("The destination tab could not be opened");
      const result = await insertWhenReady(tab.id, message.text);
      return { ...result, tabId: tab.id } satisfies OpenHandoffResult;
    }
    const [tab] = await browser.tabs.query({ active: true, currentWindow: true });
    if (message.type === "activeTab") {
      const url = tab?.url ?? "";
      const hostname = safeHostname(url);
      const provider =
        hostname === "chatgpt.com"
          ? "chatgpt"
          : hostname === "claude.ai"
            ? "claude"
            : hostname === "gemini.google.com"
              ? "gemini"
              : "manual";
      return { title: tab?.title ?? "Untitled page", url, provider };
    }
    if (tab?.id === undefined) throw new Error("No active tab is available");
    return browser.tabs.sendMessage(tab.id, message.request) as Promise<PageResponse>;
  });
});

async function insertWhenReady(tabId: number, text: string): Promise<OpenHandoffResult> {
  let detail = "Destination opened; the handoff is on your clipboard.";
  for (let attempt = 0; attempt < 40; attempt += 1) {
    try {
      const tab = await browser.tabs.get(tabId);
      if (tab.status === "complete") {
        const result = (await browser.tabs.sendMessage(tabId, {
          type: "insert",
          text,
        })) as PageResponse;
        if ("inserted" in result) {
          if (result.inserted) return { ...result, tabId };
          detail = result.detail ?? detail;
        }
      }
    } catch (error) {
      detail = error instanceof Error ? error.message : String(error);
    }
    await delay(250);
  }
  return { inserted: false, method: "clipboard", detail, tabId };
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function safeHostname(url: string): string {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}
