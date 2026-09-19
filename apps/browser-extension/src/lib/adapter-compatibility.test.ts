/** @vitest-environment node */
import { verifyAdapterFixture } from "@tf0000/adapter-sdk";
import { JSDOM } from "jsdom";
import { describe, expect, it } from "vitest";
import chatgptHtml from "../../../../packages/test-fixtures/fixtures/chatgpt.html?raw";
import claudeHtml from "../../../../packages/test-fixtures/fixtures/claude.html?raw";
import geminiHtml from "../../../../packages/test-fixtures/fixtures/gemini.html?raw";
import { adapterRegistrations } from "./adapter-registry";

const fixtures = [
  {
    id: "chatgpt",
    url: "https://chatgpt.com/c/test-conversation",
    title: "Phase 2 planning | ChatGPT",
    expectedTitle: "Phase 2 planning",
    html: chatgptHtml,
    messages: [
      { role: "user" as const, body: "Build the browser capture safely." },
      { role: "assistant" as const, body: "I will keep message sending under user control." },
    ],
  },
  {
    id: "claude",
    url: "https://claude.ai/chat/claude-conversation",
    title: "Portable context – Claude",
    expectedTitle: "Portable context",
    html: claudeHtml,
    messages: [
      { role: "user" as const, body: "Carry this decision to another provider." },
      { role: "assistant" as const, body: "The normalized schema keeps the role and exact text." },
    ],
  },
  {
    id: "gemini",
    url: "https://gemini.google.com/app/gemini-conversation",
    title: "Cross-provider handoff | Gemini",
    expectedTitle: "Cross-provider handoff",
    html: geminiHtml,
    messages: [
      { role: "user" as const, body: "Use the context captured in Claude." },
      { role: "assistant" as const, body: "The source link remains attached." },
    ],
  },
];

describe("published adapter compatibility matrix", () => {
  it.each(fixtures)("$id passes the stable SDK fixture contract", async (fixture) => {
    const dom = new JSDOM(fixture.html, { url: fixture.url });
    Object.defineProperty(dom.window.document, "title", {
      configurable: true,
      value: fixture.title,
    });
    const registration = adapterRegistrations.find(({ manifest }) => manifest.id === fixture.id);
    expect(registration).toBeTruthy();
    const result = await verifyAdapterFixture(registration?.create(dom.window.document) as never, {
      provider: fixture.id,
      title: fixture.expectedTitle,
      messages: fixture.messages,
    });
    expect(result.passed).toBe(true);
  });
});
