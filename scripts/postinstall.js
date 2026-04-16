#!/usr/bin/env node

const fs = require("node:fs");
const fsp = require("node:fs/promises");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const tar = require("tar");

const TARGETS = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-arm64": "aarch64-unknown-linux-gnu",
  "linux-x64": "x86_64-unknown-linux-gnu",
  "win32-arm64": "aarch64-pc-windows-msvc",
  "win32-x64": "x86_64-pc-windows-msvc",
};

function getPackageVersion() {
  const pkgPath = path.resolve(__dirname, "..", "package.json");
  return JSON.parse(fs.readFileSync(pkgPath, "utf8")).version;
}

function resolveDownloadUrl(target, version) {
  const custom = process.env.FIG2R_BINARY_BASE_URL;
  const baseUrl =
    custom || "https://github.com/taradepan/fig2r/releases/download";
  const tag = `v${version}`;
  const archive = `fig2r-${target}.tar.gz`;
  return `${baseUrl}/${tag}/${archive}`;
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (response) => {
      if (
        response.statusCode &&
        response.statusCode >= 300 &&
        response.statusCode < 400 &&
        response.headers.location
      ) {
        response.resume();
        download(response.headers.location, destination).then(resolve, reject);
        return;
      }

      if (response.statusCode !== 200) {
        response.resume();
        reject(
          new Error(`Download failed with status ${response.statusCode}: ${url}`)
        );
        return;
      }

      const file = fs.createWriteStream(destination);
      response.pipe(file);
      file.on("finish", () => file.close(resolve));
      file.on("error", (error) => reject(error));
    });

    request.on("error", (error) => reject(error));
    request.setTimeout(30_000, () => {
      request.destroy(new Error("Download timed out"));
    });
  });
}

async function findBinary(rootDir) {
  const expected = process.platform === "win32" ? "fig2r.exe" : "fig2r";
  const stack = [rootDir];

  while (stack.length > 0) {
    const current = stack.pop();
    const entries = await fsp.readdir(current, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(fullPath);
        continue;
      }
      if (entry.isFile() && entry.name === expected) {
        return fullPath;
      }
    }
  }

  throw new Error(`Archive does not contain ${expected}`);
}

async function main() {
  if (process.env.FIG2R_SKIP_DOWNLOAD === "1") {
    console.log("[fig2r] Skipping native binary download (FIG2R_SKIP_DOWNLOAD=1)");
    return;
  }

  const targetKey = `${process.platform}-${process.arch}`;
  const target = TARGETS[targetKey];
  if (!target) {
    throw new Error(`Unsupported platform/arch: ${targetKey}`);
  }

  const version = getPackageVersion();
  const url = resolveDownloadUrl(target, version);
  const packageRoot = path.resolve(__dirname, "..");
  const binDir = path.join(packageRoot, "bin");
  const outputName = process.platform === "win32" ? "fig2r-native.exe" : "fig2r-native";
  const outputPath = path.join(binDir, outputName);

  await fsp.mkdir(binDir, { recursive: true });

  const tempDir = await fsp.mkdtemp(path.join(os.tmpdir(), "fig2r-install-"));
  const archivePath = path.join(tempDir, "fig2r.tar.gz");
  const extractDir = path.join(tempDir, "extract");
  await fsp.mkdir(extractDir, { recursive: true });

  try {
    console.log(`[fig2r] Downloading ${url}`);
    await download(url, archivePath);
    await tar.x({ file: archivePath, cwd: extractDir });

    const extractedBinary = await findBinary(extractDir);
    await fsp.copyFile(extractedBinary, outputPath);
    if (process.platform !== "win32") {
      await fsp.chmod(outputPath, 0o755);
    }
    console.log(`[fig2r] Installed ${outputName}`);
  } finally {
    await fsp.rm(tempDir, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(`[fig2r] Install failed: ${error.message}`);
  process.exit(1);
});
