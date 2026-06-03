//! Post-process pandoc's GFM output. Structured-HTML sources (EPUB, raw HTML)
//! produce already-clean text — but pandoc preserves the original `<div>`/`<span>`
//! wrappers, and calibre (for MOBI→EPUB) inlines code as `<span class="kbd …">`.
//! This crate strips that noise and recovers inline code as proper backticks.
//!
//! Shared by the `ebook` and `html` extractor adapters: both run pandoc to GFM,
//! then `clean()` the result.

use once_cell::sync::Lazy;
use regex::Regex;

/// Calibre-specific: `<span class="kbd calibre12">CODE</span>` → `` `CODE` ``.
/// Allows escaped `\<` and `\>` inside the span (pandoc escapes them in GFM).
static RX_KBD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<span class="[^"]*\bkbd\b[^"]*">((?:[^<]|\\<)+?)</span>"#).unwrap()
});

/// Cross-reference `<a href="…" class="…">TEXT</a>` → `TEXT`.
static RX_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<a\s+href="[^"]*"[^>]*>([^<]+)</a>"#).unwrap()
});

/// Generic `<span …>X</span>` → `X` (no nested span). Run repeatedly for nesting.
static RX_SPAN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<span\b[^>]*>([^<]*)</span>"#).unwrap()
});

/// Standalone empty span anchors: `<span id="…"></span>`.
static RX_EMPTY_ANCHOR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"<span id="[^"]*"></span>"#).unwrap()
});

/// Line-only `<div …>` or `</div>` — structural wrappers.
static RX_DIV_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^[ \t]*</?div\b[^>]*>[ \t]*$").unwrap()
});

/// Open/close tags we strip but keep inner: `<aside>`, `<article>`.
static RX_WRAPPER_TAGS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"</?(?:aside|article)\b[^>]*>").unwrap()
});

/// Whole `<figure>…</figure>` block (drop — visuals we can't embed).
static RX_FIGURE_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)<figure\b[^>]*>.*?</figure>").unwrap()
});

/// Whole `<svg>…</svg>` block.
static RX_SVG_BLOCK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?s)<svg\b[^>]*>.*?</svg>").unwrap()
});

/// Self-closing `<image .../>` or `<img .../>`.
static RX_IMAGE_TAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<(?:image|img)\b[^>]*/?>").unwrap()
});

/// Calibre's `mce-root` code-fence label (artefact of their TinyMCE editor).
static RX_MCE_FENCE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^```\s*mce-root\s*$").unwrap()
});

/// 3+ blank lines → exactly 2.
static RX_BLANK_RUN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\n{3,}").unwrap()
});

pub fn clean(input: &str) -> String {
    let mut s = input.to_string();

    // 1. Recover calibre's kbd-wrapped inline code as backticks (must run
    //    before the generic span stripper, which would lose the markup).
    s = RX_KBD.replace_all(&s, "`$1`").into_owned();

    // 1b. Cross-reference anchors → bare text.
    s = RX_ANCHOR.replace_all(&s, "$1").into_owned();

    // 1c. Wrapper tags that should disappear but keep their inner content.
    s = RX_WRAPPER_TAGS.replace_all(&s, "").into_owned();

    // 2. Drop generic spans (multiple passes to handle pandoc's nesting).
    for _ in 0..5 {
        let next = RX_SPAN.replace_all(&s, "$1").into_owned();
        if next == s { break; }
        s = next;
    }
    s = RX_EMPTY_ANCHOR.replace_all(&s, "").into_owned();

    // 3. Drop the `<div>` scaffolding entirely (was just layout).
    s = RX_DIV_LINE.replace_all(&s, "").into_owned();

    // 4. Drop figures/svg/image blocks — visuals we don't capture.
    s = RX_FIGURE_BLOCK.replace_all(&s, "").into_owned();
    s = RX_SVG_BLOCK.replace_all(&s, "").into_owned();
    s = RX_IMAGE_TAG.replace_all(&s, "").into_owned();

    // 5. Normalise calibre's `mce-root` fences to bare `` ``` ``.
    s = RX_MCE_FENCE.replace_all(&s, "```").into_owned();

    // 6. Collapse 3+ blank lines into 2 so chunks don't fragment on whitespace.
    s = RX_BLANK_RUN.replace_all(&s, "\n\n").into_owned();

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_calibre_kbd_as_backticks() {
        let input = r#"the <span class="kbd calibre12">get_second()</span> function"#;
        assert_eq!(clean(input), "the `get_second()` function");
    }

    #[test]
    fn strips_generic_spans_keeping_text() {
        let input = r#"by <span class="firstname">Maxwell</span> <span class="surname">Flitton</span>"#;
        assert_eq!(clean(input), "by Maxwell Flitton");
    }

    #[test]
    fn drops_div_scaffolding() {
        let input = "<div class=\"section\">\n# Heading\n\nProse here.\n</div>";
        assert_eq!(clean(input), "\n# Heading\n\nProse here.\n");
    }

    #[test]
    fn drops_figure_block() {
        let input = "before\n<figure data-type=\"cover\">\n<img src=\"x.png\" />\n</figure>\nafter";
        assert!(clean(input).contains("before"));
        assert!(clean(input).contains("after"));
        assert!(!clean(input).contains("<figure"));
    }

    #[test]
    fn strips_cross_ref_anchor_keeping_text() {
        let input = r##"see <a href="#p20.html" class="calibre9" target="_blank">Chapter 1</a>, then"##;
        assert_eq!(clean(input), "see Chapter 1, then");
    }

    #[test]
    fn normalises_mce_root_fences() {
        let input = "```mce-root\nlet x = 1;\n```";
        assert_eq!(clean(input), "```\nlet x = 1;\n```");
    }

    #[test]
    fn collapses_excessive_blank_lines() {
        let input = "para1\n\n\n\n\npara2";
        assert_eq!(clean(input), "para1\n\npara2");
    }

    #[test]
    fn idempotent_on_clean_markdown() {
        let clean_in = "# Heading\n\nA paragraph with `code` and *emphasis*.\n\n- item\n- item\n";
        assert_eq!(clean(clean_in), clean_in);
    }
}
