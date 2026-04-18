//! Shared font helpers used by both codegen and tailwind class emission.
//!
//! `GOOGLE_FONTS` is the canonical curated list of families we know how to
//! auto-import via `next/font/google`. Families outside this list are treated
//! as "custom" — the component file gets an inline TSX comment and a runtime
//! warning, and text using that family falls back to a generic Tailwind
//! `font-serif` / `font-sans` class instead of emitting a dead CSS variable
//! reference.

/// Curated list of Google Fonts we wire up automatically via next/font/google.
/// Keep this in sync with the Next.js `next/font/google` named exports.
/// Alphabetical; case-sensitive — must match the family name exactly.
pub const GOOGLE_FONTS: &[&str] = &[
    "Abril Fatface",
    "Alegreya",
    "Anton",
    "Arapey",
    "Archivo",
    "Archivo Black",
    "Arvo",
    "Asap",
    "Assistant",
    "Barlow",
    "Barlow Condensed",
    "Be Vietnam Pro",
    "Bitter",
    "Bodoni Moda",
    "Bricolage Grotesque",
    "Cabin",
    "Catamaran",
    "Caveat",
    "Comfortaa",
    "Cormorant Garamond",
    "Courier Prime",
    "Crimson Pro",
    "DM Mono",
    "DM Sans",
    "DM Serif Display",
    "Darker Grotesque",
    "Dela Gothic One",
    "Domine",
    "EB Garamond",
    "Eczar",
    "Electrolize",
    "Epilogue",
    "Figtree",
    "Fira Code",
    "Fira Mono",
    "Fira Sans",
    "Fjalla One",
    "Fraunces",
    "Funnel Sans",
    "Gabarito",
    "Geist",
    "Gelasio",
    "Grandstander",
    "Great Vibes",
    "Hanken Grotesk",
    "Heebo",
    "Host Grotesk",
    "IBM Plex Sans",
    "Inconsolata",
    "Inria Sans",
    "Instrument Sans",
    "Instrument Serif",
    "Inter",
    "Inter Tight",
    "Italiana",
    "JetBrains Mono",
    "Josefin Sans",
    "Kanit",
    "Karantina",
    "Karla",
    "Lato",
    "Lexend",
    "Libre Baskerville",
    "Libre Caslon Text",
    "Libre Franklin",
    "Lilita One",
    "Limelight",
    "Literata",
    "Lora",
    "Maitree",
    "Major Mono Display",
    "Manrope",
    "Marcellus",
    "Material Icons",
    "Material Symbols Outlined",
    "Material Symbols Rounded",
    "Material Symbols Sharp",
    "Merriweather",
    "Merriweather Sans",
    "Montserrat",
    "Mr Dafoe",
    "Mulish",
    "Neucha",
    "Newsreader",
    "Noto Sans",
    "Noto Serif",
    "Nunito",
    "Nunito Sans",
    "Onest",
    "Open Sans",
    "Oswald",
    "Outfit",
    "Overpass",
    "Oxygen",
    "PT Sans",
    "PT Serif",
    "Pacifico",
    "Permanent Marker",
    "Philosopher",
    "Pinyon Script",
    "Playfair",
    "Playfair Display",
    "Plus Jakarta Sans",
    "Poppins",
    "Press Start 2P",
    "Quattrocento",
    "Quicksand",
    "Raleway",
    "Red Hat Display",
    "Red Hat Mono",
    "Red Hat Text",
    "Reem Kufi",
    "Roboto",
    "Roboto Condensed",
    "Roboto Mono",
    "Roboto Serif",
    "Roboto Slab",
    "Rozha One",
    "Rubik",
    "Rubik Mono One",
    "Sacramento",
    "Satisfy",
    "Schibsted Grotesk",
    "Shrikhand",
    "Signika",
    "Silkscreen",
    "Sora",
    "Source Code Pro",
    "Source Sans 3",
    "Source Serif 4",
    "Space Grotesk",
    "Space Mono",
    "Spectral",
    "Staatliches",
    "Syne",
    "Syne Mono",
    "Tenor Sans",
    "Tinos",
    "Titillium Web",
    "Ubuntu",
    "Ubuntu Mono",
    "Unbounded",
    "Vollkorn",
    "Work Sans",
    "Young Serif",
    "Zilla Slab",
];

/// Returns true if `family` is a Google font we know how to auto-import.
/// Custom families fall through to a Tailwind fallback class.
pub fn is_google_font(family: &str) -> bool {
    GOOGLE_FONTS.contains(&family)
}

/// Tailwind fallback class for a custom (non-Google) family.
/// Heuristic: names suggesting a serif lineage (contains "serif", "roman",
/// or "times", case-insensitive) → `font-serif`; everything else → `font-sans`.
pub fn custom_font_fallback_class(family: &str) -> &'static str {
    let lower = family.to_ascii_lowercase();
    if lower.contains("serif") || lower.contains("roman") || lower.contains("times") {
        "font-serif"
    } else {
        "font-sans"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inter_is_google_font() {
        assert!(is_google_font("Inter"));
    }

    #[test]
    fn jetbrains_mono_is_google_font() {
        assert!(is_google_font("JetBrains Mono"));
    }

    #[test]
    fn custom_font_is_not_google() {
        assert!(!is_google_font("Perfectly Nineties"));
        assert!(!is_google_font("Neue Haas Grotesk"));
    }

    #[test]
    fn serif_hint_picks_font_serif() {
        assert_eq!(
            custom_font_fallback_class("Perfectly Nineties Serif"),
            "font-serif"
        );
        assert_eq!(custom_font_fallback_class("Times New Roman"), "font-serif");
        assert_eq!(custom_font_fallback_class("Roman Antique"), "font-serif");
    }

    #[test]
    fn no_hint_picks_font_sans() {
        assert_eq!(
            custom_font_fallback_class("Perfectly Nineties"),
            "font-sans"
        );
        assert_eq!(custom_font_fallback_class("Neue Haas Grotesk"), "font-sans");
    }

    #[test]
    fn test_is_google_font_expanded() {
        // A sample of families that used to silently fall back to font-sans
        // before the GOOGLE_FONTS list was expanded.
        for family in [
            "IBM Plex Sans",
            "Outfit",
            "Instrument Sans",
            "Geist",
            "Space Grotesk",
            "Inter Tight",
            "Figtree",
            "Bricolage Grotesque",
            "JetBrains Mono",
            "Material Symbols Outlined",
        ] {
            assert!(is_google_font(family), "{family} should be a Google font");
        }
        // Sanity: Inter is still on the list.
        assert!(is_google_font("Inter"));
    }

    #[test]
    fn google_fonts_is_alphabetical() {
        for pair in GOOGLE_FONTS.windows(2) {
            assert!(
                pair[0] < pair[1],
                "GOOGLE_FONTS not alphabetical: {:?} before {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}
