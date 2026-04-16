use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// fig2r - Convert Figma designs to React + Tailwind components
///
/// Reads an Intermediate Representation (IR) JSON file describing a Figma design
/// and generates production-ready React components with Tailwind CSS classes.
///
/// Two modes:
///   1. `fig2r fetch <figma-url>` — fetch directly from Figma API and convert
///   2. `fig2r convert <ir.json>` — convert from pre-built IR JSON
///
/// Supports stdin: echo '{"version":"1.0",...}' | fig2r convert -o ./src/components
#[derive(Parser, Debug)]
#[command(name = "fig2r", version, about, long_about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Fetch a Figma design and convert to React + Tailwind components
    ///
    /// Fetches design data directly from the Figma API, transforms it to IR,
    /// and generates React/Tailwind components. Prints IR JSON to stdout by default
    /// (for piping to `fig2r convert` or reading by AI agents).
    /// Use `--save` to write component files directly.
    Fetch {
        /// Figma URL or file key (e.g., https://www.figma.com/design/KEY/Name?node-id=1-2)
        url: String,

        /// Figma API token (overrides FIGMA_TOKEN env var and ~/.fig2r/config.toml)
        #[arg(long)]
        token: Option<String>,

        /// Write component files to this directory instead of printing IR JSON
        #[arg(long)]
        save: Option<PathBuf>,

        /// Component naming convention (only with --save)
        #[arg(long, value_enum, default_value = "pascal")]
        naming: NamingStyle,

        /// How to handle SVG assets (only with --save)
        #[arg(long, value_enum, default_value = "react-component")]
        svg_mode: SvgMode,

        /// Skip theme/token extraction (only with --save)
        #[arg(long)]
        no_theme: bool,

        /// Skip index.ts re-export files (only with --save)
        #[arg(long)]
        no_index: bool,

        /// Flat output, no subdirectories (only with --save)
        #[arg(long)]
        flat: bool,

        /// Import path for cn() utility (only with --save)
        #[arg(long, default_value = "../utils/cn")]
        cn_import: String,

        /// Write generated assets to this public directory (e.g. ./public)
        #[arg(long)]
        public_dir: Option<PathBuf>,

        /// Suppress non-error logs and warnings
        #[arg(long)]
        quiet: bool,

        /// Suggest icon imports from a specific icon library in JSX comments
        #[arg(long, value_enum, default_value = "none")]
        icon_library: IconLibrary,

        /// Make root containers responsive (`w-full max-w-[Npx]`)
        #[arg(long)]
        responsive: bool,
    },

    /// Convert IR JSON to React + Tailwind components
    ///
    /// Reads the IR JSON from a file or stdin, then generates:
    ///   - React .tsx components with Tailwind classes
    ///   - SVG icons as React components
    ///   - Image assets extracted to files
    ///   - Tailwind theme config from design tokens
    ///   - TypeScript token constants
    ///   - index.ts re-export files
    Convert {
        /// Path to IR JSON file (omit to read from stdin)
        input: Option<PathBuf>,

        /// Output directory for generated components
        #[arg(short, long, default_value = "./components")]
        output: PathBuf,

        /// Fail on any unsupported or ambiguous construct (for CI/quality gates)
        #[arg(long)]
        strict: bool,

        /// Component naming convention
        #[arg(long, value_enum, default_value = "pascal")]
        naming: NamingStyle,

        /// How to handle SVG assets
        #[arg(long, value_enum, default_value = "react-component")]
        svg_mode: SvgMode,

        /// Skip theme/token extraction
        #[arg(long)]
        no_theme: bool,

        /// Skip index.ts re-export files
        #[arg(long)]
        no_index: bool,

        /// Flat output (no subdirectories per component)
        #[arg(long)]
        flat: bool,

        /// Import path for the `cn()` utility function
        #[arg(long, default_value = "../utils/cn")]
        cn_import: String,

        /// Write generated assets to this public directory (e.g. ./public)
        #[arg(long)]
        public_dir: Option<PathBuf>,

        /// Suppress non-error logs and warnings
        #[arg(long)]
        quiet: bool,

        /// Suggest icon imports from a specific icon library in JSX comments
        #[arg(long, value_enum, default_value = "none")]
        icon_library: IconLibrary,

        /// Make root containers responsive (`w-full max-w-[Npx]`)
        #[arg(long)]
        responsive: bool,
    },

    /// Validate an IR JSON file without converting
    ///
    /// Checks that the IR JSON is well-formed and contains valid structure.
    /// Useful for debugging IR generation in your pipeline.
    Validate {
        /// Path to IR JSON file (omit to read from stdin)
        input: Option<PathBuf>,
    },

    /// Save your Figma API token for future use
    ///
    /// Stores the token in ~/.fig2r/config.toml.
    /// Generate a token at: https://www.figma.com/settings → Security
    /// Required scope: file_content:read
    Auth {
        /// Your Figma Personal Access Token
        token: String,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum NamingStyle {
    Pascal,
    Kebab,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SvgMode {
    ReactComponent,
    File,
    Inline,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
pub enum IconLibrary {
    None,
    Phosphor,
    Lucide,
    Heroicons,
}
