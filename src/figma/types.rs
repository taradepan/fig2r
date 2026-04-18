use serde::Deserialize;
use std::collections::HashMap;

/// Top-level response from GET /v1/files/:key/nodes
#[derive(Debug, Deserialize)]
pub struct FileNodesResponse {
    pub name: String,
    pub nodes: HashMap<String, Option<NodeContainer>>,
}

#[derive(Debug, Deserialize)]
pub struct NodeContainer {
    pub document: FigmaNode,
}

/// Top-level response from GET /v1/images/:key
#[derive(Debug, Deserialize)]
pub struct ImageResponse {
    pub images: HashMap<String, Option<String>>,
}

/// A Figma node — covers FRAME, TEXT, VECTOR, COMPONENT, INSTANCE, GROUP, etc.
#[derive(Debug, Deserialize)]
pub struct FigmaNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub visible: Option<bool>,
    #[serde(default)]
    pub children: Vec<FigmaNode>,

    // Layout
    #[serde(rename = "layoutMode")]
    pub layout_mode: Option<String>,
    #[serde(rename = "layoutSizingHorizontal")]
    pub layout_sizing_horizontal: Option<String>,
    #[serde(rename = "layoutSizingVertical")]
    pub layout_sizing_vertical: Option<String>,
    #[serde(rename = "primaryAxisAlignItems")]
    pub primary_axis_align_items: Option<String>,
    #[serde(rename = "counterAxisAlignItems")]
    pub counter_axis_align_items: Option<String>,
    #[serde(rename = "paddingLeft")]
    pub padding_left: Option<f64>,
    #[serde(rename = "paddingRight")]
    pub padding_right: Option<f64>,
    #[serde(rename = "paddingTop")]
    pub padding_top: Option<f64>,
    #[serde(rename = "paddingBottom")]
    pub padding_bottom: Option<f64>,
    #[serde(rename = "itemSpacing")]
    pub item_spacing: Option<f64>,
    #[serde(rename = "clipsContent")]
    pub clips_content: Option<bool>,
    #[serde(rename = "layoutPositioning")]
    pub layout_positioning: Option<String>,
    #[serde(rename = "layoutWrap")]
    pub layout_wrap: Option<String>,
    #[serde(rename = "counterAxisSpacing")]
    pub counter_axis_spacing: Option<f64>,
    #[serde(rename = "counterAxisAlignContent")]
    pub counter_axis_align_content: Option<String>,
    #[serde(rename = "minWidth")]
    pub min_width: Option<f64>,
    #[serde(rename = "maxWidth")]
    pub max_width: Option<f64>,
    #[serde(rename = "minHeight")]
    pub min_height: Option<f64>,
    #[serde(rename = "maxHeight")]
    pub max_height: Option<f64>,
    #[serde(rename = "layoutAlign")]
    pub layout_align: Option<String>,
    #[serde(rename = "layoutGrow")]
    pub layout_grow: Option<f64>,
    #[serde(rename = "overflowDirection")]
    pub overflow_direction: Option<String>,
    #[serde(rename = "gridRowGap")]
    pub grid_row_gap: Option<f64>,
    #[serde(rename = "gridColumnGap")]
    pub grid_column_gap: Option<f64>,
    #[serde(
        rename = "gridColumnsSizing",
        default,
        deserialize_with = "de_grid_sizing"
    )]
    pub grid_columns_sizing: Option<Vec<String>>,
    #[serde(
        rename = "gridRowsSizing",
        default,
        deserialize_with = "de_grid_sizing"
    )]
    pub grid_rows_sizing: Option<Vec<String>>,
    #[serde(rename = "gridColumnSpan")]
    pub grid_column_span: Option<u32>,
    #[serde(rename = "gridRowSpan")]
    pub grid_row_span: Option<u32>,
    #[serde(rename = "gridColumnAnchorIndex")]
    pub grid_column_anchor_index: Option<u32>,
    #[serde(rename = "gridRowAnchorIndex")]
    pub grid_row_anchor_index: Option<u32>,

    // Bounding box
    #[serde(rename = "absoluteBoundingBox")]
    pub absolute_bounding_box: Option<BoundingBox>,

    // Style
    #[serde(default)]
    pub fills: Vec<FigmaPaint>,
    #[serde(default)]
    pub strokes: Vec<FigmaPaint>,
    #[serde(rename = "strokeWeight")]
    pub stroke_weight: Option<f64>,
    #[serde(rename = "individualStrokeWeights")]
    pub individual_stroke_weights: Option<IndividualStrokeWeights>,
    #[serde(rename = "strokeAlign")]
    pub stroke_align: Option<String>,
    #[serde(rename = "strokeDashes")]
    pub stroke_dashes: Option<Vec<f64>>,
    #[serde(default)]
    pub effects: Vec<FigmaEffect>,
    pub opacity: Option<f64>,
    #[serde(rename = "blendMode")]
    pub blend_mode: Option<String>,
    pub rotation: Option<f64>,
    #[serde(rename = "cornerRadius")]
    pub corner_radius: Option<f64>,
    #[serde(rename = "rectangleCornerRadii")]
    pub rectangle_corner_radii: Option<[f64; 4]>,
    #[serde(rename = "fillGeometry")]
    pub fill_geometry: Option<Vec<FigmaPath>>,
    #[serde(rename = "relativeTransform")]
    pub relative_transform: Option<[[f64; 3]; 2]>,

    // Text
    pub characters: Option<String>,
    pub style: Option<FigmaTypeStyle>,
    #[serde(rename = "lineTypes")]
    pub line_types: Option<Vec<String>>,
    #[serde(rename = "characterStyleOverrides")]
    pub character_style_overrides: Option<Vec<u32>>,
    #[serde(rename = "styleOverrideTable")]
    pub style_override_table: Option<HashMap<String, FigmaTypeStyle>>,

    // Mask
    #[serde(rename = "isMask")]
    pub is_mask: Option<bool>,

    // Component
    #[serde(rename = "componentProperties")]
    pub component_properties: Option<HashMap<String, ComponentProperty>>,
    #[serde(rename = "componentPropertyDefinitions")]
    pub component_property_definitions: Option<HashMap<String, ComponentPropertyDef>>,
    #[allow(dead_code)]
    #[serde(rename = "boundVariables")]
    pub bound_variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct IndividualStrokeWeights {
    #[serde(default)]
    pub top: f64,
    #[serde(default)]
    pub right: f64,
    #[serde(default)]
    pub bottom: f64,
    #[serde(default)]
    pub left: f64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct BoundingBox {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Deserialize)]
