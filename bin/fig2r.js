#!/usr/bin/env node

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const binaryName = process.platform === "win32" ? "fig2r-native.exe" : "fig2r-native";
const binaryPath = path.join(__dirname, binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(
    "[fig2r] Native binary is missing. Reinstall fig2r or run with FIG2R_SKIP_DOWNLOAD unset."
  );
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
});

child.on("error", (error) => {
  console.error(`[fig2r] Failed to start binary: ${error.message}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
