import type {
  AdapterHealth,
  CapturedConversation,
  CapturedFragment,
  CapturedMessage,
  ChatProviderAdapter,
  ConversationMetadata,
  ConversationRef,
  InsertResult,
  MessageRole,
  ProviderId,
} from "@tf0000/adapter-sdk";

export interface ClipboardWriter {
  writeText(text: string): Promise<void>;
}

export interface DomProviderConfig {
  provider: ProviderId;
  displayName: string;
  hostnames: readonly string[];
  titleSuffix: RegExp;
  conversationId(location: Location): string | undefined;
  turnSelectors: readonly string[];
  bodySelectors: readonly string[];
  composerSelectors: readonly string[];
  role(element: HTMLElement): MessageRole;
}

const ADAPTER_VERSION = "0.2.0";

export class DomProviderAdapter implements ChatProviderAdapter {
  constructor(
    private readonly document: Document,
    private readonly config: DomProviderConfig,
    private readonly clipboard: ClipboardWriter | undefined = document.defaultView?.navigator
      .clipboard,
  ) {}

  async detectConversation(): Promise<ConversationRef | null> {
    const { location } = this.document;
    if (!this.config.hostnames.includes(location.hostname)) return null;
    return {
      provider: this.config.provider,
      externalRef: this.config.conversationId(location) ?? `page:${location.pathname}`,
      title: this.cleanTitle(this.document.title),
      url: location.href,
    };
  }

  async captureVisibleConversation(): Promise<CapturedConversation> {
    const conversation = await this.detectConversation();
    if (!conversation) {
      throw new Error(`No ${this.config.displayName} conversation was detected on this page`);
    }
    const messages: CapturedMessage[] = [];
    for (const [ordinal, element] of this.findTurns().entries()) {
      const body = this.readMessageBody(element);
      if (!body) continue;
      const role = this.config.role(element);
      const sourceHash = await sha256(`${role}\n${body}`);
      messages.push({
        externalRef: messageExternalRef(element, ordinal, sourceHash),
        role,
        body,
        ordinal,
        sourceHash,
      });
    }
    return { ...conversation, capturedAt: new Date().toISOString(), messages };
  }

  async captureSelection(): Promise<CapturedFragment[]> {
    const selection = this.document.defaultView?.getSelection();
    const selectedText = selection?.toString().trim() ?? "";
    if (!selection || selection.rangeCount === 0 || !selectedText) return [];
    const turn = this.closestTurn(selection.getRangeAt(0).commonAncestorContainer);
    if (!turn) return [];
    const ordinal = this.findTurns().indexOf(turn);
    const body = this.readMessageBody(turn);
    const role = this.config.role(turn);
    const messageHash = await sha256(`${role}\n${body}`);
    const startOffset = body.indexOf(selectedText);
    return [
      {
        messageExternalRef: messageExternalRef(turn, Math.max(ordinal, 0), messageHash),
        ...(startOffset >= 0 ? { startOffset, endOffset: startOffset + selectedText.length } : {}),
        selectedText,
        sourceHash: await sha256(`${messageHash}\n${selectedText}`),
      },
    ];
  }

  async readComposer(): Promise<string | null> {
    const composer = this.findComposer();
    if (!composer) return null;
    return isTextControl(composer, this.document)
      ? (composer as HTMLTextAreaElement | HTMLInputElement).value
      : composer.textContent;
  }

  async insertContext(text: string): Promise<InsertResult> {
    const cleanText = text.trim();
    if (!cleanText) return { inserted: false, method: "composer", detail: "Context is empty" };
    const composer = this.findComposer();
    if (!composer)
      return this.copyFallback(cleanText, `${this.config.displayName} composer missing`);
    try {
      insertIntoComposer(composer, cleanText, this.document);
      return { inserted: true, method: "composer" };
    } catch (error) {
      return this.copyFallback(cleanText, error instanceof Error ? error.message : String(error));
    }
  }

  async getConversationMetadata(): Promise<ConversationMetadata> {
    const conversation = await this.detectConversation();
    if (!conversation) throw new Error(`${this.config.displayName} is not active`);
    return { ...conversation, messageCount: this.findTurns().length };
  }

