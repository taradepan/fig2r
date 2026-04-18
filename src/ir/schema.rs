use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignIR {
    pub version: String,
    pub name: String,
    pub theme: Option<Theme>,
    #[serde(default)]
    pub components: Vec<Node>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub colors: Option<HashMap<String, String>>,
    pub spacing: Option<HashMap<String, String>>,
    #[serde(rename = "borderRadius")]
    pub border_radius: Option<HashMap<String, String>>,
    #[serde(rename = "fontSize")]
    pub font_size: Option<HashMap<String, String>>,
    #[serde(rename = "fontFamily")]
    pub font_family: Option<HashMap<String, String>>,
    pub shadows: Option<HashMap<String, String>>,
    pub opacity: Option<HashMap<String, f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub layout: Option<Layout>,
    pub style: Option<Style>,
    pub text: Option<TextProps>,
    pub vector: Option<VectorProps>,
    /// Inline SVG paths extracted from fillGeometry/strokeGeometry.
    /// When present, renders as inline `<svg><path/></svg>` instead of remote asset.
    #[serde(rename = "vectorPaths", skip_serializing_if = "Option::is_none")]
    pub vector_paths: Option<Vec<VectorPath>>,
    #[serde(rename = "booleanOp")]
    pub boolean_op: Option<BooleanOp>,
    pub mask: Option<Mask>,
    pub component: Option<ComponentInfo>,
    #[serde(default)]
    pub children: Vec<Node>,
    /// True if this node is a modal/overlay that should be absolutely positioned
    #[serde(default)]
    pub overlay: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Frame,
    Text,
    Image,
    Vector,
    Group,
    Instance,
    BooleanOp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layout {
    pub mode: Option<LayoutMode>,
    pub width: Option<Dimension>,
    pub height: Option<Dimension>,
    pub padding: Option<Padding>,
    pub gap: Option<f64>,
    #[serde(rename = "mainAxisAlign")]
    pub main_axis_align: Option<Alignment>,
    #[serde(rename = "crossAxisAlign")]
    pub cross_axis_align: Option<Alignment>,
    pub constraints: Option<Constraints>,
    pub position: Option<Position>,
    pub overflow: Option<Overflow>,
    pub rotation: Option<f64>,
    /// Parent's flex direction — used to decide if FILL means flex-1 (main axis) or default stretch (cross axis).
    #[serde(rename = "parentFlexDir", skip_serializing_if = "Option::is_none")]
    pub parent_flex_dir: Option<LayoutMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wrap: Option<bool>,
    #[serde(rename = "wrapGap", skip_serializing_if = "Option::is_none")]
    pub wrap_gap: Option<f64>,
    #[serde(rename = "wrapAlign", skip_serializing_if = "Option::is_none")]
    pub wrap_align: Option<Alignment>,
    #[serde(rename = "minWidth", skip_serializing_if = "Option::is_none")]
    pub min_width: Option<f64>,
    #[serde(rename = "maxWidth", skip_serializing_if = "Option::is_none")]
    pub max_width: Option<f64>,
    #[serde(rename = "minHeight", skip_serializing_if = "Option::is_none")]
    pub min_height: Option<f64>,
    #[serde(rename = "maxHeight", skip_serializing_if = "Option::is_none")]
    pub max_height: Option<f64>,
    #[serde(rename = "selfAlign", skip_serializing_if = "Option::is_none")]
    pub self_align: Option<Alignment>,
    #[serde(rename = "overflowX", skip_serializing_if = "Option::is_none")]
    pub overflow_x: Option<Overflow>,
    #[serde(rename = "overflowY", skip_serializing_if = "Option::is_none")]
    pub overflow_y: Option<Overflow>,
    #[serde(rename = "zIndex", skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
    #[serde(rename = "aspectRatio", skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<f64>,
    /// Grid container: column template (e.g. ["1fr", "200px", "min-content"])
    #[serde(rename = "gridColumnsSizing", skip_serializing_if = "Option::is_none")]
    pub grid_columns_sizing: Option<Vec<String>>,
    #[serde(rename = "gridRowsSizing", skip_serializing_if = "Option::is_none")]
    pub grid_rows_sizing: Option<Vec<String>>,
    #[serde(rename = "gridColumnGap", skip_serializing_if = "Option::is_none")]
    pub grid_column_gap: Option<f64>,
    #[serde(rename = "gridRowGap", skip_serializing_if = "Option::is_none")]
    pub grid_row_gap: Option<f64>,
    /// Grid child: span/start positioning
    #[serde(rename = "gridColumnSpan", skip_serializing_if = "Option::is_none")]
    pub grid_column_span: Option<u32>,
    #[serde(rename = "gridRowSpan", skip_serializing_if = "Option::is_none")]
    pub grid_row_span: Option<u32>,
    #[serde(rename = "gridColumnStart", skip_serializing_if = "Option::is_none")]
    pub grid_column_start: Option<u32>,
    #[serde(rename = "gridRowStart", skip_serializing_if = "Option::is_none")]
    pub grid_row_start: Option<u32>,
    /// Horizontal flip (mirror) — emitted as `scale-x-[-1]`
    #[serde(rename = "flipX", skip_serializing_if = "Option::is_none")]
    pub flip_x: Option<bool>,
    /// Vertical flip — emitted as `scale-y-[-1]`
    #[serde(rename = "flipY", skip_serializing_if = "Option::is_none")]
    pub flip_y: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    Horizontal,
    Vertical,
    Grid,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimension {
    #[serde(rename = "type")]
    pub dim_type: DimensionType,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DimensionType {
    Fixed,
    Fill,
    Hug,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Padding {
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub right: f64,
    #[serde(default)]
    pub bottom: f64,
    #[serde(default)]
    pub left: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    Start,
    Center,
    End,
    SpaceBetween,
    Stretch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    pub horizontal: Option<ConstraintAxis>,
    pub vertical: Option<ConstraintAxis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConstraintAxis {
    Left,
    Right,
    Center,
    Stretch,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Style {
    pub fills: Option<Vec<Fill>>,
    pub stroke: Option<Stroke>,
    #[serde(rename = "borderRadius")]
    pub border_radius: Option<BorderRadius>,
    pub effects: Option<Vec<Effect>>,
    pub opacity: Option<f64>,
    #[serde(rename = "blendMode")]
    pub blend_mode: Option<BlendMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Fill {
    Solid {
        color: String,
        opacity: Option<f64>,
    },
    Gradient {
        #[serde(rename = "gradientType")]
        gradient_type: GradientType,
        stops: Vec<GradientStop>,
        /// Gradient angle in degrees (0=right, 90=down, 180=left, 270=up)
        #[serde(skip_serializing_if = "Option::is_none")]
        angle: Option<f64>,
    },
    Image {
        #[serde(rename = "assetRef")]
        asset_ref: String,
        #[serde(rename = "scaleMode", skip_serializing_if = "Option::is_none")]
        scale_mode: Option<ScaleMode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GradientType {
    Linear,
    Radial,
    Angular,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ScaleMode {
    Fill,
    Fit,
    Crop,
    Tile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradientStop {
    pub position: f64,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stroke {
    pub color: String,
    pub width: f64,
    pub position: Option<StrokePosition>,
    /// Per-side widths: [top, right, bottom, left]. Overrides `width` when present.
    #[serde(rename = "sideWidths", skip_serializing_if = "Option::is_none")]
    pub side_widths: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dashed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StrokePosition {
    Inside,
    Outside,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorderRadius {
    #[serde(rename = "topLeft", default)]
    pub top_left: f64,
    #[serde(rename = "topRight", default)]
    pub top_right: f64,
    #[serde(rename = "bottomRight", default)]
    pub bottom_right: f64,
    #[serde(rename = "bottomLeft", default)]
    pub bottom_left: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Effect {
    DropShadow {
        offset: Position,
        radius: f64,
        spread: Option<f64>,
        color: String,
    },
    InnerShadow {
        offset: Position,
        radius: f64,
        spread: Option<f64>,
        color: String,
    },
    Blur {
        #[serde(rename = "blurType")]
        blur_type: Option<BlurType>,
        radius: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BlurType {
    Layer,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    #[serde(rename = "color-dodge")]
    ColorDodge,
    #[serde(rename = "color-burn")]
    ColorBurn,
    #[serde(rename = "hard-light")]
    HardLight,
    #[serde(rename = "soft-light")]
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextProps {
    pub content: String,
    #[serde(rename = "fontSize")]
    pub font_size: Option<f64>,
    #[serde(rename = "fontFamily")]
    pub font_family: Option<String>,
    #[serde(rename = "fontWeight")]
    pub font_weight: Option<u32>,
    #[serde(rename = "lineHeight")]
    pub line_height: Option<f64>,
    #[serde(rename = "letterSpacing")]
    pub letter_spacing: Option<f64>,
    #[serde(rename = "textAlign")]
    pub text_align: Option<TextAlign>,
    #[serde(rename = "textDecoration")]
    pub text_decoration: Option<TextDecoration>,
    #[serde(
        rename = "textDecorationStyle",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_decoration_style: Option<TextDecorationStyle>,
    #[serde(
        rename = "textDecorationOffset",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_decoration_offset: Option<f64>,
    #[serde(
        rename = "textDecorationThickness",
        skip_serializing_if = "Option::is_none"
    )]
    pub text_decoration_thickness: Option<f64>,
    #[serde(rename = "textTransform")]
    pub text_transform: Option<TextTransform>,
    pub truncation: Option<Truncation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(rename = "verticalAlign", skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    #[serde(rename = "paragraphSpacing", skip_serializing_if = "Option::is_none")]
    pub paragraph_spacing: Option<f64>,
    #[serde(rename = "maxLines", skip_serializing_if = "Option::is_none")]
    pub max_lines: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperlink: Option<String>,
    #[serde(rename = "listType", skip_serializing_if = "Option::is_none")]
    pub list_type: Option<ListType>,
    #[serde(rename = "opentypeFlags", skip_serializing_if = "Option::is_none")]
    pub opentype_flags: Option<HashMap<String, i32>>,
    /// Rich text spans — when present, content is split into styled runs.
    /// Each span may override base text style (weight, italic, color, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spans: Option<Vec<TextSpan>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextSpan {
    pub content: String,
    #[serde(rename = "fontWeight", skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(rename = "textDecoration", skip_serializing_if = "Option::is_none")]
    pub text_decoration: Option<TextDecoration>,
    #[serde(rename = "fontFamily", skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(rename = "fontSize", skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperlink: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ListType {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VerticalAlign {
    Top,
    Center,
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextDecoration {
    None,
    Underline,
    Strikethrough,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextDecorationStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
    #[serde(rename = "small-caps")]
    SmallCaps,
    #[serde(rename = "all-small-caps")]
    AllSmallCaps,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Truncation {
    None,
    Ellipsis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorProps {
    #[serde(rename = "svgPath")]
    pub svg_path: String,
    #[serde(rename = "fillRule")]
    pub fill_rule: Option<FillRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPath {
    pub d: String,
    #[serde(rename = "fillRule", skip_serializing_if = "Option::is_none")]
    pub fill_rule: Option<FillRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(rename = "strokeWidth", skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FillRule {
    Nonzero,
    Evenodd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BooleanOp {
    pub operation: BooleanOperation,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BooleanOperation {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mask {
    #[serde(rename = "isMask")]
    pub is_mask: bool,
    #[serde(rename = "maskType")]
    pub mask_type: MaskType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MaskType {
    Alpha,
    Vector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    #[serde(rename = "isComponent")]
    pub is_component: bool,
    pub variants: Option<HashMap<String, Vec<String>>>,
    #[serde(rename = "variantValues")]
    pub variant_values: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub asset_type: AssetType,
    pub format: String,
    /// Base64-encoded data (images) or raw SVG markup. Optional — Claude can download images
    /// separately and just include asset refs in the IR for path resolution.
    #[serde(default)]
    pub data: String,
    /// Original URL of the asset (informational — fig2r does not download, Claude handles that)
    pub url: Option<String>,
    /// Stable source identifier used for deduplication (e.g. Figma imageRef hash).
    #[serde(rename = "sourceRef")]
    pub source_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Image,
    Svg,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_minimal_ir() {
        let json = r##"{
            "version": "1.0",
            "name": "TestExport",
            "components": [
                {
                    "id": "frame-1",
                    "name": "Container",
                    "type": "frame",
                    "children": []
                }
            ]
        }"##;
        let ir: DesignIR = serde_json::from_str(json).unwrap();
        assert_eq!(ir.version, "1.0");
        assert_eq!(ir.name, "TestExport");
        assert_eq!(ir.components.len(), 1);
        assert_eq!(ir.components[0].name, "Container");
    }

    #[test]
    fn test_deserialize_node_with_layout() {
        let json = r##"{
            "id": "node-1",
            "name": "Row",
            "type": "frame",
            "layout": {
                "mode": "horizontal",
                "width": { "type": "fill" },
                "height": { "type": "hug" },
                "padding": { "top": 8, "right": 16, "bottom": 8, "left": 16 },
                "gap": 12,
                "mainAxisAlign": "center",
                "crossAxisAlign": "stretch"
            },
            "children": []
        }"##;
        let node: Node = serde_json::from_str(json).unwrap();
        let layout = node.layout.unwrap();
        assert_eq!(layout.mode, Some(LayoutMode::Horizontal));
        assert_eq!(layout.gap, Some(12.0));
        assert_eq!(layout.padding.unwrap().top, 8.0);
    }

    #[test]
    fn test_deserialize_node_with_style() {
        let json = r##"{
            "id": "node-2",
            "name": "Box",
            "type": "frame",
            "style": {
                "fills": [
                    { "type": "solid", "color": "#3B82F6", "opacity": 1.0 }
                ],
                "borderRadius": { "topLeft": 8, "topRight": 8, "bottomRight": 8, "bottomLeft": 8 },
                "opacity": 0.9
            },
            "children": []
        }"##;
        let node: Node = serde_json::from_str(json).unwrap();
        let style = node.style.unwrap();
        assert_eq!(style.fills.unwrap().len(), 1);
        assert_eq!(style.opacity, Some(0.9));
    }

    #[test]
    fn test_deserialize_text_node() {
        let json = r##"{
            "id": "text-1",
            "name": "Label",
            "type": "text",
            "text": {
                "content": "Hello World",
                "fontSize": 16,
                "fontWeight": 600,
                "textAlign": "center"
            },
            "children": []
        }"##;
        let node: Node = serde_json::from_str(json).unwrap();
        let text = node.text.unwrap();
        assert_eq!(text.content, "Hello World");
        assert_eq!(text.font_size, Some(16.0));
        assert_eq!(text.font_weight, Some(600));
        assert_eq!(text.text_align, Some(TextAlign::Center));
    }

    #[test]
    fn test_deserialize_component_with_variants() {
        let json = r##"{
            "id": "btn-1",
            "name": "Button",
            "type": "frame",
            "component": {
                "isComponent": true,
                "variants": {
                    "size": ["sm", "md", "lg"],
                    "variant": ["primary", "secondary"]
                },
                "variantValues": { "size": "md", "variant": "primary" }
            },
            "children": []
        }"##;
        let node: Node = serde_json::from_str(json).unwrap();
        let comp = node.component.unwrap();
        assert!(comp.is_component);
        assert_eq!(comp.variants.unwrap()["size"], vec!["sm", "md", "lg"]);
    }

    #[test]
    fn test_deserialize_asset() {
        let json = r#"{
            "id": "asset-1",
            "name": "logo",
            "type": "svg",
            "format": "svg",
            "data": "<svg>...</svg>"
        }"#;
        let asset: Asset = serde_json::from_str(json).unwrap();
        assert_eq!(asset.name, "logo");
        assert_eq!(asset.asset_type, AssetType::Svg);
    }

    #[test]
    fn test_deserialize_vector_node() {
        let json = r#"{
            "id": "vec-1",
            "name": "Icon",
            "type": "vector",
            "vector": {
                "svgPath": "M10 20 L30 40",
                "fillRule": "nonzero"
            },
            "children": []
        }"#;
        let node: Node = serde_json::from_str(json).unwrap();
        let vector = node.vector.unwrap();
        assert_eq!(vector.svg_path, "M10 20 L30 40");
    }

    #[test]
    fn test_deserialize_effects() {
        let json = r#"{
            "id": "eff-1",
            "name": "Shadow Box",
            "type": "frame",
            "style": {
                "effects": [
                    { "type": "drop-shadow", "offset": { "x": 0, "y": 4 }, "radius": 6, "spread": 0, "color": "rgba(0,0,0,0.1)" },
                    { "type": "blur", "blurType": "background", "radius": 8 }
                ]
            },
            "children": []
        }"#;
        let node: Node = serde_json::from_str(json).unwrap();
        let effects = node.style.unwrap().effects.unwrap();
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_deserialize_gradient_fill() {
        let json = r##"{
            "id": "grad-1",
            "name": "Gradient Box",
            "type": "frame",
            "style": {
                "fills": [
                    {
                        "type": "gradient",
                        "gradientType": "linear",
                        "stops": [
                            { "position": 0.0, "color": "#3B82F6" },
                            { "position": 1.0, "color": "#8B5CF6" }
                        ]
                    }
                ]
            },
            "children": []
        }"##;
        let node: Node = serde_json::from_str(json).unwrap();
        let fills = node.style.unwrap().fills.unwrap();
        if let Fill::Gradient { stops, .. } = &fills[0] {
            assert_eq!(stops.len(), 2);
        } else {
            panic!("Expected gradient fill");
        }
    }

    #[test]
    fn test_deserialize_theme() {
        let json = r##"{
            "version": "1.0",
            "name": "Themed",
            "theme": {
                "colors": { "primary": "#3B82F6" },
                "spacing": { "sm": "4px" },
                "borderRadius": { "md": "8px" },
                "fontSize": { "base": "16px" },
                "fontFamily": { "sans": "Inter" },
                "shadows": { "sm": "0 1px 2px rgba(0,0,0,0.05)" },
                "opacity": { "disabled": 0.5 }
            },
            "components": []
        }"##;
        let ir: DesignIR = serde_json::from_str(json).unwrap();
        let theme = ir.theme.unwrap();
        assert_eq!(theme.colors.unwrap()["primary"], "#3B82F6");
    }

    #[test]
    fn test_deserialize_boolean_op() {
        let json = r#"{
            "id": "bool-1",
            "name": "CombinedShape",
            "type": "boolean_op",
            "booleanOp": {
                "operation": "union",
                "children": [
                    {
                        "id": "child-1",
                        "name": "Circle",
                        "type": "vector",
                        "vector": { "svgPath": "M0 0 Z", "fillRule": "nonzero" },
                        "children": []
                    }
                ]
            },
            "children": []
        }"#;
        let node: Node = serde_json::from_str(json).unwrap();
        let bool_op = node.boolean_op.unwrap();
        assert_eq!(bool_op.operation, BooleanOperation::Union);
        assert_eq!(bool_op.children.len(), 1);
    }

    #[test]
    fn test_deserialize_mask() {
        let json = r#"{
            "id": "mask-1",
            "name": "MaskLayer",
            "type": "frame",
            "mask": { "isMask": true, "maskType": "alpha" },
            "children": []
        }"#;
        let node: Node = serde_json::from_str(json).unwrap();
        let mask = node.mask.unwrap();
        assert!(mask.is_mask);
        assert_eq!(mask.mask_type, MaskType::Alpha);
    }
}
