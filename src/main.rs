#![deny(unsafe_code)]

mod cli;
mod codegen;
mod emit;
mod error;
mod figma;
mod ir;
mod tailwind;
mod warning;

use clap::Parser;
use mimalloc::MiMalloc;
use std::io::Read;
use std::process;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use cli::{Cli, Command};
use codegen::tree::{ConvertOptions, IconLibrary, NamingStyle, SvgMode, generate_file_tree};
use emit::writer::write_files;
use ir::validate::validate_ir;
use warning::WarningCollector;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Fetch {
            url,
            token,
            save,
            naming,
            svg_mode,
            no_theme,
            no_index,
            flat,
            cn_import,
            public_dir,
            quiet,
            icon_library,
            responsive,
        } => {
            let api_token = match figma::config::resolve_token(token.as_deref()) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("[ERROR] {e}");
                    process::exit(2);
                }
            };

            let figma_ref = match figma::url::parse_figma_url(&url) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[ERROR] {e}");
                    process::exit(2);
                }
            };

            if !quiet {
                eprintln!(
                    "[INFO] Fetching from Figma: file={} node={:?}",
                    figma_ref.file_key, figma_ref.node_id
                );
            }

            let client = figma::api::FigmaClient::new(api_token);
            let node_ids: Vec<&str> = figma_ref
                .node_id
                .as_deref()
                .map(|id| vec![id])
                .unwrap_or_default();

            let response = match client
                .get_nodes(
                    &figma_ref.file_key,
                    if node_ids.is_empty() {
                        None
                    } else {
                        Some(&node_ids)
                    },
                )
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("[ERROR] {e}");
                    process::exit(2);
                }
            };

            let figma_node = response
                .nodes
                .values()
                .find_map(|v| v.as_ref().map(|c| &c.document))
                .unwrap_or_else(|| {
                    eprintln!("[ERROR] No nodes found in Figma response");
                    process::exit(2);
                });

            let mut design_ir = figma::transform::figma_to_ir(&response.name, figma_node);

            if let Err(e) = validate_ir(&design_ir) {
                eprintln!("[ERROR] Invalid IR from transform: {e}");
                process::exit(2);
            }

            let mut failed_downloads = 0usize;

            if save.is_some() && !design_ir.assets.is_empty() {
                let png_ids: Vec<&str> = design_ir
                    .assets
                    .iter()
                    .filter(|a| a.format == "png")
                    .map(|a| a.id.as_str())
                    .collect();
                let svg_ids: Vec<&str> = design_ir
                    .assets
                    .iter()
                    .filter(|a| a.format == "svg")
                    .map(|a| a.id.as_str())
                    .collect();

                if !quiet {
                    eprintln!(
                        "[INFO] Exporting {} PNGs + {} SVGs from Figma...",
                        png_ids.len(),
                        svg_ids.len()
                    );
                }

                // Fetch PNG and SVG export URLs in parallel
                let file_key = &figma_ref.file_key;
                let mut all_tasks: Vec<(String, String)> = Vec::new();

                let (png_results, svg_results) = tokio::join!(
                    fetch_export_urls(&client, file_key, &png_ids, "png", 2.0),
                    fetch_export_urls(&client, file_key, &svg_ids, "svg", 1.0),
                );
                all_tasks.extend(png_results);
                all_tasks.extend(svg_results);

                if !quiet {
                    eprintln!(
                        "[INFO] Downloading {} assets in parallel...",
                        all_tasks.len()
                    );
                }

                // Download all concurrently
                let results = client.download_images_parallel(&all_tasks).await;

                for result in results {
                    if let Some(asset) = design_ir.assets.iter_mut().find(|a| a.id == result.id) {
                        asset.url = Some(result.url);
                        match result.data {
                            Ok(bytes) => {
                                if asset.format == "svg" {
                                    match String::from_utf8(bytes) {
                                        Ok(s) => asset.data = s,
                                        Err(_) => {
                                            failed_downloads += 1;
                                            if !quiet {
                                                eprintln!(
                                                    "[WARN] SVG {} is not valid UTF-8",
                                                    asset.id
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    asset.data = base64::Engine::encode(
                                        &base64::engine::general_purpose::STANDARD,
                                        &bytes,
                                    );
                                }
                            }
                            Err(e) => {
                                failed_downloads += 1;
                                if !quiet {
                                    eprintln!("[WARN] Failed {}: {e}", asset.id);
                                }
                            }
                        }
                    }
                }

                let downloaded = design_ir
                    .assets
                    .iter()
                    .filter(|a| !a.data.is_empty())
                    .count();
                if !quiet {
                    eprintln!(
                        "[INFO] Downloaded {}/{} assets",
                        downloaded,
                        design_ir.assets.len()
                    );
                }
            }

            if let Some(output_dir) = save {
                let mut warnings = WarningCollector::new();
                let opts = make_convert_options(
                    cn_import,
                    icon_library,
                    responsive,
                    flat,
                    no_index,
                    no_theme,
                    naming,
                    svg_mode,
                );

                let files = generate_file_tree(&design_ir, &opts, &mut warnings);

                if !quiet {
                    for w in warnings.warnings() {
                        eprintln!("{w}");
                    }
                }

                let (asset_files, component_files): (Vec<_>, Vec<_>) = files
                    .into_iter()
                    .partition(|f| f.path.starts_with("assets/"));

                if let Err(e) = write_files(&output_dir, &component_files) {
                    eprintln!("[ERROR] Failed to write output: {e}");
                    process::exit(2);
                }
                if let Some(public_dir) = public_dir {
                    if let Err(e) = write_files(&public_dir, &asset_files) {
                        eprintln!("[ERROR] Failed to write public assets: {e}");
                        process::exit(2);
                    }
                } else if let Err(e) = write_files(&output_dir, &asset_files) {
                    eprintln!("[ERROR] Failed to write output assets: {e}");
                    process::exit(2);
                }

                if !quiet {
                    eprintln!(
                        "[OK] Generated {} files in {}",
                        component_files.len() + asset_files.len(),
                        output_dir.display()
                    );
                }
                let stats = count_nodes(&design_ir);
                eprintln!(
                    "[SUMMARY] nodes={} texts={} images={} failed_downloads={}",
                    stats.total, stats.texts, stats.images, failed_downloads
                );
            } else {
                match serde_json::to_string_pretty(&design_ir) {
                    Ok(json) => println!("{json}"),
                    Err(e) => {
                        eprintln!("[ERROR] Failed to serialize IR: {e}");
                        process::exit(2);
                    }
                }
                if !quiet {
                    eprintln!("[OK] IR JSON printed to stdout");
                }
                let stats = count_nodes(&design_ir);
                eprintln!(
                    "[SUMMARY] nodes={} texts={} images={} failed_downloads=0",
                    stats.total, stats.texts, stats.images
                );
            }
        }

        Command::Convert {
            input,
            output,
            strict,
            naming,
            svg_mode,
            no_theme,
            no_index,
            flat,
            cn_import,
            public_dir,
            quiet,
            icon_library,
            responsive,
        } => {
            let json = read_input(input.as_deref());
            let ir: ir::schema::DesignIR = match serde_json::from_str(&json) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("[ERROR] Failed to parse IR JSON: {e}");
                    process::exit(2);
                }
            };

            if let Err(e) = validate_ir(&ir) {
                eprintln!("[ERROR] Invalid IR: {e}");
                process::exit(2);
            }

            let mut warnings = WarningCollector::new();
            let opts = make_convert_options(
                cn_import,
                icon_library,
                responsive,
                flat,
                no_index,
                no_theme,
                naming,
                svg_mode,
            );

            let files = generate_file_tree(&ir, &opts, &mut warnings);

            if !quiet {
                for w in warnings.warnings() {
                    eprintln!("{w}");
                }
            }

            if strict && warnings.has_warnings() {
                eprintln!(
                    "[ERROR] Strict mode: {} warnings found, aborting",
                    warnings.warnings().len()
                );
                process::exit(1);
            }

            let (asset_files, component_files): (Vec<_>, Vec<_>) = files
                .into_iter()
                .partition(|f| f.path.starts_with("assets/"));

            if let Err(e) = write_files(&output, &component_files) {
                eprintln!("[ERROR] Failed to write output: {e}");
                process::exit(2);
            }
            if let Some(public_dir) = public_dir {
                if let Err(e) = write_files(&public_dir, &asset_files) {
                    eprintln!("[ERROR] Failed to write public assets: {e}");
                    process::exit(2);
                }
            } else if let Err(e) = write_files(&output, &asset_files) {
                eprintln!("[ERROR] Failed to write output assets: {e}");
                process::exit(2);
            }

            if !quiet {
                eprintln!(
                    "[OK] Generated {} files in {}",
                    component_files.len() + asset_files.len(),
                    output.display()
                );
            }
            let stats = count_nodes(&ir);
            eprintln!(
                "[SUMMARY] nodes={} texts={} images={} failed_downloads=0",
                stats.total, stats.texts, stats.images
            );
        }

        Command::Validate { input } => {
            let json = read_input(input.as_deref());
            let ir: ir::schema::DesignIR = match serde_json::from_str(&json) {
                Ok(ir) => ir,
                Err(e) => {
                    eprintln!("[ERROR] Failed to parse IR JSON: {e}");
                    process::exit(1);
                }
            };
            match validate_ir(&ir) {
                Ok(()) => {
                    eprintln!(
                        "[OK] IR is valid ({} components, {} assets)",
                        ir.components.len(),
                        ir.assets.len()
                    );
                }
                Err(e) => {
                    eprintln!("[ERROR] Invalid IR: {e}");
                    process::exit(1);
                }
            }
        }

        Command::Auth { token } => match figma::config::save_token(&token) {
            Ok(()) => eprintln!("[OK] Token saved to ~/.fig2r/config.toml"),
            Err(e) => {
                eprintln!("[ERROR] Failed to save token: {e}");
                process::exit(2);
            }
        },
    }
}

