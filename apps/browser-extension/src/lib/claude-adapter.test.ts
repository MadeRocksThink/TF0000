/**
 * @vitest-environment jsdom
 * @vitest-environment-options { "url": "https://claude.ai/" }
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../../../packages/test-fixtures/fixtures/claude.html?raw";

import { ClaudeAdapter } from "./claude-adapter";

describe("ClaudeAdapter", () => {
  beforeEach(() => {
    document.documentElement.innerHTML = fixture;
    window.history.replaceState({}, "", "/chat/claude-conversation");
    Object.defineProperty(document, "title", {
      configurable: true,
      value: "Portable context – Claude",
    });
  });

  it("normalizes Claude messages into the shared schema", async () => {
    const capture = await new ClaudeAdapter(document).captureVisibleConversation();
    expect(capture.provider).toBe("claude");
    expect(capture.externalRef).toBe("claude-conversation");
    expect(capture.title).toBe("Portable context");
    expect(capture.messages.map(({ role, body }) => ({ role, body }))).toEqual([
      { role: "user", body: "Carry this decision to another provider." },
      { role: "assistant", body: "The normalized schema keeps the role and exact text." },
    ]);
  });

  it("inserts without sending", async () => {
    const onSend = vi.fn();
    document.querySelector("[data-testid='send-button']")?.addEventListener("click", onSend);
    const result = await new ClaudeAdapter(document).insertContext("Context from Gemini");
    expect(result.inserted).toBe(true);
    expect(document.querySelector("[data-testid='chat-input']")?.textContent).toBe(
      "Context from Gemini",
    );
    expect(onSend).not.toHaveBeenCalled();
  });
});
