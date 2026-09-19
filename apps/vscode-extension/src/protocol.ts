import { Buffer } from "node:buffer";

import type { NativeResponse } from "./types";

export const MAX_FRAME_BYTES = 2 * 1024 * 1024;

export function encodeFrame(message: unknown): Buffer {
  const payload = Buffer.from(JSON.stringify(message), "utf8");
  if (payload.length === 0 || payload.length > MAX_FRAME_BYTES) {
    throw new Error(`Invalid native message length: ${payload.length}`);
  }
  const frame = Buffer.allocUnsafe(payload.length + 4);
  frame.writeUInt32LE(payload.length, 0);
  payload.copy(frame, 4);
  return frame;
}

export class FrameDecoder {
  private buffer: Buffer = Buffer.alloc(0);

  push(chunk: Buffer): NativeResponse<unknown>[] {
    this.buffer = this.buffer.length === 0 ? chunk : Buffer.concat([this.buffer, chunk]);
    const messages: NativeResponse<unknown>[] = [];
    while (this.buffer.length >= 4) {
      const length = this.buffer.readUInt32LE(0);
      if (length === 0 || length > MAX_FRAME_BYTES) {
        this.buffer = Buffer.alloc(0);
        throw new Error(`Invalid native message length: ${length}`);
      }
      if (this.buffer.length < length + 4) break;
      const payload = this.buffer.subarray(4, length + 4);
      this.buffer = this.buffer.subarray(length + 4);
      const parsed: unknown = JSON.parse(payload.toString("utf8"));
      if (!isNativeResponse(parsed)) throw new Error("Invalid native host response");
      messages.push(parsed);
    }
    return messages;
  }
}

function isNativeResponse(value: unknown): value is NativeResponse<unknown> {
  if (!value || typeof value !== "object") return false;
  const response = value as Record<string, unknown>;
  return (
    response.protocolVersion === 1 &&
    typeof response.requestId === "string" &&
    typeof response.ok === "boolean"
  );
}