/// Fetch export URLs for a set of node IDs, batched in chunks of 100.
async fn fetch_export_urls(
    client: &figma::api::FigmaClient,
    file_key: &str,
    ids: &[&str],
    format: &str,
    scale: f64,
) -> Vec<(String, String)> {
    let mut tasks = Vec::new();
    for chunk in ids.chunks(100) {
        if let Ok(resp) = client.get_image_urls(file_key, chunk, format, scale).await {
            for (id, url) in resp.images {
                if let Some(u) = url {
                    tasks.push((id, u));
                }
            }
        }
    }
    tasks
}

#[allow(clippy::too_many_arguments)]
fn make_convert_options(
    cn_import: String,
    icon_library: cli::IconLibrary,
    responsive: bool,
    flat: bool,
    no_index: bool,
    no_theme: bool,
    naming: cli::NamingStyle,
    svg_mode: cli::SvgMode,
) -> ConvertOptions {
    ConvertOptions {
        cn_import,
        asset_public_base: "/assets".into(),
        icon_library: match icon_library {
            cli::IconLibrary::None => IconLibrary::None,
            cli::IconLibrary::Phosphor => IconLibrary::Phosphor,
            cli::IconLibrary::Lucide => IconLibrary::Lucide,
            cli::IconLibrary::Heroicons => IconLibrary::Heroicons,
        },
        responsive,
        flat,
        no_index,
        no_theme,
        naming: match naming {
            cli::NamingStyle::Pascal => NamingStyle::Pascal,
            cli::NamingStyle::Kebab => NamingStyle::Kebab,
        },
        svg_mode: match svg_mode {
            cli::SvgMode::ReactComponent => SvgMode::ReactComponent,
            cli::SvgMode::File => SvgMode::File,
            cli::SvgMode::Inline => SvgMode::Inline,
        },
    }
}

fn read_input(path: Option<&std::path::Path>) -> String {
    match path {
        Some(p) => std::fs::read_to_string(p).unwrap_or_else(|e| {
            eprintln!("[ERROR] Cannot read {}: {e}", p.display());
            process::exit(2);
        }),
        None => {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .unwrap_or_else(|e| {
                    eprintln!("[ERROR] Cannot read stdin: {e}");
                    process::exit(2);
                });
            buf
        }
    }
}

#[derive(Default)]
struct NodeStats {
    total: usize,
    texts: usize,
    images: usize,
}

fn count_nodes(ir: &ir::schema::DesignIR) -> NodeStats {
    fn walk(node: &ir::schema::Node, stats: &mut NodeStats) {
        stats.total += 1;
        match node.node_type {
            ir::schema::NodeType::Text => stats.texts += 1,
            ir::schema::NodeType::Image => stats.images += 1,
            _ => {}
        }
        for child in &node.children {
            walk(child, stats);
        }
    }

    let mut stats = NodeStats::default();
    for component in &ir.components {
        walk(component, &mut stats);
    }
    stats
}
