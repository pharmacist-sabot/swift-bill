//! PDF generation for all three report types.
//!
//! Uses `printpdf 0.9` with an embedded Thai TrueType font (CordiaNew/THSarabun).
//! The font bytes are compiled into the binary via `include_bytes!`.
//!
//! # Output layout
//! - **ส่งหนี้เบิกยา** – one A4 landscape PDF
//! - **สรุปรับยา**     – one A4 landscape PDF
//! - **เบิกยาปะหน้า** – one A4 portrait PDF *per invoice*

// The invoice-submission and receiving-summary PDF paths are superseded by
// Excel export; suppress dead_code lints for the still-present helpers.
// PDF rendering helpers legitimately require many parameters.
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use printpdf::{
  Color, FontId, Line, LinePoint, Mm, Op, PaintMode, ParsedFont, PdfDocument, PdfPage,
  PdfSaveOptions, PdfWarnMsg, Point, Polygon, PolygonRing, Pt, Rgb, WindingOrder,
};
use std::fs;
use std::path::Path;

use crate::models::{
  CoverLetterPage, CoverLettersParams, InvoiceSubmissionParams, InvoiceSubmissionRow,
  ReceivingSummaryParams, ReceivingSummaryRow,
};

// Embedded fonts (compiled into the binary)
const FONT_REGULAR: &[u8] = include_bytes!("THSarabun.ttf");
const FONT_BOLD: &[u8] = include_bytes!("THSarabunBold.ttf");

// Page dimensions in mm
const A4_LAND_W: f64 = 297.0;
const A4_LAND_H: f64 = 210.0;
const A4_PORT_W: f64 = 210.0;
const A4_PORT_H: f64 = 297.0;

// Margin (mm)
const MARGIN: f64 = 15.0;

// f32 versions used directly by printpdf Mm/Pt structs
const A4_LAND_W_F: f32 = 297.0;
const A4_LAND_H_F: f32 = 210.0;
const A4_PORT_W_F: f32 = 210.0;
const A4_PORT_H_F: f32 = 297.0;

// Unit helpers
#[inline]
fn mm(v: f64) -> Pt {
  Mm(v as f32).into()
}

#[inline]
fn pt_f(v: f64) -> Pt {
  Pt(v as f32)
}

// Number formatter: thousands-separated, 2 decimal places
fn fmt_money(v: f64) -> String {
  let s = format!("{:.2}", v);
  let (int_s, dec_s) = s.split_once('.').unwrap_or((&s, "00"));
  let negative = int_s.starts_with('-');
  let digits = if negative { &int_s[1..] } else { int_s };
  let mut rev = String::new();
  for (i, ch) in digits.chars().rev().enumerate() {
    if i > 0 && i % 3 == 0 {
      rev.push(',');
    }
    rev.push(ch);
  }
  let formatted: String = rev.chars().rev().collect();
  if negative {
    format!("-{}.{}", formatted, dec_s)
  } else {
    format!("{}.{}", formatted, dec_s)
  }
}

// Thai month name
fn thai_month(m: u32) -> &'static str {
  match m {
    1 => "มกราคม",
    2 => "กุมภาพันธ์",
    3 => "มีนาคม",
    4 => "เมษายน",
    5 => "พฤษภาคม",
    6 => "มิถุนายน",
    7 => "กรกฎาคม",
    8 => "สิงหาคม",
    9 => "กันยายน",
    10 => "ตุลาคม",
    11 => "พฤศจิกายน",
    12 => "ธันวาคม",
    _ => "ไม่ทราบ",
  }
}

// Context that carries fonts + page dimensions for building Op streams
struct PageCtx {
  #[allow(dead_code)]
  font_id: FontId,
  #[allow(dead_code)]
  font_bold_id: FontId,
  #[allow(dead_code)]
  page_w: f64, // mm
  page_h: f64, // mm
}

impl PageCtx {
  /// Convert a top-down y coordinate (distance from top) to PDF bottom-up pt.
  fn y(&self, top_y_mm: f64) -> Pt {
    mm(self.page_h - top_y_mm)
  }
}

// Op-stream builder helpers

/// Append ops to place a text string at (x_mm, top_y_mm).
fn op_text(
  ops: &mut Vec<Op>,
  ctx: &PageCtx,
  font_id: &FontId,
  size: f64,
  x_mm: f64,
  top_y_mm: f64,
  s: &str,
) {
  ops.push(Op::StartTextSection);
  ops.push(Op::SetFont {
    font: printpdf::PdfFontHandle::External(font_id.clone()),
    size: pt_f(size),
  });
  ops.push(Op::SetTextCursor {
    pos: Point {
      x: mm(x_mm),
      y: ctx.y(top_y_mm),
    },
  });
  ops.push(Op::ShowText {
    items: vec![printpdf::TextItem::Text(s.to_string())],
  });
  ops.push(Op::EndTextSection);
}

