use crate::codegen::tree::IconLibrary;
use crate::codegen::variant::{self, VariantProp};
use crate::emit::formatter::{escape_jsx_text, indent, join_classes, sanitize_component_name};
use crate::ir::schema::{
    DimensionType, Fill, ListType, Node, NodeType, ScaleMode, Style, VerticalAlign,
};
use crate::tailwind::{layout, style, text, text as text_tw};
use crate::warning::WarningCollector;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

/// Maps asset ID → relative file path (e.g., "hero.png")
pub type AssetMap = HashMap<String, String>;

#[allow(clippy::too_many_arguments)]
pub fn generate_component(
    node: &Node,
    theme_colors: Option<&HashMap<String, String>>,
    cn_import: &str,
    asset_public_base: &str,
    responsive: bool,
    icon_library: &IconLibrary,
    asset_map: &AssetMap,
    warnings: &mut WarningCollector,
) -> String {
    let name = sanitize_component_name(&node.name);
    let has_children = !node.children.is_empty();
    let mut output = String::new();

    let variant_props = extract_variant_props(node);
    let uses_cn = !variant_props.is_empty();

    let font_block = generate_font_block(node);
    if !font_block.is_empty() {
        output.push_str(&font_block);
        output.push('\n');
    }

    // Import cn only when needed (variant components use conditional classes)
    if uses_cn {
        let _ = writeln!(output, "import {{ cn }} from '{cn_import}';\n");
    }

    if !variant_props.is_empty() {
        output.push_str(&variant::generate_prop_interface(
            &name,
            &variant_props,
            has_children,
        ));
        output.push_str("\n\n");
        let destructure = variant::generate_destructure(&variant_props, has_children);
        let _ = writeln!(
            output,
            "export function {name}({destructure}: {name}Props) {{"
        );
    } else if has_children {
        let _ = writeln!(
            output,
            "interface {name}Props {{\n  children?: React.ReactNode;\n}}\n"
        );
        let _ = writeln!(
            output,
            "export function {name}({{ children }}: {name}Props) {{"
        );
    } else {
        let _ = writeln!(output, "export function {name}() {{");
    }

    // Collect font variable idents so we can expose the CSS variables to
    // descendants via a display:contents wrapper (avoids an extra layout box).
    let mut font_data: HashMap<String, BTreeSet<u32>> = HashMap::new();
    collect_font_data(node, &mut font_data);
    let font_vars: Vec<String> = font_data
        .keys()
        .filter(|f| is_google_font(f))
        .map(|f| google_font_var_ident(f))
        .collect();

    let _ = writeln!(output, "{}return (", indent(1));
    let (wrap_open, wrap_close, inner_depth) = if font_vars.is_empty() {
        (String::new(), String::new(), 2)
    } else {
        let expr = font_vars
            .iter()
            .map(|v| format!("${{{v}.variable}}"))
            .collect::<Vec<_>>()
            .join(" ");
        (
            format!("{}<div className={{`{expr} contents`}}>\n", indent(2)),
            format!("{}</div>\n", indent(2)),
            3,
        )
    };
    output.push_str(&wrap_open);
    render_node(
        &mut output,
        node,
        inner_depth,
        None,
        None,
        true,
        theme_colors,
        asset_public_base,
        responsive,
        icon_library,
        asset_map,
        warnings,
    );
    output.push_str(&wrap_close);
    let _ = writeln!(output, "{});", indent(1));
    output.push_str("}\n");

    output
}

