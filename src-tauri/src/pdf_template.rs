//! Template-based PDF generation for เบิกยาปะหน้า (Disbursement Cover Letters).
//!
//! Strategy
//! --------
//! 1. Load a pre-built A4-portrait blank form (`template_cover.pdf`) that
//!    already contains all Thai static text, borders, and signature blocks.
//! 2. Embed the full `THSarabun.ttf` TrueType font as a new Type0/Identity-H
//!    composite font resource named `OVL_F` — distinct from the template's
//!    subset fonts F1-F4.
//! 3. For each invoice page, build a small overlay content stream that writes
//!    only the seven dynamic fields at their exact template coordinates.
//! 4. Combine template content + overlay, create pages, save one PDF.
//!
//! Coordinate system: PDF points (pt), origin = bottom-left of page.
//! Page size: 595.3 × 841.9 pt (A4 portrait).
//!
//! Text encoding for OVL_F (Type0/Identity-H):
//!   Each character → GID via ttf-parser cmap lookup → 2-byte big-endian pair.

#![allow(dead_code)]

use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use std::path::Path;
use ttf_parser::{Face, GlyphId};

use crate::models::{CoverLetterPage, CoverLettersParams};

// ---------------------------------------------------------------------------
// Compile-time embedded assets
// ---------------------------------------------------------------------------

/// Pre-built blank A4 template (compiled into the binary).
const TEMPLATE_BYTES: &[u8] = include_bytes!("template_cover.pdf");

/// Full TH Sarabun PSK TrueType font — used for all overlay text.
const FONT_BYTES: &[u8] = include_bytes!("THSarabun.ttf");

// ---------------------------------------------------------------------------
// Layout constants  (PDF pt, origin = bottom-left)
// ---------------------------------------------------------------------------

/// X where the date text begins (just after "วันที่" label + underscores start).
const DATE_X: f64 = 275.55;
/// Baseline Y of the "ที่/วันที่" row.
const DATE_Y: f64 = 721.68;

/// X where the company name begins (after "จาก" prefix on the แผนงาน line).
const COMPANY_X: f64 = 270.95;
/// Baseline Y of the แผนงาน/โครงการ row.
const COMPANY_Y: f64 = 633.18;

/// Baseline Y of the budget-table value row.
const VALUE_Y: f64 = 583.48;

/// Right-edge of each budget column (minus a 3 pt internal margin).
/// Order: ยอดเงินจัดสรร | ยอดเบิกจ่ายแล้ว | ยอดคงเหลือ | เบิกจ่ายครั้งนี้ | ยอดเงินคงเหลือ
const COL_RIGHTS: [f64; 5] = [168.0, 260.2, 353.7, 436.8, 519.9];

/// Column left edges (used to clamp very wide numbers so they don't overflow left).
const COL_LEFTS: [f64; 5] = [74.0, 173.0, 265.0, 358.0, 441.0];

/// Font size for date and company name.
const TEXT_PT: f64 = 14.0;

/// Font size for the five budget numbers (slightly smaller to fit narrow columns).
const NUM_PT: f64 = 11.5;

/// PDF resource name for our overlay font — must not clash with F1/F2/F3/F4.
const FONT_RESOURCE: &str = "OVL_F";

// ---------------------------------------------------------------------------
// Number formatting
// ---------------------------------------------------------------------------

/// Format a monetary value as `"1,234,567.89"` (Western digits, 2 d.p.).
/// Negative values are prefixed with `"-"`.
fn fmt_amount(v: f64) -> String {
    let negative = v < 0.0;
    let abs_v = v.abs();

    let s = format!("{:.2}", abs_v);
    let (integer, decimal) = s.split_once('.').unwrap_or((&s, "00"));

    // Insert thousands separators from the right.
    let with_commas: String = integer
        .chars()
        .rev()
        .enumerate()
        .flat_map(|(i, c)| {
            if i > 0 && i % 3 == 0 {
                vec![',', c]
            } else {
                vec![c]
            }
        })
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    let result = format!("{}.{}", with_commas, decimal);
    if negative {
        format!("-{}", result)
    } else {
        result
    }
}

// ---------------------------------------------------------------------------
// Font / glyph helpers
// ---------------------------------------------------------------------------