/// Rough character-width estimator (mm) for centering/right-align.
/// Thai + ASCII mixed: we use a unified estimate.
fn char_width_mm(size: f64) -> f64 {
  // 1 pt ≈ 0.353 mm; Thai glyphs in Cordia/Sarabun are roughly 0.55× em wide.
  size * 0.353 * 0.55
}

fn text_width_mm(s: &str, size: f64) -> f64 {
  s.chars().count() as f64 * char_width_mm(size)
}

/// Place text centred within [col_x .. col_x+col_w].
fn op_text_center(
  ops: &mut Vec<Op>,
  ctx: &PageCtx,
  font_id: &FontId,
  size: f64,
  col_x: f64,
  col_w: f64,
  top_y: f64,
  s: &str,
) {
  let tw = text_width_mm(s, size);
  let x = col_x + (col_w - tw) / 2.0;
  op_text(ops, ctx, font_id, size, x.max(col_x + 0.5), top_y, s);
}

/// Place text right-aligned within [col_x .. col_x+col_w], with padding_right (mm) from right edge.
fn op_text_right(
  ops: &mut Vec<Op>,
  ctx: &PageCtx,
  font_id: &FontId,
  size: f64,
  col_x: f64,
  col_w: f64,
  top_y: f64,
  padding_right: f64,
  s: &str,
) {
  let tw = text_width_mm(s, size);
  let x = col_x + col_w - tw - padding_right;
  op_text(ops, ctx, font_id, size, x.max(col_x + 0.5), top_y, s);
}

/// Draw a horizontal line.
fn op_hline(ops: &mut Vec<Op>, ctx: &PageCtx, x1: f64, x2: f64, top_y: f64) {
  let y = ctx.y(top_y);
  ops.push(Op::DrawLine {
    line: Line {
      points: vec![
        LinePoint {
          p: Point { x: mm(x1), y },
          bezier: false,
        },
        LinePoint {
          p: Point { x: mm(x2), y },
          bezier: false,
        },
      ],
      is_closed: false,
    },
  });
}

/// Draw a vertical line.
fn op_vline(ops: &mut Vec<Op>, ctx: &PageCtx, x: f64, top_y1: f64, top_y2: f64) {
  ops.push(Op::DrawLine {
    line: Line {
      points: vec![
        LinePoint {
          p: Point {
            x: mm(x),
            y: ctx.y(top_y1),
          },
          bezier: false,
        },
        LinePoint {
          p: Point {
            x: mm(x),
            y: ctx.y(top_y2),
          },
          bezier: false,
        },
      ],
      is_closed: false,
    },
  });
}

/// Draw a filled rectangle. top_y is distance from page top.
fn op_filled_rect(
  ops: &mut Vec<Op>,
  ctx: &PageCtx,
  x: f64,
  top_y: f64,
  w: f64,
  h: f64,
  r: f64,
  g: f64,
  b: f64,
) {
  ops.push(Op::SetFillColor {
    col: Color::Rgb(Rgb {
      r: r as f32,
      g: g as f32,
      b: b as f32,
      icc_profile: None,
    }),
  });
  let y_bottom = ctx.y(top_y + h); // bottom-left in PDF coords
  ops.push(Op::DrawPolygon {
    polygon: Polygon {
      rings: vec![PolygonRing {
        points: vec![
          LinePoint {
            p: Point {
              x: mm(x),
              y: y_bottom,
            },
            bezier: false,
          },
          LinePoint {
            p: Point {
              x: mm(x + w),
              y: y_bottom,
            },
            bezier: false,
          },
          LinePoint {
            p: Point {
              x: mm(x + w),
              y: ctx.y(top_y),
            },
            bezier: false,
          },
          LinePoint {
            p: Point {
              x: mm(x),
              y: ctx.y(top_y),
            },
            bezier: false,
          },
        ],
      }],
      mode: PaintMode::Fill,
      winding_order: WindingOrder::NonZero,
    },
  });
  // Reset to black fill
  ops.push(Op::SetFillColor {
    col: Color::Rgb(Rgb {
      r: 0.0_f32,
      g: 0.0_f32,
      b: 0.0_f32,
      icc_profile: None,
    }),
  });
}

/// Draw a stroked (outline only) rectangle.
fn op_box_rect(ops: &mut Vec<Op>, ctx: &PageCtx, x: f64, top_y: f64, w: f64, h: f64) {
  let y_bottom = ctx.y(top_y + h);
  ops.push(Op::DrawPolygon {
    polygon: Polygon {
      rings: vec![PolygonRing {
        points: vec![
          LinePoint {
            p: Point {
              x: mm(x),
              y: y_bottom,
            },
            bezier: false,
          },
          LinePoint {
            p: Point {
              x: mm(x + w),
              y: y_bottom,
            },
            bezier: false,
          },
          LinePoint {
            p: Point {
              x: mm(x + w),
              y: ctx.y(top_y),
            },
            bezier: false,
          },
          LinePoint {
            p: Point {
              x: mm(x),
              y: ctx.y(top_y),
            },
            bezier: false,
          },
        ],
      }],
      mode: PaintMode::Stroke,
      winding_order: WindingOrder::NonZero,
    },
  });
}

