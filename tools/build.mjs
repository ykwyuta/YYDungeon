import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
const cargo = spawnSync("cargo", ["build", "--locked", "--release", "--lib", "--target", "wasm32-unknown-unknown"], {
  cwd: fileURLToPath(root), stdio: "inherit",
});
if (cargo.error) throw cargo.error;
if (cargo.status !== 0) process.exit(cargo.status || 1);
const dist = new URL("dist/", root);
mkdirSync(dist, { recursive: true });
for (const [src, dest] of [
  ["web/index.html", "index.html"],
  ["web/main.js", "main.js"],
  ["web/_headers", "_headers"],
  ["web/manifest.webmanifest", "manifest.webmanifest"],
  ["web/icon-192.png", "icon-192.png"],
  ["web/icon-512.png", "icon-512.png"],
  ["web/apple-touch-icon.png", "apple-touch-icon.png"],
  ["assets/app_icon.ico", "favicon.ico"],
  ["target/wasm32-unknown-unknown/release/yydungeon.wasm", "yydungeon.wasm"],
]) {
  copyFileSync(new URL(src, root), new URL(dest, dist));
}