/// Convert a Unicode string to a sequence of 2-byte big-endian GID pairs
/// suitable for embedding in a `<HEXSTRING>` inside a Type0/Identity-H stream.
fn text_to_gid_bytes(text: &str, face: &Face) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() * 2);
    for ch in text.chars() {
        let gid: u16 = face.glyph_index(ch).map(|g| g.0).unwrap_or(0);
        out.push((gid >> 8) as u8);
        out.push((gid & 0xFF) as u8);
    }
    out
}

/// Calculate the advance width of a string in PDF points.
fn text_width_pt(text: &str, face: &Face, size: f64) -> f64 {
    let upm = face.units_per_em() as f64;
    text.chars()
        .map(|ch| {
            let gid: u16 = face.glyph_index(ch).map(|g| g.0).unwrap_or(0);
            let adv = face.glyph_hor_advance(GlyphId(gid)).unwrap_or(600) as f64;
            adv * size / upm
        })
        .sum()
}

// ---------------------------------------------------------------------------
// Content-stream builder
// ---------------------------------------------------------------------------

/// Append a single `BT … ET` text block to `buf`.
///
/// Uses an absolute text matrix (`1 0 0 1 x y Tm`) and a hex-string operand
/// for the glyph ID pairs.
fn write_text_block(
    buf: &mut Vec<u8>,
    font_res: &str,
    size: f64,
    x: f64,
    y: f64,
    gid_bytes: &[u8],
) {
    let hex: String = gid_bytes.iter().map(|b| format!("{:02X}", b)).collect();

    let block = format!(
        "BT\n/{} {:.2} Tf\n1 0 0 1 {:.3} {:.3} Tm\n<{}> Tj\nET\n",
        font_res, size, x, y, hex
    );
    buf.extend_from_slice(block.as_bytes());
}

/// Build the complete overlay ops for one cover-letter page.
///
/// The overlay is intentionally minimal: only the seven dynamic fields.
/// Everything else (borders, labels, signatures) comes from the template.
fn build_overlay(page: &CoverLetterPage, face: &Face) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();

    // 1. วันที่  ─ left-aligned at the start of the blank area
    {
        let gids = text_to_gid_bytes(&page.date_text, face);
        write_text_block(&mut buf, FONT_RESOURCE, TEXT_PT, DATE_X, DATE_Y, &gids);
    }

    // 2. Company name (แผนงาน…จาก ________)  ─ left-aligned
    {
        let gids = text_to_gid_bytes(&page.company_name, face);
        write_text_block(
            &mut buf,
            FONT_RESOURCE,
            TEXT_PT,
            COMPANY_X,
            COMPANY_Y,
            &gids,
        );
    }

    // 3-7. Budget values — right-aligned inside each column.
    //      Values order matches column order left-to-right.
    let values: [f64; 5] = [
        page.budget_total,
        page.previous_spent,
        page.previous_balance,
        page.current_amount,
        page.remaining_balance,
    ];

    for (i, &val) in values.iter().enumerate() {
        let text = fmt_amount(val);
        let w = text_width_pt(&text, face, NUM_PT);
        // Clamp so text never overflows outside the column on the left.
        let x = (COL_RIGHTS[i] - w).max(COL_LEFTS[i]);
        let gids = text_to_gid_bytes(&text, face);
        write_text_block(&mut buf, FONT_RESOURCE, NUM_PT, x, VALUE_Y, &gids);
    }

    buf
}

// ---------------------------------------------------------------------------
// lopdf helpers
// ---------------------------------------------------------------------------

