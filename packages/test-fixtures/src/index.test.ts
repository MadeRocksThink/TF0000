import { describe, expect, it } from "vitest";
import { normalizedConversationFixtures } from "./index";

describe("published normalized fixtures", () => {
  it.each(Object.entries(normalizedConversationFixtures))(
    "%s has ordered, non-empty messages",
    (provider, fixture) => {
      expect(fixture.provider).toBe(provider);
      expect(fixture.title.trim()).not.toBe("");
      expect(fixture.messages.length).toBeGreaterThan(0);
      expect(fixture.messages.map((message) => message.ordinal)).toEqual(
        fixture.messages.map((_, index) => index),
      );
      expect(fixture.messages.every((message) => message.body.trim().length > 0)).toBe(true);
    },
  );
});
