import { describe, expect, it } from "vitest";

import { encodeFrame, FrameDecoder, MAX_FRAME_BYTES } from "./protocol";

const response = {
  protocolVersion: 1,
  requestId: "request-1",
  ok: true,
  data: { healthy: true },
};

describe("native host framing", () => {
  it("decodes split and adjacent frames", () => {
    const first = encodeFrame(response);
    const second = encodeFrame({ ...response, requestId: "request-2" });
    const decoder = new FrameDecoder();
    expect(decoder.push(first.subarray(0, 7))).toEqual([]);
    expect(decoder.push(Buffer.concat([first.subarray(7), second]))).toEqual([
      response,
      { ...response, requestId: "request-2" },
    ]);
  });

  it("rejects oversized native messages", () => {
    const frame = Buffer.alloc(4);
    frame.writeUInt32LE(MAX_FRAME_BYTES + 1);
    expect(() => new FrameDecoder().push(frame)).toThrow("Invalid native message length");
  });
});