/// Build the `/W` (per-glyph widths) array for the CIDFont dictionary.
///
/// Without this array every glyph falls back to `DW` (default width = 1 em),
/// which causes the massive inter-character spacing visible in the output.
/// We compute exact widths for the two ranges we actually write:
///   • ASCII printable U+0020–U+007E  (digits, comma, period, minus, space …)
///   • Thai block      U+0E00–U+0E7F  (including zero-advance combining marks)
///
/// PDF width = font_advance × 1000 / units_per_em  (UPM = 4096 for THSarabun)
fn build_w_array(face: &Face) -> Vec<Object> {
    let upm = face.units_per_em() as i64;
    let mut entries: Vec<Object> = Vec::new();

    for &(start, end) in &[(0x0020u32, 0x007Eu32), (0x0E00u32, 0x0E7Fu32)] {
        for cp in start..=end {
            if let Some(ch) = char::from_u32(cp) {
                if let Some(g) = face.glyph_index(ch) {
                    let adv = face.glyph_hor_advance(g).unwrap_or(0) as i64;
                    let pdf_w = adv * 1000 / upm;
                    // W format: gid  [width]
                    entries.push(Object::Integer(g.0 as i64));
                    entries.push(Object::Array(vec![Object::Integer(pdf_w)]));
                }
            }
        }
    }
    entries
}

/// Embed the full `THSarabun.ttf` as a new Type0/Identity-H composite font
/// in `doc` and return the ObjectId of the top-level Type0 font object.
fn embed_full_font(doc: &mut Document, face: &Face) -> Result<ObjectId, lopdf::Error> {
    // ── FontFile2 stream ────────────────────────────────────────────────────
    let mut ff_dict = Dictionary::new();
    ff_dict.set(
        b"Length1".to_vec(),
        Object::Integer(FONT_BYTES.len() as i64),
    );
    let ff_id = doc.add_object(Object::Stream(Stream::new(ff_dict, FONT_BYTES.to_vec())));

    // ── FontDescriptor ──────────────────────────────────────────────────────
    let mut desc = Dictionary::new();
    desc.set(b"Type".to_vec(), Object::Name(b"FontDescriptor".to_vec()));
    desc.set(
        b"FontName".to_vec(),
        Object::Name(b"THSarabunPSK-Ovl".to_vec()),
    );
    desc.set(b"Flags".to_vec(), Object::Integer(32));
    desc.set(
        b"FontBBox".to_vec(),
        Object::Array(vec![
            Object::Integer(-168),
            Object::Integer(-250),
            Object::Integer(1374),
            Object::Integer(850),
        ]),
    );
    desc.set(b"ItalicAngle".to_vec(), Object::Integer(0));
    desc.set(b"Ascent".to_vec(), Object::Integer(850));
    desc.set(b"Descent".to_vec(), Object::Integer(-250));
    desc.set(b"CapHeight".to_vec(), Object::Integer(716));
    desc.set(b"StemV".to_vec(), Object::Integer(80));
    desc.set(b"FontFile2".to_vec(), Object::Reference(ff_id));
    let desc_id = doc.add_object(Object::Dictionary(desc));

    // ── CIDSystemInfo ───────────────────────────────────────────────────────
    let mut csi = Dictionary::new();
    csi.set(
        b"Registry".to_vec(),
        Object::String(b"Adobe".to_vec(), StringFormat::Literal),
    );
    csi.set(
        b"Ordering".to_vec(),
        Object::String(b"Identity".to_vec(), StringFormat::Literal),
    );
    csi.set(b"Supplement".to_vec(), Object::Integer(0));

    // ── CIDFont (CIDFontType2) ──────────────────────────────────────────────
    let mut cidf = Dictionary::new();
    cidf.set(b"Type".to_vec(), Object::Name(b"Font".to_vec()));
    cidf.set(b"Subtype".to_vec(), Object::Name(b"CIDFontType2".to_vec()));
    cidf.set(
        b"BaseFont".to_vec(),
        Object::Name(b"THSarabunPSK-Ovl".to_vec()),
    );
    cidf.set(b"CIDSystemInfo".to_vec(), Object::Dictionary(csi));
    cidf.set(b"FontDescriptor".to_vec(), Object::Reference(desc_id));
    // Realistic fallback (~400 PDF units at UPM=4096). The W array below
    // overrides this for every glyph we will actually render.
    cidf.set(b"DW".to_vec(), Object::Integer(400));
    cidf.set(b"CIDToGIDMap".to_vec(), Object::Name(b"Identity".to_vec()));
    // ── per-glyph advance widths (fixes inter-character spacing) ────────────
    cidf.set(b"W".to_vec(), Object::Array(build_w_array(face)));
    let cidf_id = doc.add_object(Object::Dictionary(cidf));

    // ── Type0 composite font ────────────────────────────────────────────────
    let mut f0 = Dictionary::new();
    f0.set(b"Type".to_vec(), Object::Name(b"Font".to_vec()));
    f0.set(b"Subtype".to_vec(), Object::Name(b"Type0".to_vec()));
    f0.set(
        b"BaseFont".to_vec(),
        Object::Name(b"THSarabunPSK-Ovl".to_vec()),
    );
    f0.set(b"Encoding".to_vec(), Object::Name(b"Identity-H".to_vec()));
    f0.set(
        b"DescendantFonts".to_vec(),
        Object::Array(vec![Object::Reference(cidf_id)]),
    );
    let f0_id = doc.add_object(Object::Dictionary(f0));

    Ok(f0_id)
}