  async healthCheck(): Promise<AdapterHealth> {
    if (!(await this.detectConversation())) {
      return {
        status: "unavailable",
        adapterVersion: ADAPTER_VERSION,
        detail: `Open a ${this.config.displayName} conversation`,
      };
    }
    const messageCount = this.findTurns().length;
    const composerReady = Boolean(this.findComposer());
    return {
      status: messageCount > 0 && composerReady ? "healthy" : "degraded",
      adapterVersion: ADAPTER_VERSION,
      detail: `${messageCount} messages detected; ${composerReady ? "composer ready" : "composer missing"}`,
    };
  }

  private findTurns(): HTMLElement[] {
    for (const selector of this.config.turnSelectors) {
      const matches = Array.from(this.document.querySelectorAll<HTMLElement>(selector)).filter(
        (element) => !element.hidden && element.getAttribute("aria-hidden") !== "true",
      );
      if (matches.length > 0) return removeNestedTurns(matches);
    }
    return [];
  }

  private closestTurn(node: Node): HTMLElement | null {
    const element =
      node.nodeType === this.document.defaultView?.Node.ELEMENT_NODE
        ? (node as Element)
        : node.parentElement;
    return element?.closest<HTMLElement>(this.config.turnSelectors.join(",")) ?? null;
  }

  private readMessageBody(element: HTMLElement): string {
    const body = element.querySelector<HTMLElement>(this.config.bodySelectors.join(","));
    return (
      body?.innerText ??
      body?.textContent ??
      element.innerText ??
      element.textContent ??
      ""
    ).trim();
  }

  private findComposer(): HTMLElement | null {
    for (const selector of this.config.composerSelectors) {
      const composer = this.document.querySelector<HTMLElement>(selector);
      if (composer) return composer;
    }
    return null;
  }

  private cleanTitle(title: string): string {
    return (
      title.replace(this.config.titleSuffix, "").trim() ||
      `Untitled ${this.config.displayName} conversation`
    );
  }

  private async copyFallback(text: string, reason: string): Promise<InsertResult> {
    if (!this.clipboard) return { inserted: false, method: "clipboard", detail: reason };
    try {
      await this.clipboard.writeText(text);
      return { inserted: false, method: "clipboard", detail: `${reason}; copied to clipboard` };
    } catch {
      return { inserted: false, method: "clipboard", detail: `${reason}; clipboard write failed` };
    }
  }
}

export function attributeRole(element: HTMLElement): MessageRole {
  return normalizeRole(
    element.getAttribute("data-message-author-role") ??
      element.querySelector<HTMLElement>("[data-message-author-role]")?.dataset.messageAuthorRole,
  );
}

export function normalizeRole(value: string | undefined | null): MessageRole {
  const role = value?.toLowerCase();
  if (role === "human") return "user";
  if (role === "model" || role === "claude") return "assistant";
  return role && ["user", "assistant", "system", "tool"].includes(role)
    ? (role as MessageRole)
    : "unknown";
}

export async function sha256(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function removeNestedTurns(elements: HTMLElement[]): HTMLElement[] {
  return elements.filter(
    (element) => !elements.some((other) => other !== element && other.contains(element)),
  );
}

function messageExternalRef(element: HTMLElement, ordinal: number, hash: string): string {
  return (
    element.dataset.messageId ??
    element.getAttribute("data-message-id") ??
    element.getAttribute("data-testid") ??
    (element.id || `turn-${ordinal}-${hash.slice(0, 12)}`)
  );
}

function isTextControl(element: HTMLElement, document: Document): boolean {
  const view = document.defaultView;
  return Boolean(
    view &&
      (element instanceof view.HTMLTextAreaElement || element instanceof view.HTMLInputElement),
  );
}

function insertIntoComposer(composer: HTMLElement, text: string, document: Document): void {
  composer.focus();
  const view = document.defaultView;
  if (!view) throw new Error("Page window is unavailable");
  if (isTextControl(composer, document)) {
    const control = composer as HTMLTextAreaElement | HTMLInputElement;
    const separator = control.value.trim() ? "\n\n" : "";
    const prototype =
      control instanceof view.HTMLTextAreaElement
        ? view.HTMLTextAreaElement.prototype
        : view.HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(prototype, "value")?.set;
    if (!setter) throw new Error("Composer value setter is unavailable");
    setter.call(control, `${control.value}${separator}${text}`);
  } else {
    const existing = composer.textContent?.trim() ?? "";
    composer.textContent = `${existing}${existing ? "\n\n" : ""}${text}`;
  }
  const InputEventConstructor = view.InputEvent ?? view.Event;
  composer.dispatchEvent(
    new InputEventConstructor("input", { bubbles: true, inputType: "insertText", data: text }),
  );
}