pub struct FigmaPaint {
    #[serde(rename = "type")]
    pub paint_type: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    pub opacity: Option<f64>,
    pub color: Option<FigmaColor>,
    #[serde(rename = "gradientStops")]
    pub gradient_stops: Option<Vec<FigmaColorStop>>,
    #[serde(rename = "gradientHandlePositions")]
    pub gradient_handle_positions: Option<Vec<FigmaVector>>,
    #[serde(rename = "imageRef")]
    pub image_ref: Option<String>,
    #[serde(rename = "scaleMode")]
    pub scale_mode: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "boundVariables")]
    pub bound_variables: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct FigmaColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl FigmaColor {
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0).round() as u8;
        let g = (self.g * 255.0).round() as u8;
        let b = (self.b * 255.0).round() as u8;
        if (self.a - 1.0).abs() < 0.01 {
            format!("#{r:02X}{g:02X}{b:02X}")
        } else {
            // Use hex with alpha channel — works in Tailwind arbitrary values
            let a = (self.a * 255.0).round() as u8;
            format!("#{r:02X}{g:02X}{b:02X}{a:02X}")
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FigmaColorStop {
    pub position: f64,
    pub color: FigmaColor,
}

#[derive(Debug, Deserialize)]
pub struct FigmaEffect {
    #[serde(rename = "type")]
    pub effect_type: String,
    #[serde(default = "default_true")]
    pub visible: bool,
    pub radius: Option<f64>,
    pub color: Option<FigmaColor>,
    pub offset: Option<FigmaVector>,
    pub spread: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct FigmaVector {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FigmaPath {
    pub path: String,
    #[serde(rename = "windingRule")]
    pub winding_rule: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FigmaTypeStyle {
    #[serde(rename = "fontFamily")]
    pub font_family: Option<String>,
    #[serde(rename = "fontWeight")]
    pub font_weight: Option<f64>,
    #[serde(rename = "fontSize")]
    pub font_size: Option<f64>,
    #[serde(rename = "lineHeightPx")]
    pub line_height_px: Option<f64>,
    #[serde(rename = "lineHeightPercentFontSize")]
    pub line_height_percent_font_size: Option<f64>,
    #[serde(rename = "lineHeightUnit")]
    pub line_height_unit: Option<String>,
    #[serde(rename = "letterSpacing")]
    pub letter_spacing: Option<f64>,
    #[serde(rename = "textAlignHorizontal")]
    pub text_align_horizontal: Option<String>,
    #[serde(rename = "textDecoration")]
    pub text_decoration: Option<String>,
    #[serde(rename = "textDecorationStyle")]
    pub text_decoration_style: Option<String>,
    #[serde(rename = "textDecorationOffset")]
    pub text_decoration_offset: Option<f64>,
    #[serde(rename = "textDecorationThickness")]
    pub text_decoration_thickness: Option<f64>,
    #[serde(rename = "textCase")]
    pub text_case: Option<String>,
    #[serde(rename = "textTruncation")]
    pub text_truncation: Option<String>,
    pub italic: Option<bool>,
    #[serde(rename = "textAlignVertical")]
    pub text_align_vertical: Option<String>,
    #[serde(rename = "paragraphSpacing")]
    pub paragraph_spacing: Option<f64>,
    #[serde(rename = "maxLines")]
    pub max_lines: Option<u32>,
    pub hyperlink: Option<serde_json::Value>,
    #[serde(rename = "opentypeFlags")]
    pub opentype_flags: Option<HashMap<String, i32>>,
    pub fills: Option<Vec<FigmaPaint>>,
    #[allow(dead_code)]
    #[serde(rename = "boundVariables")]
    pub bound_variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ComponentProperty {
    #[serde(rename = "type")]
    pub prop_type: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct ComponentPropertyDef {
    #[serde(rename = "type")]
    pub prop_type: String,
    #[serde(rename = "variantOptions")]
    pub variant_options: Option<Vec<String>>,
}

/// Figma returns gridColumnsSizing / gridRowsSizing as either a per-track array
/// (`["1FR", "MIN_CONTENT"]`) or a single CSS template string
/// (`"repeat(2,minmax(0,1fr))"`). Normalize both into `Vec<String>`.
fn de_grid_sizing<'de, D>(d: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Shape {
        List(Vec<String>),
        Template(String),
    }
    Ok(Option::<Shape>::deserialize(d)?.map(|s| match s {
        Shape::List(v) => v,
        Shape::Template(s) => vec![s],
    }))
}
