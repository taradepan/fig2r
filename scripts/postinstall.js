#!/usr/bin/env node
// Link the bundled fig2r agent skill into detected skill directories so
// `npm install -g fig2r@latest` keeps the skill in sync without a copy.
//
// Targets (always created — the skill is globally available even if the
// agent isn't installed yet; we only create the `skills/` subdir, never
// anything else inside the agent's config root):
//   ~/.claude/skills/fig2r     (Claude Code)
//   ~/.agents/skills/fig2r     (Cursor, Codex, Aider, Cline, etc.)
//
// Opt out:      FIG2R_SKIP_SKILL_INSTALL=1
// CI skip:      automatic when process.env.CI is set
// Idempotent:   existing correct symlink is left alone; existing directory
//               or symlink pointing elsewhere is respected and skipped.

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

if (process.env.FIG2R_SKIP_SKILL_INSTALL || process.env.CI) {
  process.exit(0);
}

const source = path.resolve(__dirname, "..", "skills", "fig2r");

if (!fs.existsSync(source) || !fs.statSync(source).isDirectory()) {
  // Not running from a layout that includes the skill (e.g. cargo-only dev).
  process.exit(0);
}

const home = os.homedir();
const targets = [
  { parent: path.join(home, ".claude", "skills"), label: "Claude Code" },
  { parent: path.join(home, ".agents", "skills"), label: "agents (.agents/)" },
];

let linked = 0;
for (const { parent, label } of targets) {
  try {
    fs.mkdirSync(parent, { recursive: true });
  } catch {
    continue;
  }

  const link = path.join(parent, "fig2r");

  try {
    const st = fs.lstatSync(link);
    if (st.isSymbolicLink()) {
      const resolved = path.resolve(parent, fs.readlinkSync(link));
      if (resolved === source) {
        linked++;
        continue;
      }
      console.log(`[fig2r] skill: ${link} is an existing symlink to ${resolved} — leaving in place`);
      continue;
    }
    if (st.isDirectory()) {
      console.log(`[fig2r] skill: ${link} exists as a directory — skipping (remove it to link the bundled skill)`);
      continue;
    }
    console.log(`[fig2r] skill: ${link} exists and is not a symlink — skipping`);
    continue;
  } catch {
    // Path doesn't exist — proceed to create the symlink.
  }

  try {
    const type = process.platform === "win32" ? "junction" : "dir";
    fs.symlinkSync(source, link, type);
    console.log(`[fig2r] skill linked: ${link} -> ${source} (${label})`);
    linked++;
  } catch (err) {
    console.log(`[fig2r] skill: could not symlink ${link}: ${err.message}`);
  }
}

if (linked > 0) {
  console.log(`[fig2r] skill available in ${linked} location(s). Restart your agent to pick it up.`);
}
