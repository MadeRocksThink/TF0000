import { beforeEach, describe, expect, it, vi } from "vitest";
import fixture from "../../../../packages/test-fixtures/fixtures/chatgpt.html?raw";

import { ChatGptAdapter } from "./chatgpt-adapter";

describe("ChatGptAdapter", () => {
  beforeEach(() => {
    document.documentElement.innerHTML = fixture;
    window.history.replaceState({}, "", "/c/test-conversation");
    Object.defineProperty(document, "title", {
      configurable: true,
      value: "Phase 2 planning | ChatGPT",
    });
  });

  it("captures title, conversation id, roles, and visible message text", async () => {
    const capture = await new ChatGptAdapter(document).captureVisibleConversation();
    expect(capture.externalRef).toBe("test-conversation");
    expect(capture.title).toBe("Phase 2 planning");
    expect(capture.messages.map(({ role, body }) => ({ role, body }))).toEqual([
      { role: "user", body: "Build the browser capture safely." },
      { role: "assistant", body: "I will keep message sending under user control." },
    ]);
  });

  it("captures a selected fragment and its source message", async () => {
    const text = document.querySelector(".markdown")?.firstChild;
    expect(text).toBeTruthy();
    const range = document.createRange();
    range.setStart(text as Node, 2);
    range.setEnd(text as Node, 13);
    window.getSelection()?.removeAllRanges();
    window.getSelection()?.addRange(range);
    const fragments = await new ChatGptAdapter(document).captureSelection();
    expect(fragments).toHaveLength(1);
    expect(fragments[0]?.selectedText).toBe("will keep m");
    expect(fragments[0]?.messageExternalRef).toBe("msg-assistant-1");
  });

  it("inserts context without activating the send control", async () => {
    const onSend = vi.fn();
    document.querySelector("[data-testid='send-button']")?.addEventListener("click", onSend);
    const result = await new ChatGptAdapter(document).insertContext("A verified decision");
    expect(result).toEqual({ inserted: true, method: "composer" });
    expect(document.querySelector("#prompt-textarea")?.textContent).toBe("A verified decision");
    expect(onSend).not.toHaveBeenCalled();
  });

  it("uses the clipboard when the composer is unavailable", async () => {
    document.querySelector("#prompt-textarea")?.remove();
    const writeText = vi.fn().mockResolvedValue(undefined);
    const result = await new ChatGptAdapter(document, { writeText }).insertContext("Fallback");
    expect(result.method).toBe("clipboard");
    expect(writeText).toHaveBeenCalledWith("Fallback");
  });
});