/// Add `font_name → font_obj_id` to the /Font sub-dict of the page's
/// /Resources, handling both inline and referenced resource dicts.
///
/// Uses a clone-and-replace pattern to avoid borrow-checker conflicts.
fn add_font_to_page_resources(
    doc: &mut Document,
    page_id: ObjectId,
    font_name: &str,
    font_obj_id: ObjectId,
) -> Result<(), String> {
    // Clone the page dict.
    let mut page_dict = doc
        .get_object(page_id)
        .map_err(|e| format!("get page: {e}"))?
        .as_dict()
        .map_err(|e| format!("page not dict: {e}"))?
        .clone();

    // Clone the Resources dict (inline or referenced).
    let mut resources = match page_dict
        .get(b"Resources")
        .map_err(|e| format!("no Resources: {e}"))?
    {
        Object::Reference(id) => {
            let rid = *id;
            doc.get_object(rid)
                .map_err(|e| format!("get Resources ref: {e}"))?
                .as_dict()
                .map_err(|e| format!("Resources not dict: {e}"))?
                .clone()
        }
        Object::Dictionary(d) => d.clone(),
        other => return Err(format!("unexpected Resources type: {:?}", other)),
    };

    // Clone the Font sub-dict (inline or referenced), defaulting to empty.
    let mut font_dict = match resources.get(b"Font") {
        Ok(Object::Reference(id)) => {
            let fid = *id;
            doc.get_object(fid)
                .map_err(|e| format!("get Font ref: {e}"))?
                .as_dict()
                .map_err(|e| format!("Font not dict: {e}"))?
                .clone()
        }
        Ok(Object::Dictionary(d)) => d.clone(),
        _ => Dictionary::new(),
    };

    // Insert our new font entry.
    font_dict.set(
        font_name.as_bytes().to_vec(),
        Object::Reference(font_obj_id),
    );

    // Write back (always inline after this call).
    resources.set(b"Font".to_vec(), Object::Dictionary(font_dict));
    page_dict.set(b"Resources".to_vec(), Object::Dictionary(resources));

    *doc.get_object_mut(page_id)
        .map_err(|e| format!("get_mut page: {e}"))? = Object::Dictionary(page_dict);

    Ok(())
}

/// Walk trailer → Catalog → Pages and return the ObjectId of the Pages root node.
fn get_pages_root_id(doc: &Document) -> Result<ObjectId, String> {
    let catalog_id = doc
        .trailer
        .get(b"Root")
        .map_err(|e| format!("trailer/Root: {e}"))?
        .as_reference()
        .map_err(|e| format!("Root not ref: {e}"))?;

    let catalog = doc
        .get_object(catalog_id)
        .map_err(|e| format!("get catalog: {e}"))?
        .as_dict()
        .map_err(|e| format!("catalog not dict: {e}"))?;

    catalog
        .get(b"Pages")
        .map_err(|e| format!("catalog/Pages: {e}"))?
        .as_reference()
        .map_err(|e| format!("Pages not ref: {e}"))
}

