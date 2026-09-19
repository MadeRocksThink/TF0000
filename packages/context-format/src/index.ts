export const PORTABLE_FORMAT = "tf0000-project/v1" as const;

export interface PortableProject {
  id: string;
  name: string;
  description: string;
  createdAt: string;
  archivedAt: string | null;
}
export interface PortableSpace {
  id: string;
  projectId: string | null;
  parentId: string | null;
  name: string;
  description: string;
  defaultScope: string;
  createdAt: string;
  archivedAt: string | null;
}
export interface PortableVersion {
  id: string;
  memoryId: string;
  content: string;
  changeType: string;
  createdAt: string;
  supersedesVersionId: string | null;
}
export interface PortableSource {
  memoryVersionId: string;
  sourceType: string;
  sourceId: string;
  sourceHash: string;
  sourceLabel: string;
  sourceExcerpt: string;
}
export interface PortableMemoryRecord {
  memory: {
    id: string;
    projectId: string | null;
    memoryType: string;
    authority: string;
    status: string;
    title: string;
    currentVersionId: string;
    currentContent: string;
    createdAt: string;
  };
  versions: PortableVersion[];
  sources: PortableSource[];
  memorySpaceIds: string[];
}
export interface PortablePack {
  pack: {
    id: string;
    projectId: string | null;
    name: string;
    description: string;
    currentVersion: number;
    createdAt: string;
    updatedAt: string;
  };
  items: Array<{ targetId: string; ordering: number; inclusionMode: string }>;
}
export interface PortableWorkspace {
  format: typeof PORTABLE_FORMAT;
  schemaVersion: 1;
  exportedAt: string;
  sourceDeviceId: string;
  projects: PortableProject[];
  memorySpaces: PortableSpace[];
  memories: PortableMemoryRecord[];
  contextPacks: PortablePack[];
}

export interface ContextItem {
  id: string;
  title: string;
  content: string;
  sourceLabel?: string;
}
export function formatContextPack(name: string, items: readonly ContextItem[]): string {
  const sections = items.map((item, index) => {
    const source = item.sourceLabel ? `\nSource: ${item.sourceLabel}` : "";
    return `## ${index + 1}. ${item.title}${source}\n\n${item.content.trim()}`;
  });
  return `# TF0000 Context Pack: ${name.trim()}\n\n${sections.join("\n\n")}`;
}
export function estimateTokens(text: string): number {
  return Math.max(1, Math.ceil(text.length / 4));
}

export function validatePortableWorkspace(value: unknown): asserts value is PortableWorkspace {
  if (!isRecord(value) || value.format !== PORTABLE_FORMAT || value.schemaVersion !== 1)
    throw new Error("Unsupported TF0000 portable format");
  requiredString(value.exportedAt, "exportedAt", 64);
  requiredString(value.sourceDeviceId, "sourceDeviceId", 128);
  const projects = requiredArray(value.projects, "projects", 10_000);
  const spaces = requiredArray(value.memorySpaces, "memorySpaces", 50_000);
  const memories = requiredArray(value.memories, "memories", 100_000);
  const packs = requiredArray(value.contextPacks, "contextPacks", 50_000);
  const projectIds = new Set(
    projects.map((item, i) => entity(item, `projects[${i}]`, ["id", "name", "createdAt"]).id),
  );
  const spaceIds = new Set(
    spaces.map((item, i) => entity(item, `memorySpaces[${i}]`, ["id", "name", "createdAt"]).id),
  );
  for (const [index, item] of memories.entries()) {
    if (!isRecord(item)) throw new Error(`memories[${index}] must be an object`);
    const memory = entity(item.memory, `memories[${index}].memory`, [
      "id",
      "title",
      "currentVersionId",
      "currentContent",
      "createdAt",
    ]);
    const versions = requiredArray(item.versions, `memories[${index}].versions`, 100_000);
    if (!versions.some((version) => isRecord(version) && version.id === memory.currentVersionId))
      throw new Error(`memories[${index}] current version is missing`);
    for (const id of requiredArray(
      item.memorySpaceIds,
      `memories[${index}].memorySpaceIds`,
      10_000,
    )) {
      if (typeof id !== "string" || !spaceIds.has(id))
        throw new Error(`memories[${index}] references an unknown space`);
    }
  }
  for (const [index, item] of packs.entries()) {
    if (!isRecord(item)) throw new Error(`contextPacks[${index}] must be an object`);
    entity(item.pack, `contextPacks[${index}].pack`, ["id", "name", "createdAt", "updatedAt"]);
    requiredArray(item.items, `contextPacks[${index}].items`, 100_000);
  }
  for (const [index, space] of spaces.entries()) {
    if (!isRecord(space)) continue;
    if (
      space.projectId !== null &&
      (typeof space.projectId !== "string" || !projectIds.has(space.projectId))
    )
      throw new Error(`memorySpaces[${index}] references an unknown project`);
  }
}

export function serializePortableWorkspace(workspace: PortableWorkspace): string {
  validatePortableWorkspace(workspace);
  return `${JSON.stringify(workspace, null, 2)}\n`;
}
export function parsePortableWorkspace(json: string): PortableWorkspace {
  if (new TextEncoder().encode(json).byteLength > 100 * 1024 * 1024)
    throw new Error("Portable workspace exceeds 100 MB");
  const value: unknown = JSON.parse(json);
  validatePortableWorkspace(value);
  return value;
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function requiredString(value: unknown, field: string, max: number): string {
  if (typeof value !== "string" || !value.trim() || value.length > max)
    throw new Error(`${field} must be a non-empty string up to ${max} characters`);
  return value;
}
function requiredArray(value: unknown, field: string, max: number): unknown[] {
  if (!Array.isArray(value) || value.length > max)
    throw new Error(`${field} must be an array with at most ${max} items`);
  return value;
}
function entity(value: unknown, field: string, strings: string[]): Record<string, unknown> {
  if (!isRecord(value)) throw new Error(`${field} must be an object`);
  for (const key of strings)
    requiredString(value[key], `${field}.${key}`, key === "currentContent" ? 10_000_000 : 10_000);
  return value;
}

export { default as portableWorkspaceJsonSchema } from "../schema/project-v1.schema.json";
