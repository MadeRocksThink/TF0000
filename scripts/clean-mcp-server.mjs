import { rmSync } from "node:fs";
import { dirname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const applicationRoot = resolve(repositoryRoot, "apps", "mcp-server");
const destination = resolve(applicationRoot, "dist");

if (!destination.startsWith(`${applicationRoot}${sep}`)) {
  throw new Error(`Refusing to clean unexpected path: ${destination}`);
}

rmSync(destination, { recursive: true, force: true });