fn generate_font_block(root: &Node) -> String {
    let mut font_data: HashMap<String, BTreeSet<u32>> = HashMap::new();
    collect_font_data(root, &mut font_data);
    if font_data.is_empty() {
        return String::new();
    }

    let mut google: Vec<(&String, &BTreeSet<u32>)> = font_data
        .iter()
        .filter(|(f, _)| is_google_font(f))
        .collect();
    google.sort_by_key(|(f, _)| f.to_string());
    let custom: Vec<&String> = font_data.keys().filter(|f| !is_google_font(f)).collect();

    let mut block = String::new();
    if !google.is_empty() {
        let imports = google
            .iter()
            .map(|(f, _)| google_font_import_ident(f))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(block, "import {{ {imports} }} from 'next/font/google';");
        block.push('\n');
        for (family, weights) in &google {
            let import_ident = google_font_import_ident(family);
            let var_ident = google_font_var_ident(family);
            let css_var = format!("--font-{}", var_ident.replace('_', "-"));
            if weights.len() <= 1 && weights.first().is_none_or(|w| *w == 400) {
                let _ = writeln!(
                    block,
                    "const {var_ident} = {import_ident}({{ subsets: ['latin'], variable: '{css_var}' }});"
                );
            } else {
                let weight_strs: Vec<String> = weights.iter().map(|w| format!("'{w}'")).collect();
                let _ = writeln!(
                    block,
                    "const {var_ident} = {import_ident}({{ weight: [{}], subsets: ['latin'], variable: '{css_var}' }});",
                    weight_strs.join(", ")
                );
            }
        }
    }

    if !custom.is_empty() {
        let names: Vec<&str> = custom.iter().map(|s| s.as_str()).collect();
        let _ = writeln!(
            block,
            "// fig2r: custom font(s) detected: {}. Add @font-face and wire in app/layout.tsx.",
            names.join(", ")
        );
    }

    block
}

fn collect_font_data(node: &Node, out: &mut HashMap<String, BTreeSet<u32>>) {
    if let Some(text) = &node.text
        && let Some(family) = &text.font_family
    {
        let family = family.trim();
        if !family.is_empty() {
            let weights = out.entry(family.to_string()).or_default();
            if let Some(w) = text.font_weight {
                weights.insert(w);
            } else {
                weights.insert(400);
            }
        }
    }
    for child in &node.children {
        collect_font_data(child, out);
    }
}

fn is_google_font(family: &str) -> bool {
    // Keep this curated so custom families are not incorrectly imported.
    const GOOGLE_FONTS: &[&str] = &[
        "Inter",
        "JetBrains Mono",
        "Roboto",
        "Poppins",
        "Manrope",
        "DM Sans",
        "Open Sans",
        "Lato",
        "Montserrat",
        "Nunito",
        "Playfair Display",
        "Source Sans 3",
        "Work Sans",
        "Fira Code",
        "Merriweather",
        "Plus Jakarta Sans",
    ];
    GOOGLE_FONTS.contains(&family)
}

fn google_font_import_ident(family: &str) -> String {
    family.replace(' ', "_")
}

