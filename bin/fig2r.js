#!/usr/bin/env node
// Dispatcher: locate the per-platform sub-package installed via npm
// optionalDependencies and exec the bundled binary.

const { spawn } = require("node:child_process");
const { chmodSync, existsSync, statSync } = require("node:fs");
const { createRequire } = require("node:module");
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