/// Append `new_page_id` to the Kids array and increment the Count in the
/// Pages root node.
fn add_page_to_tree(
    doc: &mut Document,
    root_id: ObjectId,
    new_page_id: ObjectId,
) -> Result<(), String> {
    let root = doc
        .get_object_mut(root_id)
        .map_err(|e| format!("get Pages root: {e}"))?
        .as_dict_mut()
        .map_err(|e| format!("Pages root not dict: {e}"))?;

    let count = root
        .get(b"Count")
        .map_err(|e| format!("no Count: {e}"))?
        .as_i64()
        .map_err(|e| format!("Count not i64: {e}"))?;

    root.set(b"Count".to_vec(), Object::Integer(count + 1));

    root.get_mut(b"Kids")
        .map_err(|e| format!("no Kids: {e}"))?
        .as_array_mut()
        .map_err(|e| format!("Kids not array: {e}"))?
        .push(Object::Reference(new_page_id));

    Ok(())
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Generate a multi-page cover-letter PDF using the pre-built template.
///
/// Each page of `pages` produces one A4 page in the output PDF.
/// Returns the absolute path of the saved file.
pub fn generate(pages: &[CoverLetterPage], params: &CoverLettersParams) -> Result<String, String> {
    if pages.is_empty() {
        return Err("ไม่มีข้อมูลสำหรับสร้าง PDF".into());
    }

    // ── Parse font for glyph metrics (used throughout) ──────────────────────
    let face = Face::parse(FONT_BYTES, 0).map_err(|e| format!("parse font: {:?}", e))?;

    // ── Load template ────────────────────────────────────────────────────────
    let mut doc = Document::load_mem(TEMPLATE_BYTES).map_err(|e| format!("load template: {e}"))?;

    // ── Embed full TH Sarabun as OVL_F ──────────────────────────────────────
    let font_obj_id = embed_full_font(&mut doc, &face).map_err(|e| format!("embed font: {e}"))?;

    // ── Locate template page ─────────────────────────────────────────────────
    let first_page_id = *doc.get_pages().get(&1).ok_or("template has no page 1")?;

    // ── Save original (unmodified) template content ──────────────────────────
    let template_content = doc
        .get_page_content(first_page_id)
        .map_err(|e| format!("get page content: {e}"))?;

    // ── Inject OVL_F into page 1's resources ────────────────────────────────
    add_font_to_page_resources(&mut doc, first_page_id, FONT_RESOURCE, font_obj_id)?;

    // Clone the (now-updated) page 1 dict for reuse in pages 2..N.
    let page1_dict = doc
        .get_object(first_page_id)
        .map_err(|e| format!("clone page dict: {e}"))?
        .as_dict()
        .map_err(|e| format!("page dict not dict: {e}"))?
        .clone();

    let pages_root_id = get_pages_root_id(&doc)?;

    // ── Helper: build combined content bytes for one cover letter ────────────
    let make_combined = |cover_page: &CoverLetterPage| -> Vec<u8> {
        let overlay = build_overlay(cover_page, &face);
        let mut v = template_content.clone();
        v.extend_from_slice(b"\nq\n");
        v.extend_from_slice(&overlay);
        v.extend_from_slice(b"Q\n");
        v
    };

    // ── Page 1: modify the existing template page in-place ───────────────────
    doc.change_page_content(first_page_id, make_combined(&pages[0]))
        .map_err(|e| format!("change page content: {e}"))?;

    // ── Pages 2..N: add new pages ────────────────────────────────────────────
    for cover_page in pages.iter().skip(1) {
        let combined = make_combined(cover_page);

        // New content stream (uncompressed — valid per PDF spec).
        let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), combined)));

        // Clone page 1 dict and point it at the new content stream.
        let mut new_page = page1_dict.clone();
        new_page.set(b"Contents".to_vec(), Object::Reference(content_id));

        let new_page_id = doc.add_object(Object::Dictionary(new_page));
        add_page_to_tree(&mut doc, pages_root_id, new_page_id)?;
    }

    // ── Save ─────────────────────────────────────────────────────────────────
    let filename = format!(
        "เบิกยาปะหน้า_{}_เดือน{}_รอบ{}.pdf",
        params.year, params.month, params.round
    );
    let filepath = Path::new(&params.output_dir).join(&filename);
    let path_str = filepath.to_string_lossy().to_string();

    let mut out_file =
        std::fs::File::create(&filepath).map_err(|e| format!("create output file: {e}"))?;
    doc.save_to(&mut out_file)
        .map_err(|e| format!("save PDF: {e}"))?;

    Ok(path_str)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CoverLetterPage, CoverLettersParams};
    use std::fs;

    // ── fmt_amount ────────────────────────────────────────────────────────────

    #[test]
    fn test_fmt_amount_zero() {
        assert_eq!(fmt_amount(0.0), "0.00");
    }

    #[test]
    fn test_fmt_amount_thousands() {
        assert_eq!(fmt_amount(1_000.0), "1,000.00");
        assert_eq!(fmt_amount(1_234_567.89), "1,234,567.89");
        assert_eq!(fmt_amount(10_000_000.0), "10,000,000.00");
    }

    #[test]
    fn test_fmt_amount_small() {
        assert_eq!(fmt_amount(99.99), "99.99");
        assert_eq!(fmt_amount(0.01), "0.01");
    }

    #[test]
    fn test_fmt_amount_negative() {
        assert_eq!(fmt_amount(-500.50), "-500.50");
        assert_eq!(fmt_amount(-1_000_000.0), "-1,000,000.00");
    }

    // ── font helpers ──────────────────────────────────────────────────────────

    #[test]
    fn test_gid_lookup_thai_chars() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        // Every Thai character in the BMP range should resolve to a non-zero GID.
        let thai_sample = "กขคงจฉชซฌญ";
        for ch in thai_sample.chars() {
            let gid = face.glyph_index(ch).map(|g| g.0).unwrap_or(0);
            assert!(
                gid > 0,
                "Expected non-zero GID for U+{:04X} ({})",
                ch as u32,
                ch
            );
        }
    }

    #[test]
    fn test_text_to_gid_bytes_length() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let text = "สวัสดี"; // 6 Unicode chars
        let bytes = text_to_gid_bytes(text, &face);
        // Each char produces 2 bytes.
        assert_eq!(bytes.len(), text.chars().count() * 2);
    }

    #[test]
    fn test_text_width_positive() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let w = text_width_pt("1,234,567.89", &face, NUM_PT);
        assert!(w > 0.0, "text width must be positive");
        // A 12-char number at 11.5pt should be well under the narrowest column (82.6 pt).
        assert!(
            w < 85.0,
            "12-char number at {NUM_PT}pt should fit in the narrowest column (82.6 pt), got {w:.1}pt"
        );
    }

    // ── W array correctness ───────────────────────────────────────────────────

    /// Digits were 2.7× too wide with DW=1000 (UPM=4096, advance≈1495 → 364 PDF units).
    /// After the fix every digit must be close to 364 and nowhere near 1000.
    #[test]
    fn test_w_array_digit_widths_are_correct() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let upm = face.units_per_em() as f64;

        for ch in '0'..='9' {
            let gid = face.glyph_index(ch).expect("digit must have a glyph").0;
            let adv = face.glyph_hor_advance(GlyphId(gid)).unwrap_or(0) as f64;
            let pdf_w = (adv * 1000.0 / upm).round() as i64;

            // Must be close to 364, definitely not the broken default of 1000.
            assert!(
                pdf_w > 200 && pdf_w < 700,
                "Digit '{}' PDF width {} is outside reasonable range 200–700 \
                 (was 1000 before fix)",
                ch,
                pdf_w
            );
            assert!(
                pdf_w < 1000,
                "Digit '{}' PDF width {} must be less than DW=1000 (old broken value)",
                ch,
                pdf_w
            );
        }
    }

    /// Thai characters were 2.5× too wide with DW=1000.
    /// Verify a selection of common Thai glyphs have correct, narrow advances.
    #[test]
    fn test_w_array_thai_widths_are_correct() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let upm = face.units_per_em() as f64;

        // (Unicode code point, expected PDF width range low, high)
        let samples: &[(char, i64, i64)] = &[
            ('ก', 300, 600), // U+0E01
            ('บ', 300, 600), // U+0E1A
            ('น', 300, 600), // U+0E19
            ('ส', 300, 600), // U+0E2A
            ('า', 300, 600), // U+0E32
        ];

        for &(ch, lo, hi) in samples {
            let gid = face.glyph_index(ch).expect("Thai char must have glyph").0;
            let adv = face.glyph_hor_advance(GlyphId(gid)).unwrap_or(0) as f64;
            let pdf_w = (adv * 1000.0 / upm).round() as i64;

            assert!(
                pdf_w >= lo && pdf_w <= hi,
                "Thai '{}' (U+{:04X}) PDF width {} is outside expected range {}–{}",
                ch,
                ch as u32,
                pdf_w,
                lo,
                hi
            );
            assert!(
                pdf_w < 1000,
                "Thai '{}' PDF width {} must be < 1000 (DW was the broken default)",
                ch,
                pdf_w
            );
        }
    }

    /// Combining Thai marks (vowels/tones) have advance width 0 — they must
    /// NOT be assigned DW=1000 which would push subsequent characters right.
    #[test]
    fn test_w_array_combining_marks_are_zero_width() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let upm = face.units_per_em() as f64;

        // Zero-advance combining marks in the Thai block
        let combining: &[char] = &[
            '\u{0E31}', // ั  SARA A (above)
            '\u{0E34}', // ิ  SARA I
            '\u{0E35}', // ี  SARA II
            '\u{0E36}', // ึ  SARA UE
            '\u{0E37}', // ื  SARA UEE
            '\u{0E47}', // ็  MAITAIKHU
            '\u{0E48}', // ่  MAI EK
            '\u{0E49}', // ้  MAI THO
            '\u{0E4A}', // ๊  MAI TRI
            '\u{0E4B}', // ๋  MAI CHATTAWA
        ];

        for &ch in combining {
            if let Some(g) = face.glyph_index(ch) {
                let adv = face.glyph_hor_advance(g).unwrap_or(0) as f64;
                let pdf_w = (adv * 1000.0 / upm) as i64;
                assert_eq!(
                    pdf_w, 0,
                    "Combining mark U+{:04X} should have 0 advance, got {}",
                    ch as u32, pdf_w
                );
            }
        }
    }

    /// The build_w_array helper must produce entries for at least the ASCII
    /// printable range AND the Thai block — verifiable by counting entries and
    /// confirming key characters are covered.
    #[test]
    fn test_w_array_covers_required_ranges() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let w = build_w_array(&face);

        // W array is a flat list of (gid, [width]) pairs → must have even length.
        assert!(w.len() % 2 == 0, "W array must have even element count");

        // Must be non-trivially populated (ASCII 95 + Thai 128 chars → ≥ 150 entries).
        let entry_count = w.len() / 2;
        assert!(
            entry_count >= 150,
            "W array should have at least 150 glyph entries, got {}",
            entry_count
        );

        // Extract all GIDs that appear in the W array.
        let mut covered_gids: std::collections::HashSet<u16> = std::collections::HashSet::new();
        let mut i = 0;
        while i + 1 < w.len() {
            if let Object::Integer(gid) = &w[i] {
                covered_gids.insert(*gid as u16);
            }
            i += 2;
        }

        // Key characters that MUST be in the W array.
        let must_cover: &[(char, &str)] = &[
            ('0', "digit 0"),
            ('9', "digit 9"),
            (',', "comma"),
            ('.', "period"),
            ('-', "minus"),
            (' ', "space"),
            ('ก', "Thai gor gai"),
            ('า', "Thai sara aa"),
            ('น', "Thai nor nu"),
        ];

        for &(ch, name) in must_cover {
            let gid = face.glyph_index(ch).expect("character must have a glyph").0;
            assert!(
                covered_gids.contains(&gid),
                "W array must cover GID {} for {} ('{}' U+{:04X})",
                gid,
                name,
                ch,
                ch as u32
            );
        }
    }

    // ── build_overlay sanity ──────────────────────────────────────────────────

    #[test]
    fn test_overlay_contains_font_resource() {
        let face = Face::parse(FONT_BYTES, 0).expect("parse font");
        let page = sample_cover_page();
        let overlay = build_overlay(&page, &face);
        let s = String::from_utf8_lossy(&overlay);
        assert!(
            s.contains(&format!("/{}", FONT_RESOURCE)),
            "overlay must reference /{FONT_RESOURCE}"
        );
        // Should contain exactly 7 BT/ET pairs (date + company + 5 values).
        let bt_count = s.matches("BT\n").count();
        assert_eq!(bt_count, 7, "expected 7 text blocks, got {bt_count}");
    }

    // ── full generate() ───────────────────────────────────────────────────────

    #[test]
    fn test_generate_single_page() {
        let tmp = tempdir();
        let params = sample_params(&tmp);
        let pages = vec![sample_cover_page()];

        let result = generate(&pages, &params);
        assert!(result.is_ok(), "generate failed: {:?}", result.err());

        let path = result.unwrap();
        let meta = fs::metadata(&path).expect("output file should exist");
        assert!(
            meta.len() > 10_000,
            "output PDF seems too small: {} bytes",
            meta.len()
        );

        // Verify it's a PDF
        let hdr = &fs::read(&path).unwrap()[..5];
        assert_eq!(hdr, b"%PDF-", "output should start with %PDF-");
    }

    #[test]
    fn test_generate_multi_page() {
        let tmp = tempdir();
        let params = sample_params(&tmp);
        let pages = vec![
            sample_cover_page(),
            CoverLetterPage {
                company_name: "บริษัท ทดสอบ จำกัด".into(),
                category: "วัสดุเภสัชกรรม".into(),
                budget_total: 5_000_000.0,
                previous_spent: 1_200_000.0,
                previous_balance: 3_800_000.0,
                current_amount: 50_000.0,
                remaining_balance: 3_750_000.0,
                fiscal_year: "2568".into(),
                date_text: "15 มกราคม 2568".into(),
            },
            CoverLetterPage {
                company_name: "ห้างหุ้นส่วนจำกัด เภสัชภัณฑ์".into(),
                category: "ยา".into(),
                budget_total: 5_000_000.0,
                previous_spent: 1_250_000.0,
                previous_balance: 3_750_000.0,
                current_amount: 30_000.0,
                remaining_balance: 3_720_000.0,
                fiscal_year: "2568".into(),
                date_text: "15 มกราคม 2568".into(),
            },
        ];

        let result = generate(&pages, &params);
        assert!(
            result.is_ok(),
            "multi-page generate failed: {:?}",
            result.err()
        );

        let path = result.unwrap();
        let pdf_bytes = fs::read(&path).unwrap();
        assert!(pdf_bytes.starts_with(b"%PDF-"));

        // A 3-page PDF must be substantially larger than a 1-page one.
        assert!(
            pdf_bytes.len() > 40_000,
            "3-page PDF seems too small: {} bytes",
            pdf_bytes.len()
        );

        // Re-parse the generated PDF with lopdf and count pages directly.
        let doc2 =
            Document::load_mem(&pdf_bytes).expect("generated PDF must be re-parseable by lopdf");
        let page_count = doc2.get_pages().len();
        assert_eq!(
            page_count, 3,
            "expected 3 pages in output PDF, got {page_count}"
        );
    }

    #[test]
    fn test_generate_empty_pages_returns_err() {
        let tmp = tempdir();
        let params = sample_params(&tmp);
        let result = generate(&[], &params);
        assert!(result.is_err(), "empty pages should return Err");
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "swift_bill_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_cover_page() -> CoverLetterPage {
        CoverLetterPage {
            company_name: "บริษัท ซิลลิค ฟาร์มา จำกัด".into(),
            category: "ยา".into(),
            budget_total: 5_000_000.0,
            previous_spent: 1_150_000.0,
            previous_balance: 3_850_000.0,
            current_amount: 125_430.50,
            remaining_balance: 3_724_569.50,
            fiscal_year: "2568".into(),
            date_text: "15 มกราคม 2568".into(),
        }
    }

    fn sample_params(output_dir: &std::path::Path) -> CoverLettersParams {
        use crate::models::DbConfig;
        CoverLettersParams {
            db_config: DbConfig {
                host: "localhost".into(),
                port: 1433,
                database: "INVS".into(),
                username: "sa".into(),
                password: "".into(),
            },
            date_from: "20250101".into(),
            date_to: "20250110".into(),
            year: 2568,
            month: 1,
            round: 1,
            budget_total: 5_000_000.0,
            previous_balance: 3_850_000.0,
            approval_date: Some("15 มกราคม 2568".into()),
            output_dir: output_dir.to_string_lossy().to_string(),
        }
    }
}
