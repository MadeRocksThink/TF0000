import {
  type PortableMemoryRecord,
  type PortableWorkspace,
  parsePortableWorkspace,
  serializePortableWorkspace,
} from "@tf0000/context-format";

export const SYNC_FORMAT = "tf0000-sync/v1" as const;
const ITERATIONS = 310_000;
const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export interface EncryptedSyncEnvelope {
  format: typeof SYNC_FORMAT;
  algorithm: "AES-256-GCM";
  kdf: "PBKDF2-SHA-256";
  iterations: typeof ITERATIONS;
  createdAt: string;
  deviceId: string;
  salt: string;
  iv: string;
  ciphertext: string;
}

export type MemoryMergeDisposition = "identical" | "local-ahead" | "remote-ahead" | "conflict";
export interface MemoryMergeDecision {
  memoryId: string;
  disposition: MemoryMergeDisposition;
  commonVersionId: string | null;
}

export function classifyMemoryVersions(
  local: PortableMemoryRecord,
  remote: PortableMemoryRecord,
): MemoryMergeDecision {
  if (local.memory.id !== remote.memory.id) throw new Error("Cannot compare different memories");
  const localCurrent = local.memory.currentVersionId;
  const remoteCurrent = remote.memory.currentVersionId;
  if (localCurrent === remoteCurrent)
    return { memoryId: local.memory.id, disposition: "identical", commonVersionId: localCurrent };
  const localIds = new Set(local.versions.map((version) => version.id));
  const remoteIds = new Set(remote.versions.map((version) => version.id));
  if (localIds.has(remoteCurrent))
    return {
      memoryId: local.memory.id,
      disposition: "local-ahead",
      commonVersionId: remoteCurrent,
    };
  if (remoteIds.has(localCurrent))
    return {
      memoryId: local.memory.id,
      disposition: "remote-ahead",
      commonVersionId: localCurrent,
    };
  const common =
    [...localIds]
      .filter((id) => remoteIds.has(id))
      .sort()
      .at(-1) ?? null;
  return { memoryId: local.memory.id, disposition: "conflict", commonVersionId: common };
}

export function buildMemoryMergePlan(
  local: PortableWorkspace,
  remote: PortableWorkspace,
): MemoryMergeDecision[] {
  const localById = new Map(local.memories.map((memory) => [memory.memory.id, memory]));
  return remote.memories.map((memory) => {
    const existing = localById.get(memory.memory.id);
    return existing
      ? classifyMemoryVersions(existing, memory)
      : { memoryId: memory.memory.id, disposition: "remote-ahead", commonVersionId: null };
  });
}

export async function encryptWorkspace(
  workspace: PortableWorkspace,
  passphrase: string,
): Promise<EncryptedSyncEnvelope> {
  validatePassphrase(passphrase);
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const header: Omit<EncryptedSyncEnvelope, "ciphertext"> = {
    format: SYNC_FORMAT,
    algorithm: "AES-256-GCM" as const,
    kdf: "PBKDF2-SHA-256" as const,
    iterations: ITERATIONS,
    createdAt: new Date().toISOString(),
    deviceId: workspace.sourceDeviceId,
    salt: toBase64(salt),
    iv: toBase64(iv),
  };
  const key = await deriveKey(passphrase, salt, ["encrypt"]);
  const encrypted = await crypto.subtle.encrypt(
    {
      name: "AES-GCM",
      iv: arrayBuffer(iv),
      additionalData: encoder.encode(aad(header)),
      tagLength: 128,
    },
    key,
    encoder.encode(serializePortableWorkspace(workspace)),
  );
  return { ...header, ciphertext: toBase64(new Uint8Array(encrypted)) };
}

