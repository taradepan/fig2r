/// Map a pixel value to a Tailwind spacing class using exact arbitrary values.
/// Prefix examples: "p", "px", "mt", "gap".
pub fn spacing_class(prefix: &str, px: f64) -> String {
    if px.abs() < 0.01 {
        return format!("{prefix}-0");
    }
    format!("{prefix}-[{}px]", format_num(px))
}

/// Map px to w-N or h-N using exact arbitrary values.
pub fn dimension_class(prefix: &str, px: f64) -> String {
    spacing_class(prefix, px)
}

pub fn font_size_class(px: f64) -> String {
    if px.abs() < 0.01 {
        return "text-[0px]".to_string();
    }
    format!("text-[{}px]", format_num(px))
}

pub fn font_weight_class(weight: u32) -> String {
    let scale: &[(u32, &str)] = &[
        (100, "thin"),
        (200, "extralight"),
        (300, "light"),
        (400, "normal"),
        (500, "medium"),
        (600, "semibold"),
        (700, "bold"),
        (800, "extrabold"),
        (900, "black"),
    ];
    for &(val, name) in scale {
        if weight == val {
            return format!("font-{name}");
        }
    }
    format!("font-[{weight}]")
}

pub fn border_radius_class(px: f64) -> String {
    if px.abs() < 0.01 {
        return "rounded-none".to_string();
    }
    format!("rounded-[{}px]", format_num(px))
}

pub fn opacity_class(value: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let percent = (value * 100.0).round() as u32;
    let known: &[u32] = &[
        0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95, 100,
    ];
    if known.contains(&percent) {
        format!("opacity-{percent}")
    } else {
        format!("opacity-[{percent}%]")
    }
}

fn format_num(n: f64) -> String {
    if (n - n.floor()).abs() < f64::EPSILON {
        #[allow(clippy::cast_possible_truncation)]
        let i = n as i64;
        format!("{i}")
    } else {
        format!("{n}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spacing_zero() {
        assert_eq!(spacing_class("p", 0.0), "p-0");
        assert_eq!(spacing_class("gap", 0.0), "gap-0");
    }

    #[test]
    fn test_spacing_arbitrary_values() {
        assert_eq!(spacing_class("p", 4.0), "p-[4px]");
        assert_eq!(spacing_class("p", 8.0), "p-[8px]");
        assert_eq!(spacing_class("p", 16.0), "p-[16px]");
        assert_eq!(spacing_class("p", 32.0), "p-[32px]");
        assert_eq!(spacing_class("p", 7.0), "p-[7px]");
        assert_eq!(spacing_class("p", 13.0), "p-[13px]");
    }

    #[test]
    fn test_spacing_with_prefix() {
        assert_eq!(spacing_class("px", 16.0), "px-[16px]");
        assert_eq!(spacing_class("gap", 8.0), "gap-[8px]");
        assert_eq!(spacing_class("mt", 24.0), "mt-[24px]");
    }

    #[test]
    fn test_font_size_arbitrary_values() {
        assert_eq!(font_size_class(12.0), "text-[12px]");
        assert_eq!(font_size_class(14.0), "text-[14px]");
        assert_eq!(font_size_class(16.0), "text-[16px]");
        assert_eq!(font_size_class(18.0), "text-[18px]");
        assert_eq!(font_size_class(24.0), "text-[24px]");
        assert_eq!(font_size_class(15.0), "text-[15px]");
        assert_eq!(font_size_class(22.0), "text-[22px]");
    }

    #[test]
    fn test_font_weight_known() {
        assert_eq!(font_weight_class(100), "font-thin");
        assert_eq!(font_weight_class(300), "font-light");
        assert_eq!(font_weight_class(400), "font-normal");
        assert_eq!(font_weight_class(500), "font-medium");
        assert_eq!(font_weight_class(600), "font-semibold");
        assert_eq!(font_weight_class(700), "font-bold");
        assert_eq!(font_weight_class(900), "font-black");
    }

    #[test]
    fn test_font_weight_arbitrary() {
        assert_eq!(font_weight_class(550), "font-[550]");
    }

    #[test]
    fn test_border_radius_zero() {
        assert_eq!(border_radius_class(0.0), "rounded-none");
    }

    #[test]
    fn test_border_radius_arbitrary_values() {
        assert_eq!(border_radius_class(2.0), "rounded-[2px]");
        assert_eq!(border_radius_class(4.0), "rounded-[4px]");
        assert_eq!(border_radius_class(8.0), "rounded-[8px]");
        assert_eq!(border_radius_class(10.0), "rounded-[10px]");
        assert_eq!(border_radius_class(16.0), "rounded-[16px]");
        assert_eq!(border_radius_class(9999.0), "rounded-[9999px]");
    }

    #[test]
    fn test_opacity_known() {
        assert_eq!(opacity_class(0.0), "opacity-0");
        assert_eq!(opacity_class(0.05), "opacity-5");
        assert_eq!(opacity_class(0.5), "opacity-50");
        assert_eq!(opacity_class(1.0), "opacity-100");
    }

    #[test]
    fn test_opacity_arbitrary() {
        assert_eq!(opacity_class(0.33), "opacity-[33%]");
    }

    #[test]
    fn test_dimension_class() {
        assert_eq!(dimension_class("w", 0.0), "w-0");
        assert_eq!(dimension_class("w", 16.0), "w-[16px]");
        assert_eq!(dimension_class("w", 256.0), "w-[256px]");
        assert_eq!(dimension_class("h", 100.0), "h-[100px]");
        assert_eq!(dimension_class("w", 320.0), "w-[320px]");
    }
}
