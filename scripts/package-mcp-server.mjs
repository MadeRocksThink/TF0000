import { copyFileSync, cpSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { dirname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const applicationRoot = resolve(repositoryRoot, "apps", "mcp-server");
const destination = resolve(repositoryRoot, "target", "release", "tf0000-mcp");
const releaseRoot = resolve(repositoryRoot, "target", "release");

if (!destination.startsWith(`${releaseRoot}${sep}`)) {
  throw new Error(`Refusing to package to unexpected path: ${destination}`);
}

const bundle = resolve(applicationRoot, "dist", "tf0000-mcp.mjs");
const nativeBin = resolve(applicationRoot, "bin");
if (!existsSync(bundle) || !existsSync(nativeBin)) {
  throw new Error("Build the MCP bundle and prepare its native host before packaging");
}

rmSync(destination, { recursive: true, force: true });
mkdirSync(destination, { recursive: true });
copyFileSync(bundle, resolve(destination, "tf0000-mcp.mjs"));
copyFileSync(resolve(applicationRoot, "tf0000-mcp.cmd"), resolve(destination, "tf0000-mcp.cmd"));
copyFileSync(resolve(applicationRoot, "tf0000-mcp"), resolve(destination, "tf0000-mcp"));
cpSync(nativeBin, resolve(destination, "bin"), { recursive: true });
process.stdout.write(`Packaged ${destination}\n`);