// Font loading helpers
fn load_fonts(doc: &mut PdfDocument) -> Result<(FontId, FontId), String> {
  let font_reg = ParsedFont::from_bytes(FONT_REGULAR, 0, &mut Vec::new())
    .ok_or_else(|| "Failed to parse regular font".to_string())?;
  let font_bld = ParsedFont::from_bytes(FONT_BOLD, 0, &mut Vec::new())
    .ok_or_else(|| "Failed to parse bold font".to_string())?;
  let id_reg = doc.add_font(&font_reg);
  let id_bld = doc.add_font(&font_bld);
  Ok((id_reg, id_bld))
}

// Save helper
fn save_pdf(doc: &PdfDocument, path: &str) -> Result<(), String> {
  let mut warnings: Vec<PdfWarnMsg> = Vec::new();
  let bytes = doc.save(&PdfSaveOptions::default(), &mut warnings);
  fs::write(path, &bytes).map_err(|e| format!("Cannot write PDF to {path}: {e}"))
}

// FILE 1 – ส่งหนี้เบิกยา  (Invoice Submission List)  – A4 Landscape

pub fn generate_invoice_submission_pdf(
  rows: &[InvoiceSubmissionRow],
  params: &InvoiceSubmissionParams,
) -> Result<String, String> {
  let year = params.year;
  let month = params.month;
  let month_name = thai_month(month);
  let title = "ส่งรายการหนี้สินและเอกสารเบิกเงิน";
  let subtitle = format!(
    "เดือน{} รอบ {}/ ปีงบประมาณ {}",
    month_name, params.round, year
  );

  const ROWS_PER_PAGE: usize = 7;

  let mut doc = PdfDocument::new(title);
  let (font_id, font_bold_id) = load_fonts(&mut doc)?;

  let ctx = PageCtx {
    font_id: font_id.clone(),
    font_bold_id: font_bold_id.clone(),
    page_w: A4_LAND_W,
    page_h: A4_LAND_H,
  };

  // Column X positions (mm from left)
  //  0:ลำดับ 1:วันที่รับ 2:เลขที่เอกสาร 3:reg_no 4:running 5:วันที่ใบส่งของ
  //  6:รหัสบริษัท 7:ค่าใช้จ่ายเรื่อง 8:จำนวนเงินรวม  9:end
  let col_x: &[f64] = &[
    MARGIN,
    MARGIN + 12.0,
    MARGIN + 34.0,
    MARGIN + 62.0,
    MARGIN + 80.0,
    MARGIN + 92.0,
    MARGIN + 116.0,
    MARGIN + 178.0,
    MARGIN + 212.0,
    A4_LAND_W - MARGIN,
  ];
  let cw: Vec<f64> = col_x.windows(2).map(|w| w[1] - w[0]).collect();

  let table_top = MARGIN + 19.0;
  let hdr_h = 16.0;
  let row_h = 12.5;

  // Pre-compute grand total across ALL rows for the last-page totals row
  let grand_total: f64 = rows.iter().map(|r| r.total_amount).sum();

  let chunks: Vec<&[InvoiceSubmissionRow]> = rows.chunks(ROWS_PER_PAGE).collect();
  let total_pages = chunks.len().max(1);

  let mut pdf_pages: Vec<PdfPage> = Vec::new();

  for (page_idx, chunk) in chunks.iter().enumerate() {
    let is_last = page_idx + 1 == total_pages;
    let mut ops: Vec<Op> = Vec::new();

    ops.push(Op::SetOutlineThickness { pt: pt_f(0.3) });

    // Title
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      20.0,
      0.0,
      A4_LAND_W,
      MARGIN + 6.0,
      title,
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      18.0,
      0.0,
      A4_LAND_W,
      MARGIN + 13.0,
      &subtitle,
    );

    // Page number footer for multi-page documents
    if total_pages > 1 {
      op_text_right(
        &mut ops,
        &ctx,
        &font_id,
        12.0,
        0.0,
        A4_LAND_W - MARGIN,
        MARGIN + 4.0,
        3.0,
        &format!("หน้า {}/{}", page_idx + 1, total_pages),
      );
    }

    // Header background
    op_filled_rect(
      &mut ops,
      &ctx,
      col_x[0],
      table_top,
      col_x[9] - col_x[0],
      hdr_h,
      0.82,
      0.88,
      0.96,
    );

    // Header labels
    // Single-row headers: visually centred in hdr_h
    let hdr_y_single = table_top + hdr_h * 0.58; // ~9.3 mm below table_top
                                                 // Two-row headers: upper line and lower sub-label
    let hdr_y1 = table_top + hdr_h * 0.34; // ~5.4 mm — top row baseline
    let hdr_y2 = table_top + hdr_h * 0.76; // ~12.2 mm — bottom row baseline

    // Single-row column headers
    let single_hdrs: &[(&str, usize)] = &[
      ("ลำดับ", 0),
      ("วันที่รับของ", 1),
      ("เลขที่เอกสาร", 2),
      ("ชื่อบริษัท", 6),
      ("ค่าใช้จ่ายเรื่อง", 7),
      ("จำนวนเงินรวม", 8),
    ];
    for &(lbl, ci) in single_hdrs {
      op_text_center(
        &mut ops,
        &ctx,
        &font_bold_id,
        13.0,
        col_x[ci],
        cw[ci],
        hdr_y_single,
        lbl,
      );
    }

    // Two-row span: เลขทะเบียนคุม (cols 3–4)
    let span_w34 = cw[3] + cw[4];
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      13.0,
      col_x[3],
      span_w34,
      hdr_y1,
      "เลขทะเบียนคุม",
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.0,
      col_x[3],
      cw[3],
      hdr_y2,
      "เลขทะเบียน",
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.0,
      col_x[4],
      cw[4],
      hdr_y2,
      "ลำดับ",
    );

    // Two-row: วัน/เดือน/ปีใบส่งของ (col 5)
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      13.0,
      col_x[5],
      cw[5],
      hdr_y1,
      "วัน/เดือน/ปี",
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.0,
      col_x[5],
      cw[5],
      hdr_y2,
      "ใบส่งของ",
    );

    // Data rows
    let mut cur_y = table_top + hdr_h;

    for row in *chunk {
      let ty = cur_y + row_h - 1.8;
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        14.5,
        col_x[0],
        cw[0],
        ty,
        &row.seq.to_string(),
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        14.0,
        col_x[1],
        cw[1],
        ty,
        &row.receive_date,
      );
      op_text(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[2] + 1.0,
        ty,
        &row.invoice_no,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        14.0,
        col_x[3],
        cw[3],
        ty,
        &row.reg_no,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        14.0,
        col_x[4],
        cw[4],
        ty,
        &row.running_in_reg.to_string(),
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        14.0,
        col_x[5],
        cw[5],
        ty,
        &row.invoice_date,
      );
      op_text(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[6] + 1.0,
        ty,
        &row.company_name,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[7],
        cw[7],
        ty,
        &row.category,
      );
      op_text_right(
        &mut ops,
        &ctx,
        &font_id,
        14.5,
        col_x[8],
        cw[8],
        ty,
        2.0,
        &fmt_money(row.total_amount),
      );
      op_hline(&mut ops, &ctx, col_x[0], col_x[9], cur_y + row_h);
      cur_y += row_h;
    }

    // Last page: totals row + signature block
    if is_last {
      op_filled_rect(
        &mut ops,
        &ctx,
        col_x[0],
        cur_y,
        col_x[9] - col_x[0],
        row_h,
        0.93,
        0.93,
        0.93,
      );
      let ty = cur_y + row_h - 1.8;
      op_text_center(
        &mut ops,
        &ctx,
        &font_bold_id,
        15.0,
        col_x[0],
        col_x[8] - col_x[0],
        ty,
        "รวมทั้งสิ้น",
      );
      op_text_right(
        &mut ops,
        &ctx,
        &font_bold_id,
        15.0,
        col_x[8],
        cw[8],
        ty,
        2.0,
        &fmt_money(grand_total),
      );
      op_hline(&mut ops, &ctx, col_x[0], col_x[9], cur_y + row_h);
      cur_y += row_h;

      // Signature block
      let sig_y = cur_y + 12.0;
      op_text_center(&mut ops, &ctx, &font_id, 16.0, MARGIN, 60.0, sig_y, "ผู้จัดทำ");
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        A4_LAND_W - MARGIN - 65.0,
        65.0,
        sig_y,
        "ผู้ตรวจสอบ",
      );
      let sig2 = sig_y + 8.0;
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        MARGIN,
        60.0,
        sig2,
        "ลงชื่อ................................",
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        A4_LAND_W - MARGIN - 65.0,
        65.0,
        sig2,
        "ลงชื่อ................................",
      );
      let sig3 = sig2 + 7.0;
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        MARGIN,
        60.0,
        sig3,
        "(............................................)",
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        A4_LAND_W - MARGIN - 65.0,
        65.0,
        sig3,
        "(............................................)",
      );
      let sig4 = sig3 + 6.0;
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        MARGIN,
        60.0,
        sig4,
        "เจ้าหน้าที่พัสดุ",
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        16.0,
        A4_LAND_W - MARGIN - 65.0,
        65.0,
        sig4,
        "หัวหน้ากลุ่มงานเภสัชกรรมฯ",
      );
    }

    // Table box + vertical lines
    op_box_rect(
      &mut ops,
      &ctx,
      col_x[0],
      table_top,
      col_x[9] - col_x[0],
      cur_y - table_top,
    );
    for &cx in &col_x[1..9usize] {
      op_vline(&mut ops, &ctx, cx, table_top, cur_y);
    }
    op_hline(&mut ops, &ctx, col_x[0], col_x[9], table_top + hdr_h);

    pdf_pages.push(PdfPage::new(Mm(A4_LAND_W_F), Mm(A4_LAND_H_F), ops));
  }

  // Handle empty rows case
  if pdf_pages.is_empty() {
    let mut ops = Vec::new();
    ops.push(Op::SetOutlineThickness { pt: pt_f(0.3) });
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      20.0,
      0.0,
      A4_LAND_W,
      MARGIN + 6.0,
      title,
    );
    pdf_pages.push(PdfPage::new(Mm(A4_LAND_W_F), Mm(A4_LAND_H_F), ops));
  }

  doc.with_pages(pdf_pages);

  let filename = format!("ส่งหนี้เบิกยา_{}_เดือน{}_รอบ{}.pdf", year, month, params.round);
  let filepath = Path::new(&params.output_dir).join(&filename);
  let path_str = filepath.to_string_lossy().to_string();
  save_pdf(&doc, &path_str)?;
  Ok(path_str)
}

