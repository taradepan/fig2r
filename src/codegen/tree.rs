use crate::codegen::{asset, component, index, theme};
use crate::emit::formatter::{sanitize_component_name, to_kebab_case};
use crate::ir::schema::{Asset, AssetType, DesignIR};
use crate::warning::WarningCollector;
use rayon::prelude::*;
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
    let mut asset_map: component::AssetMap = component::AssetMap::default();
    let mut icon_map: component::IconMap = component::IconMap::default();
    let mut dedup_filename_by_key = std::collections::HashMap::<String, String>::new();
    let mut dedup_iconname_by_key = std::collections::HashMap::<String, String>::new();
    let mut used_names = HashSet::<String>::new();
    let mut used_icon_names = HashSet::<String>::new();
    for (i, a) in ir.assets.iter().enumerate() {
        let dedup_key = asset_dedup_key(a);
        if let Some(existing) = dedup_filename_by_key.get(&dedup_key) {
            asset_map.insert(a.id.clone(), existing.clone());
            if let Some(existing_icon) = dedup_iconname_by_key.get(&dedup_key) {
                icon_map.insert(a.id.clone(), existing_icon.clone());
            }
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

        dedup_filename_by_key.insert(dedup_key.clone(), filename.clone());
        asset_map.insert(a.id.clone(), filename);

        // Record icon mapping iff this SVG will be emitted as a React component.
        // Mirrors the filters in the asset emission loop below.
        // Multi-color illustrations (charts, avatars, decorative art) are NOT
        // suitable — stripping their fills would collapse them to a monochrome
        // silhouette. Those fall back to `<img src="/assets/*.svg">`.
        if opts.svg_mode == SvgMode::ReactComponent && a.asset_type == AssetType::Svg {
            let optimized = asset::optimize_svg(&a.data);
            if asset::svg_has_renderable_content(&optimized)
                && !asset::is_divider_svg(&optimized)
                && asset::is_monochrome_svg(&optimized)
            {
                // Generate a unique icon component name. Figma files often reuse
                // generic node names ("Frame", "Vector", "SVG") across visually
                // different icons — without uniquification, they'd all collapse
                // to a single `IconFrame.tsx` and every reference would render
                // the same (first-encountered) shape.
                let base = format!("Icon{}", sanitize_component_name(&a.name));
                let mut icon_name = base.clone();
                let mut suffix = 2;
                while !used_icon_names.insert(icon_name.clone()) {
                    icon_name = format!("{base}{suffix}");
                    suffix += 1;
                }
                dedup_iconname_by_key.insert(dedup_key, icon_name.clone());
                icon_map.insert(a.id.clone(), icon_name);
            }
        }
    }

    // Parallel codegen: each component is independent, so fan them across rayon's
    // thread pool. Per-worker WarningCollectors are merged back on the main thread.
    let per_component: Vec<(Vec<OutputFile>, WarningCollector)> = ir
        .components
        .par_iter()
        .map(|node| {
            let mut local_warnings = WarningCollector::new();
            let name = sanitize_component_name(&node.name);
            let tsx = component::generate_component(
                node,
                theme_colors.as_ref(),
                &opts.cn_import,
                &opts.asset_public_base,
                opts.responsive,
                &opts.icon_library,
                &asset_map,
                &icon_map,
                &mut local_warnings,
            );

            let dir_name = match opts.naming {
                NamingStyle::Pascal => name.clone(),
                NamingStyle::Kebab => to_kebab_case(&name),
            };
            let file_name = match opts.naming {
                NamingStyle::Pascal => format!("{name}.tsx"),
                NamingStyle::Kebab => format!("{}.tsx", to_kebab_case(&name)),
            };

            let mut local_files = Vec::with_capacity(2);
            if opts.flat {
                local_files.push(OutputFile {
                    path: file_name,
                    content: tsx,
                    binary: None,
                });
            } else {
                local_files.push(OutputFile {
                    path: format!("{dir_name}/{file_name}"),
                    content: tsx,
                    binary: None,
                });
                if !opts.no_index {
                    let file_stem = match opts.naming {
                        NamingStyle::Pascal => name.clone(),
                        NamingStyle::Kebab => to_kebab_case(&name),
                    };
                    local_files.push(OutputFile {
                        path: format!("{dir_name}/index.ts"),
                        content: index::generate_index(&[(&name, &file_stem)]),
                        binary: None,
                    });
                }
            }
            (local_files, local_warnings)
        })
        .collect();

    for (local_files, local_warnings) in per_component {
        files.extend(local_files);
        warnings.merge(local_warnings);
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
    let mut emitted_icons: Vec<String> = Vec::new();
    let mut emitted_icon_set: HashSet<String> = HashSet::new();
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
                        || !asset::is_monochrome_svg(&optimized_svg)
                    {
                        continue;
                    }
                    // Use the uniquified icon name from `icon_map` (populated in
                    // the earlier dedup pass). Recomputing from `a.name` would
                    // collide when multiple Figma nodes share a generic name
                    // ("Frame", "SVG", "Vector") but have different content —
                    // only the first would emit and every other reference would
                    // render the first icon.
                    let Some(icon_name) = icon_map.get(&a.id).cloned() else {
                        continue;
                    };
                    if !emitted_icon_set.insert(icon_name.clone()) {
                        continue;
                    }
                    emitted_icons.push(icon_name.clone());
                    files.push(OutputFile {
                        path: format!("icons/{icon_name}.tsx"),
                        content: asset::svg_to_react_component_named(
                            &Asset {
                                data: optimized_svg,
                                ..a.clone()
                            },
                            Some(&icon_name),
                        ),
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

    // Emit icons/index.ts barrel so component imports like
    // `import { IconFoo } from '../icons'` resolve.
    if !emitted_icons.is_empty() {
        emitted_icons.sort();
        let pairs: Vec<(&str, &str)> = emitted_icons
            .iter()
            .map(|n| (n.as_str(), n.as_str()))
            .collect();
        files.push(OutputFile {
            path: "icons/index.ts".into(),
            content: index::generate_index(&pairs),
            binary: None,
        });
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
                text_decoration_style: None,
                text_decoration_offset: None,
                text_decoration_thickness: None,
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

    #[test]
    fn test_svg_icon_is_imported_and_used_instead_of_img() {
        use crate::ir::schema::{Asset, AssetType, Fill, Style};

        // Build an Image node that references an SVG asset via an Image fill.
        let image_node = Node {
            id: "node-img-1".into(),
            name: "Check Icon".into(),
            node_type: NodeType::Image,
            layout: None,
            style: Some(Style {
                fills: Some(vec![Fill::Image {
                    asset_ref: "asset-svg-1".into(),
                    scale_mode: None,
                }]),
                stroke: None,
                border_radius: None,
                effects: None,
                opacity: None,
                blend_mode: None,
            }),
            text: None,
            vector: None,
            vector_paths: None,
            boolean_op: None,
            mask: None,
            component: None,
            children: vec![],
            overlay: false,
        };

        let svg_data = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5 13l4 4L19 7"/></svg>"#;
        let asset = Asset {
            id: "asset-svg-1".into(),
            name: "check".into(),
            asset_type: AssetType::Svg,
            format: "svg".into(),
            data: svg_data.into(),
            url: None,
            source_ref: None,
        };

        let ir = DesignIR {
            version: "1.0".into(),
            name: "Test".into(),
            theme: None,
            assets: vec![asset],
            components: vec![frame_node("Page", vec![image_node])],
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

        // Raw SVG is still emitted for downstream consumers.
        assert!(
            files.iter().any(|f| f.path == "assets/check.svg"),
            "assets/check.svg should still be emitted"
        );
        // Icon component file is emitted.
        assert!(
            files.iter().any(|f| f.path == "icons/IconCheck.tsx"),
            "icons/IconCheck.tsx should be emitted"
        );
        // Icons barrel is emitted.
        assert!(
            files.iter().any(|f| f.path == "icons/index.ts"),
            "icons/index.ts barrel should be emitted"
        );
        // Component TSX imports and uses the icon, not <img>.
        let page_tsx = files
            .iter()
            .find(|f| f.path.ends_with("Page/Page.tsx"))
            .expect("Page component tsx should exist");
        assert!(
            page_tsx
                .content
                .contains("import { IconCheck } from '../icons';"),
            "expected icon import in Page.tsx, got:\n{}",
            page_tsx.content
        );
        assert!(
            page_tsx.content.contains("<IconCheck "),
            "expected <IconCheck /> usage in Page.tsx, got:\n{}",
            page_tsx.content
        );
        assert!(
            !page_tsx.content.contains("<img src=\"/assets/check.svg\""),
            "Page.tsx should not <img src> the SVG, got:\n{}",
            page_tsx.content
        );
    }
}
