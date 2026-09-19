import { browser } from "wxt/browser";

const HOST_NAME = "com.tf0000.context";
const PROTOCOL_VERSION = 1;

interface NativeResponse<T> {
  protocolVersion: number;
  requestId: string;
  ok: boolean;
  sessionId?: string;
  data?: T;
  error?: string;
}

interface PendingRequest {
  resolve: (response: NativeResponse<unknown>) => void;
  reject: (error: Error) => void;
}

export class NativeClient {
  private port: Browser.runtime.Port | null = null;
  private sessionId: string | null = null;
  private connecting: Promise<void> | null = null;
  private readonly pending = new Map<string, PendingRequest>();

  async request<T>(action: string, payload: unknown = {}): Promise<T> {
    await this.connect();
    const requestId = crypto.randomUUID();
    const response = await this.send<T>({
      type: "request",
      protocolVersion: PROTOCOL_VERSION,
      requestId,
      sessionId: this.sessionId,
      action,
      payload,
    });
    return response;
  }

  private async connect(): Promise<void> {
    if (this.port && this.sessionId) return;
    if (this.connecting) return this.connecting;
    this.connecting = this.open();
    try {
      await this.connecting;
    } finally {
      this.connecting = null;
    }
  }

  private async open(): Promise<void> {
    const port = browser.runtime.connectNative(HOST_NAME);
    this.port = port;
    port.onMessage.addListener((message: NativeResponse<unknown>) => {
      const pending = this.pending.get(message.requestId);
      if (!pending) return;
      this.pending.delete(message.requestId);
      if (message.ok) pending.resolve(message);
      else pending.reject(new Error(message.error ?? "Native host request failed"));
    });
    port.onDisconnect.addListener(() => {
      const message = browser.runtime.lastError?.message ?? "Native host disconnected";
      this.port = null;
      this.sessionId = null;
      for (const pending of this.pending.values()) pending.reject(new Error(message));
      this.pending.clear();
    });
    const nonce = Array.from(crypto.getRandomValues(new Uint8Array(16)), (value) =>
      value.toString(16).padStart(2, "0"),
    ).join("");
    const response = await this.send<{ sessionId: string; nonce: string }>({
      type: "hello",
      protocolVersion: PROTOCOL_VERSION,
      requestId: crypto.randomUUID(),
      extensionId: browser.runtime.id,
      nonce,
    });
    if (response.nonce !== nonce || !response.sessionId) {
      port.disconnect();
      throw new Error("Native host handshake verification failed");
    }
    this.sessionId = response.sessionId;
  }

  private send<T>(message: Record<string, unknown>): Promise<T> {
    if (!this.port) return Promise.reject(new Error("Native host is not connected"));
    const requestId = String(message.requestId);
    return new Promise<T>((resolve, reject) => {
      this.pending.set(requestId, {
        resolve: (response) => {
          if (response.data === undefined) reject(new Error("Native host returned no data"));
          else resolve(response.data as T);
        },
        reject,
      });
      try {
        this.port?.postMessage(message);
      } catch (error) {
        this.pending.delete(requestId);
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }
}
