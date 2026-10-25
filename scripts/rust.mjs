import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";
const root = resolve(import.meta.dirname, "../../..", "work/toolchain");
const env = { ...process.env };
if (existsSync(resolve(root, "cargo/bin/cargo"))) {
  env.CARGO_HOME = resolve(root, "cargo");
  env.RUSTUP_HOME = resolve(root, "rustup");
  env.PATH = `${env.CARGO_HOME}/bin:${env.PATH}`;
}
const result = spawnSync("cargo", process.argv.slice(2), {
  stdio: "inherit",
  env,
});
if (result.error) console.error(result.error.message);
process.exit(result.status ?? 1);
