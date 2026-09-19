import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const executable = process.platform === "win32" ? "tf0000-native-host.exe" : "tf0000-native-host";
const source = resolve(repositoryRoot, "target", "release", executable);
const destination = resolve(repositoryRoot, "apps", "mcp-server", "bin", executable);

if (!existsSync(source)) {
  throw new Error(`Build the native host before packaging the MCP server: ${source}`);
}

mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
process.stdout.write(`Prepared ${destination}\n`);
