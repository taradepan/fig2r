#!/usr/bin/env node
// Dispatcher: locate the per-platform sub-package installed via npm
// optionalDependencies and exec the bundled binary.

const { spawn } = require("node:child_process");
const { chmodSync, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } = require("node:fs");
const { createRequire } = require("node:module");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");

const requireFromHere = createRequire(__filename);

const PLATFORM_PACKAGES = {
  "darwin-arm64": { target: "aarch64-apple-darwin", pkg: "@taradepan1313/fig2r-darwin-arm64" },
  "darwin-x64":   { target: "x86_64-apple-darwin",  pkg: "@taradepan1313/fig2r-darwin-x64" },
  "linux-arm64":  { target: "aarch64-unknown-linux-gnu", pkg: "@taradepan1313/fig2r-linux-arm64" },
  "linux-x64":    { target: "x86_64-unknown-linux-gnu",  pkg: "@taradepan1313/fig2r-linux-x64" },
  "win32-arm64":  { target: "aarch64-pc-windows-msvc", pkg: "@taradepan1313/fig2r-win32-arm64" },
  "win32-x64":    { target: "x86_64-pc-windows-msvc",  pkg: "@taradepan1313/fig2r-win32-x64" },
};

const key = `${process.platform}-${process.arch}`;
const entry = PLATFORM_PACKAGES[key];

if (!entry) {
  console.error(
    `[fig2r] Unsupported platform: ${process.platform}/${process.arch}.\n` +
    `Supported: ${Object.keys(PLATFORM_PACKAGES).join(", ")}`
  );
  process.exit(1);
}

const binaryName = process.platform === "win32" ? "fig2r.exe" : "fig2r";

let binaryPath;
try {
  const subpackageJson = requireFromHere.resolve(`${entry.pkg}/package.json`);
  binaryPath = path.join(path.dirname(subpackageJson), "bin", binaryName);
} catch {
  console.error(
    `[fig2r] Missing optional dependency "${entry.pkg}".\n` +
    `Reinstall fig2r:\n  npm install -g fig2r`
  );
  process.exit(1);
}

if (!existsSync(binaryPath)) {
  console.error(`[fig2r] Binary not found at ${binaryPath}`);
  process.exit(1);
}

// npm tarballs strip the executable bit from files not declared in a `bin`
// field. Restore it here so the spawn doesn't EACCES on first run.
if (process.platform !== "win32") {
  try {
    const mode = statSync(binaryPath).mode;
    if (!(mode & 0o111)) chmodSync(binaryPath, 0o755);
  } catch { /* best-effort */ }
}

maybeCheckForUpdate();

const child = spawn(binaryPath, process.argv.slice(2), { stdio: "inherit" });

for (const sig of ["SIGINT", "SIGTERM", "SIGHUP"]) {
  process.on(sig, () => {
    try { child.kill(sig); } catch { /* ignore */ }
  });
}

child.on("error", (err) => {
  console.error(`[fig2r] Failed to launch binary: ${err.message}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  else process.exit(code ?? 1);
});

// Update notifier: prints a notice when a newer fig2r is on npm.
// Reads cached result synchronously (fast); fires a background fetch at most
// once every 24h to refresh the cache. Non-blocking — never delays the CLI.
// Disable via NO_UPDATE_NOTIFIER=1 or when CI is set.
function maybeCheckForUpdate() {
  if (process.env.CI || process.env.NO_UPDATE_NOTIFIER) return;

  const cacheFile = path.join(os.homedir(), ".fig2r", "update-check.json");
  const ONE_DAY = 24 * 60 * 60 * 1000;
  const pkgJson = requireFromHere("../package.json");
  const current = pkgJson.version;

  let cache = {};
  try { cache = JSON.parse(readFileSync(cacheFile, "utf8")); } catch { /* no cache yet */ }

  if (cache.latest && isNewer(cache.latest, current)) {
    process.stderr.write(
      `\n[fig2r] Update available: ${current} → ${cache.latest}\n` +
      `        Run: npm install -g fig2r@latest\n\n`
    );
  }

  if (Date.now() - (cache.checkedAt || 0) < ONE_DAY) return;

  const req = https.get(
    `https://registry.npmjs.org/${pkgJson.name}/latest`,
    { timeout: 3000, headers: { accept: "application/json" } },
    (res) => {
      if (res.statusCode !== 200) { res.resume(); return; }
      let body = "";
      res.on("data", (chunk) => { body += chunk; });
      res.on("end", () => {
        try {
          const latest = JSON.parse(body).version;
          if (typeof latest !== "string") return;
          mkdirSync(path.dirname(cacheFile), { recursive: true });
          writeFileSync(cacheFile, JSON.stringify({ latest, checkedAt: Date.now() }));
        } catch { /* best-effort */ }
      });
    }
  );
  req.on("error", () => { /* offline or registry hiccup — ignore */ });
  req.on("timeout", () => req.destroy());
  req.on("socket", (s) => s.unref());
}

// Lightweight semver >: compares x.y.z only, ignores pre-release tags.
function isNewer(a, b) {
  const pa = String(a).split("-")[0].split(".").map((n) => parseInt(n, 10) || 0);
  const pb = String(b).split("-")[0].split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < 3; i++) {
    if ((pa[i] || 0) > (pb[i] || 0)) return true;
    if ((pa[i] || 0) < (pb[i] || 0)) return false;
  }
  return false;
}
