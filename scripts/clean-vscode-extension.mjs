import { rmSync } from "node:fs";
import { dirname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const extensionRoot = resolve(repositoryRoot, "apps", "vscode-extension");
const destination = resolve(extensionRoot, "dist");

if (!destination.startsWith(`${extensionRoot}${sep}`)) {
  throw new Error(`Refusing to clean unexpected path: ${destination}`);
}

rmSync(destination, { recursive: true, force: true });
