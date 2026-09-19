/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "https://gemini.google.com/" }
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../../../packages/test-fixtures/fixtures/gemini.html?raw";

import { GeminiAdapter } from "./gemini-adapter";

describe("GeminiAdapter", () => {
  beforeEach(() => {
    document.documentElement.innerHTML = fixture;
    window.history.replaceState({}, "", "/app/gemini-conversation");
    Object.defineProperty(document, "title", {
      configurable: true,
      value: "Cross-provider handoff | Gemini",
    });
  });

  it("normalizes Gemini messages into the shared schema", async () => {
    const capture = await new GeminiAdapter(document).captureVisibleConversation();
    expect(capture.provider).toBe("gemini");
    expect(capture.externalRef).toBe("gemini-conversation");
    expect(capture.title).toBe("Cross-provider handoff");
    expect(capture.messages.map(({ role, body }) => ({ role, body }))).toEqual([
      { role: "user", body: "Use the context captured in Claude." },
      { role: "assistant", body: "The source link remains attached." },
    ]);
  });

  it("inserts without sending", async () => {
    const onSend = vi.fn();
    document.querySelector("[data-test-id='send-button']")?.addEventListener("click", onSend);
    const result = await new GeminiAdapter(document).insertContext("Context from ChatGPT");
    expect(result.inserted).toBe(true);
    expect(document.querySelector(".ql-editor")?.textContent).toBe("Context from ChatGPT");
    expect(onSend).not.toHaveBeenCalled();
  });
});
