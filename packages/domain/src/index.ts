import { z } from "zod";

export const authoritySchema = z.enum([
  "user_confirmed",
  "external_fact",
  "ai_suggestion",
  "inferred",
]);

export const memoryStatusSchema = z.enum(["active", "draft", "superseded", "rejected", "archived"]);

export const memoryTypeSchema = z.enum([
  "decision",
  "requirement",
  "fact",
  "preference",
  "suggestion",
  "idea",
  "rejected_idea",
  "task",
  "bug",
  "question",
  "reference",
  "summary",
]);

export const scopeSchema = z.enum(["global", "project", "conversation", "task"]);
export const lifetimeSchema = z.enum([
  "one_prompt",
  "n_prompts",
  "session",
  "conversation",
  "manual",
]);

export const createMemorySchema = z.object({
  projectId: z.string().uuid().nullable(),
  memorySpaceId: z.string().uuid().nullable(),
  type: memoryTypeSchema,
  authority: authoritySchema,
  status: memoryStatusSchema,
  title: z.string().trim().min(1).max(160),
  content: z.string().trim().min(1).max(1_000_000),
});

export type Authority = z.infer<typeof authoritySchema>;
export type MemoryStatus = z.infer<typeof memoryStatusSchema>;
export type MemoryType = z.infer<typeof memoryTypeSchema>;
export type Scope = z.infer<typeof scopeSchema>;
export type Lifetime = z.infer<typeof lifetimeSchema>;
export type CreateMemoryInput = z.infer<typeof createMemorySchema>;