// FILE 2 – สรุปรับยา  (Receiving Summary)  – A4 Landscape

pub fn generate_receiving_summary_pdf(
  rows: &[ReceivingSummaryRow],
  params: &ReceivingSummaryParams,
) -> Result<String, String> {
  let year = params.year;
  let month = params.month;
  let month_name = thai_month(month);
  let title = format!(
    "สรุปยอดยาเดือน{} รอบ {} ปีงบประมาณ {}",
    month_name, params.round, year
  );

  const ROWS_PER_PAGE: usize = 10;

  let mut doc = PdfDocument::new(&title);
  let (font_id, font_bold_id) = load_fonts(&mut doc)?;

  let ctx = PageCtx {
    font_id: font_id.clone(),
    font_bold_id: font_bold_id.clone(),
    page_w: A4_LAND_W,
    page_h: A4_LAND_H,
  };

  // Column definitions (12 data columns)
  // วันที่ขออนุมัติ | วันที่สั่งซื้อ | วันที่รับของ | รหัสบริษัท |
  // จำนวนเงินรวม | รหัสลงรับยา | เลขทะเบียน | running |
  // เลขที่ลงรับ | ขอซื้อ | รายงาน/อนุมัติ | ใบสั่งซื้อ | end
  let col_x: &[f64] = &[
    MARGIN,
    MARGIN + 24.0,
    MARGIN + 48.0,
    MARGIN + 72.0,
    MARGIN + 88.0,
    MARGIN + 114.0,
    MARGIN + 130.0,
    MARGIN + 148.0,
    MARGIN + 160.0,
    MARGIN + 186.0,
    MARGIN + 206.0,
    MARGIN + 228.0,
    A4_LAND_W - MARGIN,
  ];
  let cw: Vec<f64> = col_x.windows(2).map(|w| w[1] - w[0]).collect();

  let table_top = MARGIN + 15.0;
  let hdr_h = 18.0;
  let row_h = 12.5;

  // Pre-compute grand total across ALL rows for the last-page totals row
  let grand_total: f64 = rows.iter().map(|r| r.total_amount).sum();

  let chunks: Vec<&[ReceivingSummaryRow]> = rows.chunks(ROWS_PER_PAGE).collect();
  let total_pages = chunks.len().max(1);

  let mut pdf_pages: Vec<PdfPage> = Vec::new();

  for (page_idx, chunk) in chunks.iter().enumerate() {
    let is_last = page_idx + 1 == total_pages;
    let mut ops: Vec<Op> = Vec::new();

    ops.push(Op::SetOutlineThickness { pt: pt_f(0.3) });

    // Title
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      19.0,
      0.0,
      A4_LAND_W,
      MARGIN + 6.0,
      &title,
    );

    // Page number footer for multi-page documents
    if total_pages > 1 {
      op_text_right(
        &mut ops,
        &ctx,
        &font_id,
        12.0,
        0.0,
        A4_LAND_W - MARGIN,
        MARGIN + 4.0,
        3.0,
        &format!("หน้า {}/{}", page_idx + 1, total_pages),
      );
    }

    // Header background
    op_filled_rect(
      &mut ops,
      &ctx,
      col_x[0],
      table_top,
      col_x[12] - col_x[0],
      hdr_h,
      0.82,
      0.88,
      0.96,
    );

    // Single-row headers: vertically centred
    let hdr_y_single = table_top + hdr_h * 0.58;
    // Two-row headers
    let hdr_y1 = table_top + hdr_h * 0.32; // upper sub-row baseline
    let hdr_y2 = table_top + hdr_h * 0.72; // lower sub-row baseline

    // Single-row column headers
    let single_hdrs2: &[(&str, usize)] = &[
      ("วันที่ขออนุมัติ", 0),
      ("วันที่สั่งซื้อ", 1),
      ("วันที่รับของ", 2),
      ("รหัสบริษัท", 3),
      ("จำนวนเงินรวม", 4),
      ("รหัสลงรับยา", 5),
      ("เลขที่ลงรับ", 8),
    ];
    for &(lbl, ci) in single_hdrs2 {
      op_text_center(
        &mut ops,
        &ctx,
        &font_bold_id,
        12.5,
        col_x[ci],
        cw[ci],
        hdr_y_single,
        lbl,
      );
    }

    // Two-row span: เลขทะเบียนคุม (cols 6–7)
    let span_w67 = col_x[8] - col_x[6];
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.5,
      col_x[6],
      span_w67,
      hdr_y1,
      "เลขทะเบียนคุม",
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.0,
      col_x[6],
      cw[6],
      hdr_y2,
      "เลขทะเบียน",
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.0,
      col_x[7],
      cw[7],
      hdr_y2,
      "ลำดับ",
    );

    // Two-row: ขอซื้อ / รายงาน / ใบสั่งซื้อ with sub-labels
    let two_row_cols: &[(&str, &str, usize)] = &[
      ("ขอซื้อ", "ลบ0033.302/", 9),
      ("รายงาน/อนุมัติ", "ลบ0033.302/", 10),
    ];
    for &(top_lbl, bot_lbl, ci) in two_row_cols {
      op_text_center(
        &mut ops,
        &ctx,
        &font_bold_id,
        12.5,
        col_x[ci],
        cw[ci],
        hdr_y1,
        top_lbl,
      );
      op_text_center(
        &mut ops, &ctx, &font_id, 11.5, col_x[ci], cw[ci], hdr_y2, bot_lbl,
      );
    }
    let year_str = format!("ใบสั่งซื้อ…/{}", year);
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      12.5,
      col_x[11],
      cw[11],
      hdr_y1,
      "ใบสั่งซื้อ",
    );
    op_text_center(
      &mut ops,
      &ctx,
      &font_id,
      11.5,
      col_x[11],
      cw[11],
      hdr_y2,
      &format!("…/{}", year),
    );
    let _ = year_str; // suppress unused-variable warning

    // Data rows
    let mut cur_y = table_top + hdr_h;

    for row in *chunk {
      let ty = cur_y + row_h - 1.8;
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[0],
        cw[0],
        ty,
        &row.approval_date,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[1],
        cw[1],
        ty,
        &row.po_date,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[2],
        cw[2],
        ty,
        &row.receive_date,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[3],
        cw[3],
        ty,
        &row.company_code,
      );
      op_text_right(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[4],
        cw[4],
        ty,
        1.5,
        &fmt_money(row.total_amount),
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[5],
        cw[5],
        ty,
        &row.receiving_code.to_string(),
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[6],
        cw[6],
        ty,
        &row.reg_no,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[7],
        cw[7],
        ty,
        &row.running_in_reg.to_string(),
      );
      op_text(
        &mut ops,
        &ctx,
        &font_id,
        13.0,
        col_x[8] + 1.0,
        ty,
        &row.invoice_no,
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[9],
        cw[9],
        ty,
        &row.request_no.to_string(),
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[10],
        cw[10],
        ty,
        &row.report_no.to_string(),
      );
      op_text_center(
        &mut ops,
        &ctx,
        &font_id,
        13.5,
        col_x[11],
        cw[11],
        ty,
        &row.po_no.to_string(),
      );
      op_hline(&mut ops, &ctx, col_x[0], col_x[12], cur_y + row_h);
      cur_y += row_h;
    }

    // Last page: totals row
    if is_last {
      op_filled_rect(
        &mut ops,
        &ctx,
        col_x[0],
        cur_y,
        col_x[12] - col_x[0],
        row_h,
        0.93,
        0.93,
        0.93,
      );
      let ty = cur_y + row_h - 1.8;
      op_text_center(
        &mut ops,
        &ctx,
        &font_bold_id,
        15.0,
        col_x[0],
        col_x[4] - col_x[0],
        ty,
        "รวมทั้งสิ้น",
      );
      op_text_right(
        &mut ops,
        &ctx,
        &font_bold_id,
        15.0,
        col_x[4],
        cw[4],
        ty,
        1.5,
        &fmt_money(grand_total),
      );
      op_hline(&mut ops, &ctx, col_x[0], col_x[12], cur_y + row_h);
      cur_y += row_h;
    }

    // Table borders + verticals
    op_box_rect(
      &mut ops,
      &ctx,
      col_x[0],
      table_top,
      col_x[12] - col_x[0],
      cur_y - table_top,
    );
    for &cx in &col_x[1..12usize] {
      op_vline(&mut ops, &ctx, cx, table_top, cur_y);
    }
    op_hline(&mut ops, &ctx, col_x[0], col_x[12], table_top + hdr_h);

    pdf_pages.push(PdfPage::new(Mm(A4_LAND_W_F), Mm(A4_LAND_H_F), ops));
  }

  // Handle empty rows case
  if pdf_pages.is_empty() {
    let mut ops = Vec::new();
    ops.push(Op::SetOutlineThickness { pt: pt_f(0.3) });
    op_text_center(
      &mut ops,
      &ctx,
      &font_bold_id,
      19.0,
      0.0,
      A4_LAND_W,
      MARGIN + 6.0,
      &title,
    );
    pdf_pages.push(PdfPage::new(Mm(A4_LAND_W_F), Mm(A4_LAND_H_F), ops));
  }

  doc.with_pages(pdf_pages);

  let filename = format!("สรุปรับยา_{}_เดือน{}_รอบ{}.pdf", year, month, params.round);
  let filepath = Path::new(&params.output_dir).join(&filename);
  let path_str = filepath.to_string_lossy().to_string();
  save_pdf(&doc, &path_str)?;
  Ok(path_str)
}

