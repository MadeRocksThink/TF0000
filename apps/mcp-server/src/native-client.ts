import { type ChildProcessWithoutNullStreams, spawn } from "node:child_process";
import { randomBytes, randomUUID } from "node:crypto";
import { existsSync } from "node:fs";
import { delimiter, dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { encodeFrame, FrameDecoder, type NativeResponse } from "./protocol";

const PROTOCOL_VERSION = 1;
const HOST_CLIENT_ID = "abcdefghijklmnopabcdefghijklmnop";
const REQUEST_TIMEOUT_MS = 15_000;

interface PendingRequest {
  resolve: (response: NativeResponse<unknown>) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

export class NativeHostClient {
  private process: ChildProcessWithoutNullStreams | null = null;
  private sessionId: string | null = null;
  private connecting: Promise<void> | null = null;
  private readonly pending = new Map<string, PendingRequest>();
  private decoder = new FrameDecoder();

  constructor(private readonly executable = resolveNativeHostPath()) {}

  async request<T>(action: string, payload: unknown = {}): Promise<T> {
    await this.connect();
    return this.send<T>({
      type: "request",
      protocolVersion: PROTOCOL_VERSION,
      requestId: randomUUID(),
      sessionId: this.sessionId,
      action,
      payload,
    });
  }

  dispose(): void {
    const error = new Error("TF0000 native host was stopped");
    for (const request of this.pending.values()) {
      clearTimeout(request.timer);
      request.reject(error);
    }
    this.pending.clear();
    this.process?.kill();
    this.process = null;
    this.sessionId = null;
  }

  private async connect(): Promise<void> {
    if (this.process && this.sessionId) return;
    if (this.connecting) return this.connecting;
    this.connecting = this.open();
    try {
      await this.connecting;
    } finally {
      this.connecting = null;
    }
  }

  private async open(): Promise<void> {
    this.decoder = new FrameDecoder();
    const child = spawn(this.executable, [], {
      stdio: "pipe",
      windowsHide: true,
      env: process.env,
    });
    this.process = child;
    child.stdout.on("data", (chunk: Buffer) => {
      try {
        for (const response of this.decoder.push(chunk)) this.handleResponse(response);
      } catch (error) {
        this.failAll(asError(error));
      }
    });
    child.stderr.on("data", (chunk: Buffer) => {
      const message = chunk.toString("utf8").trim();
      if (message) process.stderr.write(`[TF0000 native host] ${message}\n`);
    });
    child.on("error", (error) => this.failAll(error));
    child.on("exit", (code) => {
      this.process = null;
      this.sessionId = null;
      this.failAll(new Error(`TF0000 native host exited${code === null ? "" : ` (${code})`}`));
    });
    const nonce = randomBytes(16).toString("hex");
    const response = await this.send<{ sessionId: string; nonce: string }>({
      type: "hello",
      protocolVersion: PROTOCOL_VERSION,
      requestId: randomUUID(),
      extensionId: HOST_CLIENT_ID,
      nonce,
    });
    if (!response.sessionId || response.nonce !== nonce) {
      this.dispose();
      throw new Error("TF0000 native host handshake verification failed");
    }
    this.sessionId = response.sessionId;
  }

  private send<T>(message: Record<string, unknown>): Promise<T> {
    if (!this.process) return Promise.reject(new Error("TF0000 native host is not running"));
    const requestId = String(message.requestId);
    return new Promise<T>((resolvePromise, rejectPromise) => {
      const timer = setTimeout(() => {
        this.pending.delete(requestId);
        rejectPromise(new Error("TF0000 native host request timed out"));
      }, REQUEST_TIMEOUT_MS);
      this.pending.set(requestId, {
        resolve: (response) => {
          if (response.data === undefined) rejectPromise(new Error("Native host returned no data"));
          else resolvePromise(response.data as T);
        },
        reject: rejectPromise,
        timer,
      });
      this.process?.stdin.write(encodeFrame(message), (error) => {
        if (!error) return;
        const pending = this.pending.get(requestId);
        if (!pending) return;
        clearTimeout(pending.timer);
        this.pending.delete(requestId);
        pending.reject(error);
      });
    });
  }

  private handleResponse(response: NativeResponse<unknown>): void {
    const pending = this.pending.get(response.requestId);
    if (!pending) return;
    clearTimeout(pending.timer);
    this.pending.delete(response.requestId);
    if (response.ok) pending.resolve(response);
    else pending.reject(new Error(response.error ?? "TF0000 native host request failed"));
  }

  private failAll(error: Error): void {
    for (const request of this.pending.values()) {
      clearTimeout(request.timer);
      request.reject(error);
    }
    this.pending.clear();
  }
}

export function resolveNativeHostPath(environmentPath = process.env.PATH ?? ""): string {
  const configured = process.env.TF0000_NATIVE_HOST;
  if (configured) {
    if (!isAbsolute(configured) || !existsSync(configured)) {
      throw new Error("TF0000_NATIVE_HOST must be an existing absolute file path");
    }
    return configured;
  }
  const executable = process.platform === "win32" ? "tf0000-native-host.exe" : "tf0000-native-host";
  const moduleDirectory = dirname(fileURLToPath(import.meta.url));
  const candidates = [
    join(moduleDirectory, "..", "bin", executable),
    join(moduleDirectory, "bin", executable),
    resolve(moduleDirectory, "..", "..", "..", "target", "release", executable),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }
  for (const directory of environmentPath.split(delimiter).filter(Boolean)) {
    const candidate = join(directory, executable);
    if (existsSync(candidate)) return candidate;
  }
  return executable;
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error(String(error));
}