fn google_font_var_ident(family: &str) -> String {
    let mut out = String::new();
    for ch in family.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn extract_variant_props(node: &Node) -> Vec<VariantProp> {
    if let Some(ref comp) = node.component
        && comp.is_component
        && let Some(ref variants) = comp.variants
    {
        let defaults = comp.variant_values.clone().unwrap_or_default();
        return variant::extract_variant_props(variants, &defaults);
    }
    vec![]
}

#[allow(clippy::too_many_arguments)]
fn render_node(
    out: &mut String,
    node: &Node,
    depth: usize,
    parent_name: Option<&str>,
    parent_tag: Option<&str>,
    is_root: bool,
    theme_colors: Option<&HashMap<String, String>>,
    asset_public_base: &str,
    responsive: bool,
    icon_library: &IconLibrary,
    asset_map: &AssetMap,
    warnings: &mut WarningCollector,
) {
    match node.node_type {
        NodeType::Text => {
            render_text_node(out, node, depth, theme_colors);
        }
        NodeType::Image => {
            render_image_node(
                out,
                node,
                depth,
                theme_colors,
                asset_public_base,
                parent_name,
                asset_map,
                warnings,
            );
        }
        NodeType::Vector | NodeType::BooleanOp => {
            // Skip vector/boolean nodes — they're icons without path data
            // Icons should come from the project's icon library
        }
        _ => {
            render_frame_node(
                out,
                node,
                depth,
                parent_name,
                parent_tag,
                is_root,
                theme_colors,
                asset_public_base,
                responsive,
                icon_library,
                asset_map,
                warnings,
            );
        }
    }
}

fn render_text_node(
    out: &mut String,
    node: &Node,
    depth: usize,
    theme_colors: Option<&HashMap<String, String>>,
) {
    let ind = indent(depth);
    let mut classes = Vec::new();

    // Detect bullet marker nodes: empty/zero-width content + list_type set.
    // Figma stores these as separate tiny text nodes (21px wide, variable height)
    // which would otherwise render as awkward fixed-size boxes.
    let is_bullet_marker = node.text.as_ref().is_some_and(|t| {
        t.list_type.is_some() && (t.content.trim().is_empty() || t.content == "\u{200B}")
    });

    if let Some(ref text_props) = node.text {
        classes.extend(text::text_classes(text_props));
    }

    // Skip layout size classes for bullet markers — they only have content-driven size
    if !is_bullet_marker && let Some(ref l) = node.layout {
        let mut size = layout::size_classes(l);
        // Text leaves: drop vertical-fill classes that would crush text inside
        // fixed-height parents (e.g. `h-full` + `truncate` clips text vertically).
        // Let parent `items-center` handle vertical alignment instead.
        size.retain(|c| !matches!(c.as_str(), "h-full" | "self-stretch" | "min-h-0"));
        classes.extend(size);
    }

    // Vertical alignment: only meaningful when text has fixed height
    let has_fixed_height = node
        .layout
        .as_ref()
        .and_then(|l| l.height.as_ref())
        .is_some_and(|d| d.dim_type == DimensionType::Fixed);
    if has_fixed_height
        && !is_bullet_marker
        && let Some(ref tp) = node.text
        && let Some(ref va) = tp.vertical_align
    {
        match va {
            VerticalAlign::Top => {}
            VerticalAlign::Center => {
                classes.push("flex".into());
                classes.push("flex-col".into());
                classes.push("justify-center".into());
            }
            VerticalAlign::Bottom => {
                classes.push("flex".into());
                classes.push("flex-col".into());
                classes.push("justify-end".into());
            }
        }
    }

    // Text color from fills (use last/topmost solid fill only — Figma stacks fills)
    if let Some(ref s) = node.style {
        if let Some(ref fills) = s.fills
            && let Some(Fill::Solid { color, .. }) =
                fills.iter().rev().find(|f| matches!(f, Fill::Solid { .. }))
        {
            let color_class = if let Some(colors) = theme_colors {
                colors
                    .iter()
                    .find(|(_, v)| v.eq_ignore_ascii_case(color))
                    .map(|(name, _)| format!("text-{name}"))
            } else {
                None
            };
            classes.push(color_class.unwrap_or_else(|| format!("text-[{color}]")));
        }
        if let Some(opacity) = s.opacity
            && (opacity - 1.0).abs() > 0.01
        {
            classes.push(style::opacity_class(opacity));
        }
    }

    let raw_content = node.text.as_ref().map(|t| t.content.as_str()).unwrap_or("");
    // Figma uses separate empty text nodes with lineTypes=UNORDERED as bullet markers.
    // Render the appropriate marker character when content is empty.
    let list_type = node.text.as_ref().and_then(|t| t.list_type.as_ref());
    let raw_content = if raw_content.trim().is_empty() || raw_content == "\u{200B}" {
        match list_type {
            Some(ListType::Unordered) => "•",
            Some(ListType::Ordered) => "1.",
            None => raw_content,
        }
    } else {
        raw_content
    };
    let content = escape_jsx_text(raw_content);

    // whitespace-nowrap only when the text box is HUG width (auto-sized, single-line).
    // FILL/FIXED width means Figma auto-wraps the text within the container.
    let is_hug_width = node
        .layout
        .as_ref()
        .and_then(|l| l.width.as_ref())
        .is_none_or(|d| d.dim_type == DimensionType::Hug);
    if is_hug_width {
        classes.push("whitespace-nowrap".into());
    }

    let href = node.text.as_ref().and_then(|t| t.hyperlink.as_ref());
    let spans = node.text.as_ref().and_then(|t| t.spans.as_ref());

    // Rich text: render each span as a child <span>
    if let Some(spans) = spans {
        let outer_class = join_classes(&classes);
        let opening = if outer_class.is_empty() {
            "<span>".to_string()
        } else {
            format!("<span className=\"{outer_class}\">")
        };
        let _ = writeln!(out, "{ind}{opening}");
        for span in spans {
            let span_classes = span_classes(span);
            let span_content = escape_jsx_text(&span.content);
            let inner = if let Some(href) = &span.hyperlink {
                if span_classes.is_empty() {
                    format!("<a href=\"{href}\">{span_content}</a>")
                } else {
                    format!("<a href=\"{href}\" className=\"{span_classes}\">{span_content}</a>")
                }
            } else if span_classes.is_empty() {
                format!("<span>{span_content}</span>")
            } else {
                format!("<span className=\"{span_classes}\">{span_content}</span>")
            };
            let _ = writeln!(out, "{}{}", indent(depth + 1), inner);
        }
        let _ = writeln!(out, "{ind}</span>");
        return;
    }

    if let Some(href) = href {
        if classes.is_empty() {
            let _ = writeln!(out, "{ind}<a href=\"{href}\"><span>{content}</span></a>");
        } else {
            let _ = writeln!(
                out,
                "{ind}<a href=\"{href}\"><span className=\"{}\">{content}</span></a>",
                join_classes(&classes)
            );
        }
    } else if classes.is_empty() {
        let _ = writeln!(out, "{ind}<span>{content}</span>");
    } else {
        let _ = writeln!(
            out,
            "{ind}<span className=\"{}\">{content}</span>",
            join_classes(&classes)
        );
    }
}

fn render_inline_svg(
    out: &mut String,
    node: &Node,
    ind: &str,
    classes: &[String],
    paths: &[crate::ir::schema::VectorPath],
) {
    let (w, h) = node
        .layout
        .as_ref()
        .and_then(|l| {
            let w = l.width.as_ref()?.value?;
            let h = l.height.as_ref()?.value?;
            Some((w, h))
        })
        .unwrap_or((24.0, 24.0));
    let class_str = join_classes(classes);
    let class_attr = if class_str.is_empty() {
        String::new()
    } else {
        format!(" className=\"{class_str}\"")
    };
    let _ = writeln!(
        out,
        "{ind}<svg viewBox=\"0 0 {w} {h}\" xmlns=\"http://www.w3.org/2000/svg\"{class_attr}>"
    );
    for p in paths {
        let fill = p.fill.as_deref().unwrap_or("currentColor");
        let rule = match &p.fill_rule {
            Some(crate::ir::schema::FillRule::Evenodd) => " fillRule=\"evenodd\"",
            _ => "",
        };
        let _ = writeln!(out, "{ind}  <path d=\"{}\" fill=\"{fill}\"{rule} />", p.d);
    }
    let _ = writeln!(out, "{ind}</svg>");
}

fn span_classes(span: &crate::ir::schema::TextSpan) -> String {
    let mut classes: Vec<String> = Vec::new();
    if let Some(w) = span.font_weight {
        classes.push(match w {
            100 => "font-thin".into(),
            200 => "font-extralight".into(),
            300 => "font-light".into(),
            400 => "font-normal".into(),
            500 => "font-medium".into(),
            600 => "font-semibold".into(),
            700 => "font-bold".into(),
            800 => "font-extrabold".into(),
            900 => "font-black".into(),
            _ => format!("font-[{w}]"),
        });
    }
    if span.italic == Some(true) {
        classes.push("italic".into());
    }
    if let Some(ref d) = span.text_decoration {
        match d {
            crate::ir::schema::TextDecoration::Underline => classes.push("underline".into()),
            crate::ir::schema::TextDecoration::Strikethrough => {
                classes.push("line-through".into());
            }
            crate::ir::schema::TextDecoration::None => {}
        }
    }
    if let Some(ref fam) = span.font_family
        && fam != "Inter"
    {
        let css_var = text_tw::google_font_css_var(fam);
        classes.push(format!("font-[var({css_var})]"));
    }
    if let Some(s) = span.font_size {
        classes.push(format!("text-[{s}px]"));
    }
    if let Some(ref c) = span.color {
        classes.push(format!("text-[{c}]"));
    }
    classes.join(" ")
}

#[allow(clippy::too_many_arguments)]
fn render_image_node(
    out: &mut String,
    node: &Node,
    depth: usize,
    theme_colors: Option<&HashMap<String, String>>,
    asset_public_base: &str,
    parent_name: Option<&str>,
    asset_map: &AssetMap,
    warnings: &mut WarningCollector,
) {
    let ind = indent(depth);
    let mut classes = Vec::new();

    if should_skip_zero_sized_image(node) {
        return;
    }

    if let Some(ref l) = node.layout {
        classes.extend(layout::size_classes(l));
    }
    // For image nodes, don't add fill classes — the SVG/PNG already contains the colors.
    // Only add border-radius, stroke, effects, opacity, blend_mode.
    collect_style_classes_no_fills(&mut classes, node, theme_colors, warnings);

    // Inline SVG from extracted vector paths — no remote asset download needed.
    if let Some(ref paths) = node.vector_paths
        && !paths.is_empty()
    {
        render_inline_svg(out, node, &ind, &classes, paths);
        return;
    }

    let (src, scale_mode) = node
        .style
        .as_ref()
        .and_then(|s| s.fills.as_ref())
        .and_then(|fills| {
            fills.iter().find_map(|f| {
                if let Fill::Image {
                    asset_ref,
                    scale_mode,
                } = f
                {
                    asset_map.get(asset_ref).map(|f| (f, scale_mode.clone()))
                } else {
                    None
                }
            })
        })
        .map(|(filename, sm)| {
            let base = asset_public_base.trim_end_matches('/');
            (format!("{base}/{filename}"), sm)
        })
        .or_else(|| {
            asset_map.get(&node.id).map(|filename| {
                let base = asset_public_base.trim_end_matches('/');
                (format!("{base}/{filename}"), None)
            })
        })
        .unwrap_or_default();
    // Emit object-fit based on Figma's scaleMode
    if let Some(ref sm) = scale_mode {
        let cls = match sm {
            ScaleMode::Fill | ScaleMode::Crop => "object-cover",
            ScaleMode::Fit => "object-contain",
            ScaleMode::Tile => "object-cover", // CSS can't tile <img>; closest approximation
        };
        classes.push(cls.into());
    }
    let alt = &node.name;

    if src.is_empty() {
        let _ = writeln!(
            out,
            "{ind}{{/* fig2r: image failed to download, use Figma MCP to get node {}{} */}}",
            node.id,
            parent_name
                .map(|p| format!(" (parent: {p})"))
                .unwrap_or_default()
        );
        return;
    }

    if classes.is_empty() {
        let _ = writeln!(out, "{ind}<img src=\"{src}\" alt=\"{alt}\" />");
    } else {
        let _ = writeln!(
            out,
            "{ind}<img src=\"{src}\" alt=\"{alt}\" className=\"{}\" />",
            join_classes(&classes)
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_frame_node(
    out: &mut String,
    node: &Node,
    depth: usize,
    parent_name: Option<&str>,
    parent_tag: Option<&str>,
    is_root: bool,
    theme_colors: Option<&HashMap<String, String>>,
    asset_public_base: &str,
    responsive: bool,
    icon_library: &IconLibrary,
    asset_map: &AssetMap,
    warnings: &mut WarningCollector,
) {
    let ind = indent(depth);
    let mut classes = Vec::new();

    if let Some(ref l) = node.layout {
        classes.extend(layout::layout_classes(l));
    }
    collect_style_classes(&mut classes, node, theme_colors, warnings);
    if is_root && responsive {
        apply_responsive_root_sizing(&mut classes);
    }

    // All frames with children get relative positioning (needed for absolute children),
    // but skip if this node itself is absolutely positioned (already has "absolute").
    let is_absolute = node
        .layout
        .as_ref()
        .and_then(|l| l.position.as_ref())
        .is_some();
    if !node.children.is_empty() && !is_absolute {
        classes.push("relative".into());
    }

    let tag = semantic_tag_for(node, parent_tag);
    if tag == "button" {
        classes.push("cursor-pointer".into());
    }
    let class_str = join_classes(&classes);

    if only_skipped_vector_children(node) {
        let _ = writeln!(
            out,
            "{ind}{{/* {} */}}",
            icon_placeholder_comment(node, icon_library)
        );
        return;
    }

    if node.children.is_empty() {
        if class_str.is_empty() {
            return;
        }
        if !is_root && should_emit_comment(&node.name, parent_name) {
            let _ = writeln!(out, "{ind}{{/* {} */}}", node.name);
        }
        if tag == "hr" {
            // Tailwind preflight adds a 1px currentColor top border to <hr>.
            // Reset it so the emitted fills/height alone define the divider.
            let final_class = if class_str.contains("border-") {
                class_str.clone()
            } else {
                format!("border-0 {class_str}")
            };
            let _ = writeln!(out, "{ind}<{tag} className=\"{final_class}\" />");
            return;
        }
        if class_str.is_empty() {
            let _ = writeln!(out, "{ind}<{tag} />");
        } else {
            let _ = writeln!(out, "{ind}<{tag} className=\"{class_str}\" />");
        }
    } else {
        if !is_root && should_emit_comment(&node.name, parent_name) {
            let _ = writeln!(out, "{ind}{{/* {} */}}", node.name);
        }
        if class_str.is_empty() {
            let _ = writeln!(out, "{ind}<{tag}>");
        } else {
            let _ = writeln!(out, "{ind}<{tag} className=\"{class_str}\">");
        }

        for child in &node.children {
            if child.overlay {
                // Render overlay using IR flag from transform.
                let _ = writeln!(
                    out,
                    "{}<div className=\"absolute inset-0 z-50 flex items-center justify-center\">",
                    indent(depth + 1)
                );
                let _ = writeln!(
                    out,
                    "{}<div className=\"absolute inset-0 bg-black/50\" />",
                    indent(depth + 2)
                );
                let _ = writeln!(out, "{}<div className=\"relative\">", indent(depth + 2));
                render_node(
                    out,
                    child,
                    depth + 3,
                    Some(&node.name),
                    Some(tag),
                    false,
                    theme_colors,
                    asset_public_base,
                    responsive,
                    icon_library,
                    asset_map,
                    warnings,
                );
                let _ = writeln!(out, "{}</div>", indent(depth + 2));
                let _ = writeln!(out, "{}</div>", indent(depth + 1));
            } else {
                render_node(
                    out,
                    child,
                    depth + 1,
                    Some(&node.name),
                    Some(tag),
                    false,
                    theme_colors,
                    asset_public_base,
                    responsive,
                    icon_library,
                    asset_map,
                    warnings,
                );
            }
        }
        let _ = writeln!(out, "{ind}</{tag}>");
    }
}

fn semantic_tag_for(node: &Node, parent_tag: Option<&str>) -> &'static str {
    let lower = node.name.to_ascii_lowercase();
    let candidate = if lower.contains("header") {
        "header"
    } else if lower == "nav" || lower.contains("navbar") || lower.contains("navigation") {
        "nav"
    } else if lower.contains("footer") {
        "footer"
    } else if lower == "main" || lower.contains("content") {
        "main"
    } else if lower.contains("section") {
        "section"
    } else if lower.contains("article") {
        "article"
    } else if lower.contains("aside") || lower.contains("sidebar") {
        "aside"
    } else if lower.contains("form") {
        "form"
    } else if lower.contains("button") {
        "button"
    } else if lower.contains("separator") || lower.contains("divider") {
        "hr"
    } else {
        "div"
    };
    // Prevent invalid HTML nesting (e.g., <button> inside <button>)
    if candidate == "button" && parent_tag == Some("button") {
        return "div";
    }
    candidate
}

fn should_emit_comment(name: &str, parent_name: Option<&str>) -> bool {
    let n = name.trim();
    if n.is_empty() {
        return false;
    }
    if let Some(parent) = parent_name
        && parent.trim() == n
    {
        return false;
    }
    if n.eq_ignore_ascii_case("container") || n.eq_ignore_ascii_case("group") {
        return false;
    }
    if let Some(digits) = n.strip_prefix("Frame ")
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
    {
        return false;
    }
    true
}

fn only_skipped_vector_children(node: &Node) -> bool {
    !node.children.is_empty()
        && node
            .children
            .iter()
            .all(|c| matches!(c.node_type, NodeType::Vector | NodeType::BooleanOp))
}

fn icon_placeholder_comment(node: &Node, icon_library: &IconLibrary) -> String {
    let icon_name = sanitize_component_name(&node.name);
    let dims = node
        .layout
        .as_ref()
        .and_then(|l| match (&l.width, &l.height) {
            (Some(w), Some(h))
                if w.dim_type == DimensionType::Fixed && h.dim_type == DimensionType::Fixed =>
            {
                Some(format!(
                    " ({}x{})",
                    w.value.unwrap_or_default(),
                    h.value.unwrap_or_default()
                ))
            }
            _ => None,
        })
        .unwrap_or_default();
    let lib_hint = icon_import_hint(&icon_name, icon_library);
    if lib_hint.is_empty() {
        format!("Icon: {}{} — use your icon library", node.name, dims)
    } else {
        format!("Icon: {}{} — {}", node.name, dims, lib_hint)
    }
}

fn icon_import_hint(name: &str, icon_library: &IconLibrary) -> String {
    match icon_library {
        IconLibrary::None => String::new(),
        IconLibrary::Phosphor => format!("import {{ {name} }} from '@phosphor-icons/react'"),
        IconLibrary::Lucide => format!("import {{ {name} }} from 'lucide-react'"),
        IconLibrary::Heroicons => {
            format!("import {{ {name}Icon }} from '@heroicons/react/24/outline'")
        }
    }
}

fn should_skip_zero_sized_image(node: &Node) -> bool {
    let Some(layout) = &node.layout else {
        return false;
    };
    let zero = |d: &crate::ir::schema::Dimension| {
        d.dim_type == DimensionType::Fixed && d.value.is_some_and(|v| v <= 0.01)
    };
    layout.width.as_ref().is_some_and(zero) || layout.height.as_ref().is_some_and(zero)
}

fn apply_responsive_root_sizing(classes: &mut Vec<String>) {
    let mut replace_at = None;
    let mut max_w = None;
    for (i, c) in classes.iter().enumerate() {
        if c.starts_with("w-[") && c.ends_with("px]") {
            replace_at = Some(i);
            max_w = Some(c.replace("w-[", "max-w-["));
            break;
        }
    }
    if let Some(i) = replace_at {
        classes[i] = "w-full".into();
        if let Some(max_w) = max_w {
            classes.insert(i + 1, max_w);
        }
    }
}

fn collect_style_classes(
    classes: &mut Vec<String>,
    node: &Node,
    theme_colors: Option<&HashMap<String, String>>,
    warnings: &mut WarningCollector,
) {
    if let Some(ref s) = node.style {
        if let Some(ref fills) = s.fills {
            classes.extend(style::fill_classes(
                fills,
                theme_colors,
                warnings,
                &node.id,
                &node.name,
            ));
        }
        collect_non_fill_styles(classes, s, theme_colors, &node.id, &node.name, warnings);
    }
}

/// Like collect_style_classes but skips fills — for image nodes where the
/// SVG/PNG already contains the fill colors.
fn collect_style_classes_no_fills(
    classes: &mut Vec<String>,
    node: &Node,
    _theme_colors: Option<&HashMap<String, String>>,
    warnings: &mut WarningCollector,
) {
    if let Some(ref s) = node.style {
        collect_non_fill_styles(classes, s, _theme_colors, &node.id, &node.name, warnings);
    }
}

fn collect_non_fill_styles(
    classes: &mut Vec<String>,
    s: &Style,
    theme_colors: Option<&HashMap<String, String>>,
    node_id: &str,
    node_name: &str,
    warnings: &mut WarningCollector,
) {
    if let Some(ref br) = s.border_radius {
        classes.extend(style::border_radius_classes(br));
    }
    if let Some(ref stroke) = s.stroke {
        classes.extend(style::stroke_classes(stroke, theme_colors));
    }
    if let Some(ref effects) = s.effects {
        classes.extend(style::effect_classes(effects, warnings));
    }
    if let Some(opacity) = s.opacity
        && (opacity - 1.0).abs() > 0.01
    {
        classes.push(style::opacity_class(opacity));
    }
    if let Some(ref blend) = s.blend_mode
        && let Some(cls) = style::blend_mode_class(blend, node_id, node_name, warnings)
    {
        classes.push(cls);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::schema::{
        Alignment, BorderRadius, ComponentInfo, FillRule, Layout, LayoutMode, Padding, Style,
        TextProps, VectorProps,
    };
    use crate::warning::WarningCollector;

    fn simple_frame(name: &str, children: Vec<Node>) -> Node {
        Node {
            id: "test-id".into(),
            name: name.into(),
            node_type: NodeType::Frame,
            layout: Some(Layout {
                mode: Some(LayoutMode::Vertical),
                width: None,
                height: None,
                padding: Some(Padding {
                    top: 16.0,
                    right: 16.0,
                    bottom: 16.0,
                    left: 16.0,
                }),
                gap: Some(8.0),
                main_axis_align: Some(Alignment::Center),
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
            style: Some(Style {
                fills: Some(vec![Fill::Solid {
                    color: "#FFFFFF".into(),
                    opacity: None,
                }]),
                stroke: None,
                border_radius: Some(BorderRadius {
                    top_left: 8.0,
                    top_right: 8.0,
                    bottom_right: 8.0,
                    bottom_left: 8.0,
                }),
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
            children,
            overlay: false,
        }
    }

    fn text_node(content: &str) -> Node {
        Node {
            id: "text-id".into(),
            name: "Label".into(),
            node_type: NodeType::Text,
            layout: None,
            style: Some(Style {
                fills: Some(vec![Fill::Solid {
                    color: "#000000".into(),
                    opacity: None,
                }]),
                stroke: None,
                border_radius: None,
                effects: None,
                opacity: None,
                blend_mode: None,
            }),
            text: Some(TextProps {
                content: content.into(),
                font_size: Some(16.0),
                font_family: None,
                font_weight: Some(400),
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
    fn test_generate_simple_component() {
        let mut warnings = WarningCollector::new();
        let node = simple_frame("Card", vec![text_node("Hello")]);
        let output = generate_component(
            &node,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &HashMap::new(),
            &mut warnings,
        );
        assert!(output.contains("export function Card"));
        assert!(output.contains("flex flex-col"));
        assert!(output.contains("p-[16px]"));
        assert!(output.contains("rounded-[8px]"));
        assert!(output.contains("Hello"));
    }

    #[test]
    fn test_generate_component_with_text_color() {
        let mut warnings = WarningCollector::new();
        let node = simple_frame("Box", vec![text_node("Colored")]);
        let output = generate_component(
            &node,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &HashMap::new(),
            &mut warnings,
        );
        assert!(output.contains("text-[#000000]"));
    }

    #[test]
    fn test_no_cn_import_without_variants() {
        let mut warnings = WarningCollector::new();
        let node = simple_frame("Simple", vec![]);
        let output = generate_component(
            &node,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &HashMap::new(),
            &mut warnings,
        );
        assert!(!output.contains("import { cn }"));
    }

    #[test]
    fn test_variant_component_has_cn_import() {
        let mut warnings = WarningCollector::new();
        let mut node = simple_frame("Button", vec![text_node("Click")]);
        node.component = Some(ComponentInfo {
            is_component: true,
            variants: Some(HashMap::from([(
                "size".into(),
                vec!["sm".into(), "md".into()],
            )])),
            variant_values: Some(HashMap::from([("size".into(), "md".into())])),
        });
        let output = generate_component(
            &node,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &HashMap::new(),
            &mut warnings,
        );
        assert!(output.contains("import { cn }"));
        assert!(output.contains("interface ButtonProps"));
        assert!(output.contains("size?:"));
    }

    #[test]
    fn test_image_node_renders_img_tag() {
        let mut warnings = WarningCollector::new();
        let image = Node {
            id: "img-1".into(),
            name: "Hero".into(),
            node_type: NodeType::Image,
            layout: None,
            style: Some(Style {
                fills: Some(vec![Fill::Image {
                    asset_ref: "asset-123".into(),
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
        let asset_map: AssetMap = HashMap::from([("asset-123".into(), "hero-image.png".into())]);
        let parent = simple_frame("Page", vec![image]);
        let output = generate_component(
            &parent,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &asset_map,
            &mut warnings,
        );
        assert!(output.contains("<img"));
        assert!(output.contains("hero-image.png"));
        assert!(output.contains("alt=\"Hero\""));
    }

    #[test]
    fn test_vector_nodes_are_skipped() {
        let mut warnings = WarningCollector::new();
        let vector = Node {
            id: "vec-1".into(),
            name: "Arrow".into(),
            node_type: NodeType::Vector,
            layout: None,
            style: None,
            text: None,
            vector: Some(VectorProps {
                svg_path: "M10 20 L30 40".into(),
                fill_rule: Some(FillRule::Nonzero),
            }),
            vector_paths: None,
            boolean_op: None,
            mask: None,
            component: None,
            children: vec![],
            overlay: false,
        };
        let parent = simple_frame("Icon", vec![vector]);
        let output = generate_component(
            &parent,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &HashMap::new(),
            &mut warnings,
        );
        // Vector nodes are skipped — icons come from icon libraries
        assert!(!output.contains("<svg"));
        assert!(!output.contains("M10 20 L30 40"));
    }

    #[test]
    fn test_font_block_generated_for_google_font() {
        let mut warnings = WarningCollector::new();
        let mut title = text_node("Hello");
        if let Some(text) = title.text.as_mut() {
            text.font_family = Some("Inter".into());
        }
        let node = simple_frame("Card", vec![title]);
        let output = generate_component(
            &node,
            None,
            "../utils/cn",
            "/assets",
            false,
            &IconLibrary::None,
            &HashMap::new(),
            &mut warnings,
        );
        assert!(output.contains("import { Inter } from 'next/font/google';"));
        assert!(output.contains("const inter = Inter"));
    }
}