export async function decryptWorkspace(
  envelope: EncryptedSyncEnvelope,
  passphrase: string,
): Promise<PortableWorkspace> {
  validateEnvelope(envelope);
  validatePassphrase(passphrase);
  const salt = fromBase64(envelope.salt);
  const iv = fromBase64(envelope.iv);
  const key = await deriveKey(passphrase, salt, ["decrypt"]);
  try {
    const clear = await crypto.subtle.decrypt(
      {
        name: "AES-GCM",
        iv: arrayBuffer(iv),
        additionalData: encoder.encode(aad(envelope)),
        tagLength: 128,
      },
      key,
      arrayBuffer(fromBase64(envelope.ciphertext)),
    );
    const workspace = parsePortableWorkspace(decoder.decode(clear));
    if (workspace.sourceDeviceId !== envelope.deviceId)
      throw new Error("Encrypted sync metadata does not match its payload");
    return workspace;
  } catch (error) {
    if (error instanceof Error && error.message.includes("metadata")) throw error;
    throw new Error("Unable to decrypt sync file. Check the passphrase and file integrity.");
  }
}

export function parseEncryptedEnvelope(json: string): EncryptedSyncEnvelope {
  if (encoder.encode(json).byteLength > 125 * 1024 * 1024)
    throw new Error("Encrypted sync file exceeds 125 MB");
  const value: unknown = JSON.parse(json);
  validateEnvelope(value);
  return value;
}
export function serializeEncryptedEnvelope(envelope: EncryptedSyncEnvelope): string {
  validateEnvelope(envelope);
  return `${JSON.stringify(envelope, null, 2)}\n`;
}

function validateEnvelope(value: unknown): asserts value is EncryptedSyncEnvelope {
  if (!value || typeof value !== "object") throw new Error("Invalid encrypted sync envelope");
  const item = value as Record<string, unknown>;
  if (
    item.format !== SYNC_FORMAT ||
    item.algorithm !== "AES-256-GCM" ||
    item.kdf !== "PBKDF2-SHA-256" ||
    item.iterations !== ITERATIONS
  )
    throw new Error("Unsupported encrypted sync format");
  for (const field of ["createdAt", "deviceId", "salt", "iv", "ciphertext"] as const)
    if (typeof item[field] !== "string" || !item[field])
      throw new Error(`Invalid sync field: ${field}`);
  if (fromBase64(String(item.salt)).length !== 16 || fromBase64(String(item.iv)).length !== 12)
    throw new Error("Invalid sync cryptographic parameters");
}
function validatePassphrase(value: string): void {
  if (value.length < 12 || value.length > 1024)
    throw new Error("Sync passphrase must contain 12–1024 characters");
}
async function deriveKey(
  passphrase: string,
  salt: Uint8Array,
  usages: KeyUsage[],
): Promise<CryptoKey> {
  const material = await crypto.subtle.importKey(
    "raw",
    encoder.encode(passphrase.normalize("NFKC")),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", hash: "SHA-256", salt: arrayBuffer(salt), iterations: ITERATIONS },
    material,
    { name: "AES-GCM", length: 256 },
    false,
    usages,
  );
}
function aad(value: Omit<EncryptedSyncEnvelope, "ciphertext"> | EncryptedSyncEnvelope): string {
  return JSON.stringify({
    format: value.format,
    algorithm: value.algorithm,
    kdf: value.kdf,
    iterations: value.iterations,
    createdAt: value.createdAt,
    deviceId: value.deviceId,
    salt: value.salt,
    iv: value.iv,
  });
}
function arrayBuffer(value: Uint8Array): ArrayBuffer {
  const copy = new Uint8Array(value.byteLength);
  copy.set(value);
  return copy.buffer;
}
function toBase64(value: Uint8Array): string {
  let binary = "";
  for (const byte of value) binary += String.fromCharCode(byte);
  return btoa(binary);
}
function fromBase64(value: string): Uint8Array {
  try {
    const binary = atob(value);
    return Uint8Array.from(binary, (character) => character.charCodeAt(0));
  } catch {
    throw new Error("Invalid base64 in encrypted sync file");
  }
}
