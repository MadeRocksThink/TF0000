import { Buffer } from "node:buffer";
import { describe, expect, it } from "vitest";

import { encodeFrame, FrameDecoder, MAX_FRAME_BYTES } from "./protocol";

describe("native host framing", () => {
  it("decodes split and adjacent frames", () => {
    const first = encodeFrame({ protocolVersion: 1, requestId: "one", ok: true, data: {} });
    const second = encodeFrame({ protocolVersion: 1, requestId: "two", ok: false, error: "no" });
    const decoder = new FrameDecoder();
    expect(decoder.push(first.subarray(0, 3))).toEqual([]);
    const messages = decoder.push(Buffer.concat([first.subarray(3), second]));
    expect(messages.map((message) => message.requestId)).toEqual(["one", "two"]);
  });

  it("rejects oversized frames", () => {
    const bytes = Buffer.alloc(4);
    bytes.writeUInt32LE(MAX_FRAME_BYTES + 1);
    expect(() => new FrameDecoder().push(bytes)).toThrow("Invalid native message length");
  });
});