// FILE 3 – เบิกยาปะหน้า  (Cover Letters)  – A4 Portrait, single PDF for all invoices

pub fn generate_cover_letters_pdf(
  pages: &[CoverLetterPage],
  params: &CoverLettersParams,
) -> Result<String, String> {
  generate_combined_cover(pages, params)
}

// Helper to generate a single cover letter page (used by combined PDF)
// Returns the Ops for one complete cover letter

fn build_cover_letter_ops(
  page: &CoverLetterPage,
  _params: &CoverLettersParams,
  font_id: &FontId,
  font_bold_id: &FontId,
) -> Vec<Op> {
  let ctx = PageCtx {
    font_id: font_id.clone(),
    font_bold_id: font_bold_id.clone(),
    page_w: A4_PORT_W,
    page_h: A4_PORT_H,
  };

  let mut ops: Vec<Op> = Vec::new();
  ops.push(Op::SetOutlineThickness { pt: pt_f(0.3) });

  let lm = 20.0_f64; // left margin
  let rm = A4_PORT_W - 15.0; // right edge
  let pw = A4_PORT_W;

  // Font size 16 for main content
  let title_size = 16.0;
  let body_size = 16.0;

  // บันทึกข้อความ
  let mut y = MARGIN + 8.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_bold_id,
    title_size,
    0.0,
    pw,
    y,
    "บันทึกข้อความ",
  );

  // ส่วนราชการ
  y += 12.0;
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm,
    y,
    "ส่วนราชการ.....กลุ่มงาน/ฝ่าย..กลุ่มงานเภสัชกรรมและคุ้มครองผู้บริโภค",
  );
  y += 8.0;
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm,
    y,
    "โรงพยาบาลสระโบสถ์....อำเภอสระโบสถ์.....จังหวัดลพบุรี",
  );

  // ที่ / วันที่
  y += 9.0;
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm,
    y,
    "ที่     ลบ 0033/พิเศษ",
  );
  op_text(&mut ops, &ctx, font_id, body_size, lm + 80.0, y, "วันที่");
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 93.0,
    y,
    &page.date_text,
  );
  // underline for date
  op_hline(&mut ops, &ctx, lm + 93.0, rm, y + 2.0);

  // เรื่อง
  y += 9.0;
  op_text(&mut ops, &ctx, font_bold_id, body_size, lm, y, "เรื่อง");
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 15.0,
    y,
    "ขออนุมัติเบิกเงิน ตามโครงการ/กิจกรรม พัฒนาระบบบริหารคลัง/พัสดุ/ครุภัณฑ์",
  );

  // เรียน
  y += 9.0;
  op_text(&mut ops, &ctx, font_bold_id, body_size, lm, y, "เรียน");
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 15.0,
    y,
    "ปลัดกระทรวงสาธารณสุข",
  );

  // ด้วย
  y += 10.0;
  op_text(&mut ops, &ctx, font_id, body_size, lm + 10.0, y, "ด้วย");
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 24.0,
    y,
    "กลุ่มงานเภสัชกรรมและคุ้มครองผู้บริโภค",
  );
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 112.0,
    y,
    "มีความประสงค์จะขอเบิกเงิน",
  );

  // ค่า category
  y += 9.0;
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm,
    y,
    &format!("ค่า.....{}.....", page.category),
  );
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 58.0,
    y,
    &format!("โดยขอใช้งบประมาณประจำปี..........{}...", page.fiscal_year),
  );

  // แผนงาน / company
  y += 9.0;
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 10.0,
    y,
    "แผนงาน/โครงการ",
  );
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 49.0,
    y,
    &format!("ซื้อยาจาก{}............", page.company_name),
  );

  // Budget table
  y += 12.0;
  let tbl_x = lm;
  let tbl_w = rm - lm;
  let tbl_col_w = tbl_w / 5.0;
  let tbl_h = 10.0;

  op_filled_rect(&mut ops, &ctx, tbl_x, y, tbl_w, tbl_h, 0.82, 0.88, 0.96);

  let budget_hdrs = [
    "ยอดเงินจัดสรร",
    "ยอดเบิกจ่ายแล้ว",
    "ยอดคงเหลือ",
    "เบิกจ่ายครั้งนี้",
    "ยอดเงินคงเหลือ",
  ];
  for (i, lbl) in budget_hdrs.iter().enumerate() {
    let cx = tbl_x + i as f64 * tbl_col_w;
    op_text_center(
      &mut ops,
      &ctx,
      font_bold_id,
      13.0,
      cx,
      tbl_col_w,
      y + tbl_h * 0.62,
      lbl,
    );
  }

  let val_y = y + tbl_h;
  let budget_vals = [
    fmt_money(page.budget_total),
    fmt_money(page.previous_spent),
    fmt_money(page.previous_balance),
    fmt_money(page.current_amount),
    fmt_money(page.remaining_balance),
  ];
  for (i, val) in budget_vals.iter().enumerate() {
    let cx = tbl_x + i as f64 * tbl_col_w;
    op_text_right(
      &mut ops,
      &ctx,
      font_bold_id,
      14.0,
      cx,
      tbl_col_w,
      val_y + tbl_h - 2.5,
      3.0,
      val,
    );
  }

  // Table border + column separators + mid-line
  op_box_rect(&mut ops, &ctx, tbl_x, y, tbl_w, tbl_h * 2.0);
  op_hline(&mut ops, &ctx, tbl_x, tbl_x + tbl_w, y + tbl_h);
  for i in 1..5usize {
    op_vline(
      &mut ops,
      &ctx,
      tbl_x + i as f64 * tbl_col_w,
      y,
      y + tbl_h * 2.0,
    );
  }

  // จึงเรียน
  y += tbl_h * 2.0 + 12.0;
  op_text(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    lm + 10.0,
    y,
    "จึงเรียนมาเพื่อโปรดพิจารณาอนุมัติ",
  );

  // Signature blocks
  let sig_x = lm + 90.0;
  let sig_w = rm - sig_x;

  y += 20.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "ลงชื่อ..............................ผู้ขออนุมัติ",
  );
  y += 9.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "(.......................................)",
  );
  y += 8.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "เจ้าหน้าที่พัสดุ",
  );

  y += 16.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "ลงชื่อ..............................ผู้เห็นชอบ",
  );
  y += 9.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "(.......................................)",
  );
  y += 8.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "หัวหน้ากลุ่มงานเภสัชกรรมฯ",
  );

  y += 16.0;
  op_text(&mut ops, &ctx, font_bold_id, body_size, lm, y, "อนุมัติ");

  y += 16.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "ลงชื่อ..............................ผู้อนุมัติ",
  );
  y += 9.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "(.......................................)",
  );
  y += 8.0;
  op_text_center(
    &mut ops,
    &ctx,
    font_id,
    body_size,
    sig_x,
    sig_w,
    y,
    "ผู้อำนวยการโรงพยาบาลสระโบสถ์",
  );

  ops
}

fn generate_combined_cover(
  pages: &[CoverLetterPage],
  params: &CoverLettersParams,
) -> Result<String, String> {
  let mut doc = PdfDocument::new(&format!("เบิกยาปะหน้า - รอบ {}", params.round));
  let (font_id, font_bold_id) = load_fonts(&mut doc)?;

  let mut pdf_pages: Vec<PdfPage> = Vec::new();

  for page in pages.iter() {
    let ops = build_cover_letter_ops(page, params, &font_id, &font_bold_id);
    let page_obj = PdfPage::new(Mm(A4_PORT_W_F), Mm(A4_PORT_H_F), ops);
    pdf_pages.push(page_obj);
  }

  doc.with_pages(pdf_pages);

  let filename = format!(
    "เบิกยาปะหน้า_{}_เดือน{}_รอบ{}.pdf",
    params.year, params.month, params.round
  );
  let filepath = Path::new(&params.output_dir).join(&filename);
  let path_str = filepath.to_string_lossy().to_string();
  save_pdf(&doc, &path_str)?;
  Ok(path_str)
}
