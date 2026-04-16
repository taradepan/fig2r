use crate::codegen::{asset, component, index, theme};
use crate::emit::formatter::{sanitize_component_name, to_kebab_case};
use crate::ir::schema::{Asset, AssetType, DesignIR};
use crate::warning::WarningCollector;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingStyle {
    Pascal,
    Kebab,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SvgMode {
    ReactComponent,
    File,
    Inline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconLibrary {
    None,
    Phosphor,
    Lucide,
    Heroicons,
}

#[derive(Debug, Clone)]
pub struct OutputFile {
    pub path: String,
    pub content: String,
    pub binary: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ConvertOptions {
    pub cn_import: String,
    pub asset_public_base: String,
    pub icon_library: IconLibrary,
    pub responsive: bool,
    pub flat: bool,
    pub no_index: bool,
    pub no_theme: bool,
    pub naming: NamingStyle,
    pub svg_mode: SvgMode,
}

pub fn generate_file_tree(
    ir: &DesignIR,
    opts: &ConvertOptions,
    warnings: &mut WarningCollector,
) -> Vec<OutputFile> {
    let mut files = Vec::new();
    let theme_colors = ir.theme.as_ref().and_then(|t| t.colors.clone());

    // Build asset ID → filename map so components can resolve image refs
    // Build asset ID → filename map with deduplication for shared image refs.
    let mut asset_map: component::AssetMap = component::AssetMap::new();
    let mut dedup_filename_by_key = std::collections::HashMap::<String, String>::new();
    let mut used_names = HashSet::<String>::new();
    for (i, a) in ir.assets.iter().enumerate() {
        let dedup_key = asset_dedup_key(a);
        if let Some(existing) = dedup_filename_by_key.get(&dedup_key) {
            asset_map.insert(a.id.clone(), existing.clone());
            continue;
        }

        let mut filename = asset::asset_filename(&a.name, &a.format);
        if !used_names.insert(filename.clone()) {
            let stem = filename
                .trim_end_matches(&format!(".{}", a.format))
                .to_string();
            let mut suffix = 0usize;
            loop {
                filename = if suffix == 0 {
                    format!("{stem}-{i}.{}", a.format)
                } else {
                    format!("{stem}-{i}-{suffix}.{}", a.format)
                };
                if used_names.insert(filename.clone()) {
                    break;
                }
                suffix += 1;
            }
        }

        dedup_filename_by_key.insert(dedup_key, filename.clone());
        asset_map.insert(a.id.clone(), filename);
    }

    for node in &ir.components {
        let name = sanitize_component_name(&node.name);
        let tsx = component::generate_component(
            node,
            theme_colors.as_ref(),
            &opts.cn_import,
            &opts.asset_public_base,
            opts.responsive,
            &opts.icon_library,
            &asset_map,
            warnings,
        );

        let dir_name = match opts.naming {
            NamingStyle::Pascal => name.clone(),
            NamingStyle::Kebab => to_kebab_case(&name),
        };
        let file_name = match opts.naming {
            NamingStyle::Pascal => format!("{name}.tsx"),
            NamingStyle::Kebab => format!("{}.tsx", to_kebab_case(&name)),
        };

        if opts.flat {
            files.push(OutputFile {
                path: file_name,
                content: tsx,
                binary: None,
            });
        } else {
            files.push(OutputFile {
                path: format!("{dir_name}/{file_name}"),
                content: tsx,
                binary: None,
            });
            if !opts.no_index {
                let file_stem = match opts.naming {
                    NamingStyle::Pascal => name.clone(),
                    NamingStyle::Kebab => to_kebab_case(&name),
                };
                files.push(OutputFile {
                    path: format!("{dir_name}/index.ts"),
                    content: index::generate_index(&[(&name, &file_stem)]),
                    binary: None,
                });
            }
        }
    }

    if !opts.no_theme
        && let Some(ref t) = ir.theme
    {
        files.push(OutputFile {
            path: "theme/tailwind.extend.js".into(),
            content: theme::generate_tailwind_extend(t),
            binary: None,
        });
        files.push(OutputFile {
            path: "theme/tokens.ts".into(),
            content: theme::generate_tokens_ts(t),
            binary: None,
        });
    }

    let mut written_asset_paths = HashSet::<String>::new();
    for a in &ir.assets {
        // Use the unique filename from asset_map
        let filename = asset_map
            .get(&a.id)
            .cloned()
            .unwrap_or_else(|| asset::asset_filename(&a.name, &a.format));
        let path = format!("assets/{filename}");
        if !written_asset_paths.insert(path.clone()) {
            continue;
        }

        match a.asset_type {
            AssetType::Svg => {
                let optimized_svg = asset::optimize_svg(&a.data);
                files.push(OutputFile {
                    path,
                    content: optimized_svg.clone(),
                    binary: None,
                });
                if opts.svg_mode == SvgMode::ReactComponent {
                    if !asset::svg_has_renderable_content(&optimized_svg)
                        || asset::is_divider_svg(&optimized_svg)
                    {
                        continue;
                    }
                    let comp_name = sanitize_component_name(&a.name);
                    let icon_name = format!("Icon{comp_name}");
                    files.push(OutputFile {
                        path: format!("icons/{icon_name}.tsx"),
                        content: asset::svg_to_react_component(&Asset {
                            data: optimized_svg,
                            ..a.clone()
                        }),
                        binary: None,
                    });
                }
            }
            AssetType::Image => {
                let binary = asset::decode_image_asset(a).ok();
                files.push(OutputFile {
                    path,
                    content: String::new(),
                    binary,
                });
            }
        }
    }

    files
}

fn asset_dedup_key(asset: &Asset) -> String {
    if asset.asset_type == AssetType::Image
        && let Some(source_ref) = &asset.source_ref
    {
        return format!("source:{source_ref}");
    }
    if asset.asset_type == AssetType::Svg {
        return format!("svg:{}", asset.data);
    }
    format!("id:{}", asset.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::{Layout, LayoutMode, Node, NodeType, TextProps, Theme};
    use crate::warning::WarningCollector;

    fn frame_node(name: &str, children: Vec<Node>) -> Node {
        Node {
            id: format!("id-{name}"),
            name: name.into(),
            node_type: NodeType::Frame,
            layout: Some(Layout {
                mode: Some(LayoutMode::Vertical),
                width: None,
                height: None,
                padding: None,
                gap: None,
                main_axis_align: None,
                cross_axis_align: None,
                constraints: None,
                position: None,
                overflow: None,
                rotation: None,
                parent_flex_dir: None,
                wrap: None,
                wrap_gap: None,
                wrap_align: None,
                min_width: None,
                max_width: None,
                min_height: None,
                max_height: None,
                self_align: None,
                overflow_x: None,
                overflow_y: None,
                z_index: None,
                aspect_ratio: None,
                grid_columns_sizing: None,
                grid_rows_sizing: None,
                grid_column_gap: None,
                grid_row_gap: None,
                grid_column_span: None,
                grid_row_span: None,
                grid_column_start: None,
                grid_row_start: None,
                flip_x: None,
                flip_y: None,
            }),
            style: None,
            text: None,
            vector: None,
            vector_paths: None,
            boolean_op: None,
            mask: None,
            component: None,
            children,
            overlay: false,
        }
    }

    fn text_node(content: &str) -> Node {
        Node {
            id: "tid".into(),
            name: "Text".into(),
            node_type: NodeType::Text,
            layout: None,
            style: None,
            text: Some(TextProps {
                content: content.into(),
                font_size: None,
                font_family: None,
                font_weight: None,
                line_height: None,
                letter_spacing: None,
                text_align: None,
                text_decoration: None,
                text_transform: None,
                truncation: None,
                italic: None,
                vertical_align: None,
                paragraph_spacing: None,
                max_lines: None,
                hyperlink: None,
                list_type: None,
                opentype_flags: None,
                spans: None,
            }),
            vector: None,
            vector_paths: None,
            boolean_op: None,
            mask: None,
            component: None,
            children: vec![],
            overlay: false,
        }
    }

    #[test]
    fn test_single_component_output() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![frame_node("Card", vec![text_node("Hello")])],
        };
        let mut warnings = WarningCollector::new();
        let opts = ConvertOptions {
            cn_import: "../utils/cn".into(),
            asset_public_base: "/assets".into(),
            icon_library: IconLibrary::None,
            responsive: false,
            flat: false,
            no_index: false,
            no_theme: false,
            naming: NamingStyle::Pascal,
            svg_mode: SvgMode::ReactComponent,
        };
        let files = generate_file_tree(&ir, &opts, &mut warnings);
        assert!(files.iter().any(|f| f.path.ends_with("Card/Card.tsx")));
        assert!(files.iter().any(|f| f.path.ends_with("Card/index.ts")));
    }

    #[test]
    fn test_flat_output() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![frame_node("Button", vec![])],
        };
        let mut warnings = WarningCollector::new();
        let opts = ConvertOptions {
            cn_import: "../utils/cn".into(),
            asset_public_base: "/assets".into(),
            icon_library: IconLibrary::None,
            responsive: false,
            flat: true,
            no_index: false,
            no_theme: false,
            naming: NamingStyle::Pascal,
            svg_mode: SvgMode::ReactComponent,
        };
        let files = generate_file_tree(&ir, &opts, &mut warnings);
        assert!(files.iter().any(|f| f.path == "Button.tsx"));
    }

    #[test]
    fn test_no_index_flag() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![],
            components: vec![frame_node("Card", vec![])],
        };
        let mut warnings = WarningCollector::new();
        let opts = ConvertOptions {
            cn_import: "../utils/cn".into(),
            asset_public_base: "/assets".into(),
            icon_library: IconLibrary::None,
            responsive: false,
            flat: false,
            no_index: true,
            no_theme: false,
            naming: NamingStyle::Pascal,
            svg_mode: SvgMode::ReactComponent,
        };
        let files = generate_file_tree(&ir, &opts, &mut warnings);
        assert!(!files.iter().any(|f| f.path.ends_with("index.ts")));
    }

    #[test]
    fn test_theme_output() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: Some(Theme {
                colors: Some(std::collections::HashMap::from([(
                    "primary".into(),
                    "#000".into(),
                )])),
                spacing: None,
                border_radius: None,
                font_size: None,
                font_family: None,
                shadows: None,
                opacity: None,
            }),
            assets: vec![],
            components: vec![],
        };
        let mut warnings = WarningCollector::new();
        let opts = ConvertOptions {
            cn_import: "../utils/cn".into(),
            asset_public_base: "/assets".into(),
            icon_library: IconLibrary::None,
            responsive: false,
            flat: false,
            no_index: false,
            no_theme: false,
            naming: NamingStyle::Pascal,
            svg_mode: SvgMode::ReactComponent,
        };
        let files = generate_file_tree(&ir, &opts, &mut warnings);
        assert!(files.iter().any(|f| f.path == "theme/tailwind.extend.js"));
        assert!(files.iter().any(|f| f.path == "theme/tokens.ts"));
    }

    #[test]
    fn test_no_theme_flag() {
        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: Some(Theme {
                colors: Some(std::collections::HashMap::from([(
                    "x".into(),
                    "#000".into(),
                )])),
                spacing: None,
                border_radius: None,
                font_size: None,
                font_family: None,
                shadows: None,
                opacity: None,
            }),
            assets: vec![],
            components: vec![],
        };
        let mut warnings = WarningCollector::new();
        let opts = ConvertOptions {
            cn_import: "../utils/cn".into(),
            asset_public_base: "/assets".into(),
            icon_library: IconLibrary::None,
            responsive: false,
            flat: false,
            no_index: false,
            no_theme: true,
            naming: NamingStyle::Pascal,
            svg_mode: SvgMode::ReactComponent,
        };
        let files = generate_file_tree(&ir, &opts, &mut warnings);
        assert!(!files.iter().any(|f| f.path.starts_with("theme/")));
    }
}
