use core::cell::RefCell;
use std::rc::Rc;
use crate::win::*;
use crate::fonts::FontFamilyId;
use crate::ctx::*;
use crate::unicode::*;


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextFormat {
    pub font:          FontFamilyId,
    pub font_size:     f32,
    pub font_weight:   u32,
    pub italic:        bool,
    pub underline:     bool,
    pub strikethrough: bool,
}

impl Default for TextFormat {
    fn default() -> Self {
        TextFormat {
            font:          FontFamilyId::DEFAULT,
            font_size:     16.0,
            font_weight:   400,
            italic:        false,
            underline:     false,
            strikethrough: false,
        }
    }
}


#[allow(dead_code)]
#[derive(Debug, Default)]
struct TextSpan {
    text_begin_utf8: u32,
    text_end_utf8:   u32,

    object_index: u32, // u32::MAX for None

    // TODO: "bidi_level" instead.
    is_rtl: bool,
    script: DWRITE_SCRIPT_ANALYSIS,

    width: f32,
    ascent: f32,
    drop:   f32,

    format: TextFormat,
    font_face: Option<IDWriteFontFace>,

    // utf8 offset (relative to text_begin_utf8)
    // to index of first glyph in glyph cluster.
    cluster_map: Vec<u16>,

    glyph_indices:  Vec<u16>,
    glyph_props:    Vec<DWRITE_SHAPING_GLYPH_PROPERTIES>,
    glyph_advances: Vec<f32>,
    glyph_offsets:  Vec<DWRITE_GLYPH_OFFSET>,
}

#[derive(Debug)]
struct VisualSpan {
    text_begin_utf8: u32,
    text_end_utf8:   u32,

    span_index:  u32,
    glyph_begin: u32,
    glyph_end:   u32,

    width: f32,
}

#[allow(dead_code)]
#[derive(Debug)]
struct VisualLine {
    text_begin_utf8: u32,
    text_end_utf8:   u32,

    spans: Vec<VisualSpan>,

    width:    f32,
    height:   f32,
    baseline: f32,
}


#[allow(dead_code)] // TEMP
#[derive(Debug)]
struct Object {
    text_pos: u32,
    index:    u32,
    pos:      [f32; 2], // TODO: compute.
    size:     [f32; 2],
    baseline: f32,
}


#[derive(Clone, Copy, Debug)]
pub struct LayoutParams {
    pub width:  f32,
    pub height: f32,
    pub wrap: bool,
}

impl Default for LayoutParams {
    fn default() -> Self {
        LayoutParams {
            width:  f32::INFINITY,
            height: f32::INFINITY,
            wrap: false,
        }
    }
}


#[allow(dead_code)] // TEMP
pub struct TextLayout {
    ctx: Ctx,
    text: Vec<u8>,
    objects: Vec<Object>,
    spans: Vec<TextSpan>,
    hard_lines: Vec<u32>, // end indices in spans array.
    lines: Vec<VisualLine>,
    break_options: Vec<u32>, // bit vector.
    layout_params: LayoutParams,
}

impl TextLayout {
    pub fn text(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.text) }
    }

    pub fn set_layout_width(&mut self, w: f32) {
        if !w.is_nan() && w != self.layout_params.width {
            self.layout_params.width = w;
        }
    }

    pub fn set_layout_height(&mut self, h: f32) {
        if !h.is_nan() && h != self.layout_params.height {
            self.layout_params.height = h;
        }
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn layout_params(&self) -> LayoutParams {
        self.layout_params
    }


    pub fn layout(&mut self) {
        let max_width = self.layout_params.width;

        // update object spans.
        for span in &mut self.spans {
            if span.object_index != u32::MAX {
                let object = &self.objects[span.object_index as usize];
                span.width  = object.size[0];
                span.ascent = object.baseline;
                span.drop   = object.size[1] - object.baseline;
            }
        }


        let mut lines = vec![];

        let mut hard_lines_span_cursor = 0;
        for spans_end in &self.hard_lines {
            let spans_begin = hard_lines_span_cursor;
            let spans_end   = *spans_end as usize;
            hard_lines_span_cursor = spans_end;
            assert!(spans_begin < spans_end);

            let text_begin_utf8 = self.spans[spans_begin].text_begin_utf8;
            let text_end_utf8   = self.spans[spans_end - 1].text_end_utf8;

            let mut lb = LineBreaker {
                breaks: BreakIter {
                    at: text_begin_utf8,
                    end: text_end_utf8,
                },
                prev_break: 0,
                text_begin: text_begin_utf8,
                span_begin: spans_begin,
                cluster_begin: 0,
                segment: BreakSegment {
                    text_cursor: text_begin_utf8,
                    span_cursor: spans_begin,
                    cluster_cursor: 0,
                    line_width: 0.0, span_width: 0.0, width: 0.0,
                },
            };

            while let Some(seg) = lb.next_segment(self) {
                if seg.line_width > max_width {
                    lb.start_new_line(self, seg, &mut lines);
                }
                else {
                    lb.add_to_line(seg);
                }
            }
            lb.finalize(self, &mut lines);
        }

        self.lines = lines;
    }
}


#[derive(Clone, Copy, Debug)]
pub struct PosMetrics {
    pub x: f32,
    pub y: f32,
    pub line_height: f32,
    pub line_index:  usize,
}

impl TextLayout {
    // TODO: support RTL.
    // TODO: return info for grapheme ligature subdivision.
    pub fn offset_to_pos(&self, offset: usize) -> PosMetrics {
        let offset = offset.min(self.text.len()) as u32;

        let mut y = 0.0;

        for (line_index, line) in self.lines.iter().enumerate() {
            // end inclusive (that's the \n).
            if offset >= line.text_begin_utf8 && offset <= line.text_end_utf8 {
                let mut x = 0.0;

                for vspan in &line.spans {
                    let tspan = &self.spans[vspan.span_index as usize];

                    if offset >= vspan.text_begin_utf8
                    && offset <  vspan.text_end_utf8 {
                        let local_offset = offset - tspan.text_begin_utf8;

                        if tspan.object_index == u32::MAX {
                            let glyph_begin = vspan.glyph_begin as usize;
                            let glyph = tspan.cluster_map[local_offset as usize] as usize;
                            for i in glyph_begin..glyph {
                                x += tspan.glyph_advances[i];
                            }
                        }

                        return PosMetrics {
                            x, y,
                            line_height: line.height,
                            line_index,
                        };
                    }

                    x += vspan.width;
                }

                return PosMetrics {
                    x, y,
                    line_height: line.height,
                    line_index,
                };
            }

            y += line.height;
        }

        assert_eq!(self.text.len(), 0);
        return PosMetrics { x: 0.0, y: 0.0, line_height: 0.0, line_index: 0 };
    }
}

impl TextLayout {
    pub fn draw(&self, pos: [f32; 2], rt: &ID2D1RenderTarget, brush: &ID2D1Brush) {
        let mut cursor = pos[1];
        for line in &self.lines {
            // TEMP
            let rect = D2D_RECT_F {
                left:   pos[0],
                top:    cursor,
                right:  pos[0] + line.width,
                bottom: cursor + line.height,
            };
            unsafe { rt.DrawRectangle(&rect, brush, 1.0, None) };

            let mut x = pos[0];
            for vspan in &line.spans {
                let tspan = &self.spans[vspan.span_index as usize];

                let y = cursor + line.baseline;

                if tspan.object_index != u32::MAX {
                    let object = &self.objects[tspan.object_index as usize];

                    let y = y - object.baseline;
                    let rect = D2D_RECT_F {
                        left:   x,
                        top:    y,
                        right:  x + object.size[0],
                        bottom: y + object.size[1],
                    };

                    // TEMP.
                    unsafe { rt.FillRectangle(&rect, brush) };

                    x += vspan.width;
                    continue;
                }

                // empty line.
                if vspan.text_begin_utf8 == vspan.text_end_utf8 {
                    continue;
                }

                let rtl_offset = if tspan.is_rtl { vspan.width } else { 0.0 };

                let face = tspan.font_face.as_ref().unwrap();

                let glyph_count = vspan.glyph_end - vspan.glyph_begin;
                let glyph_indices  = &tspan.glyph_indices[vspan.glyph_begin as usize];
                let glyph_advances = &tspan.glyph_advances[vspan.glyph_begin as usize];
                let glyph_offsets  = &tspan.glyph_offsets[vspan.glyph_begin as usize];
                let run = DWRITE_GLYPH_RUN {
                    fontFace: Some(face.clone()),
                    fontEmSize: tspan.format.font_size,
                    glyphCount: glyph_count,
                    glyphIndices: glyph_indices,
                    glyphAdvances: glyph_advances,
                    glyphOffsets: glyph_offsets,
                    isSideways: false.into(),
                    bidiLevel: tspan.is_rtl as u32,
                };

                let pos = D2D_POINT_2F {
                    x: x + rtl_offset,
                    y,
                };
                unsafe { rt.DrawGlyphRun(pos, &run, brush, Default::default()) };


                if tspan.format.underline || tspan.format.strikethrough {
                    let mut metrics = Default::default();
                    unsafe { face.GetMetrics(&mut metrics) };

                    let scale = tspan.format.font_size / metrics.designUnitsPerEm as f32;

                    // TODO: should these be pixel aligned?

                    if tspan.format.underline {
                        let offset = scale * metrics.underlinePosition as f32;
                        let height = scale * metrics.underlineThickness as f32;

                        let y = y - offset;
                        let rect = D2D_RECT_F {
                            left:   x,
                            top:    y - height/2.0,
                            right:  x + vspan.width,
                            bottom: y + height/2.0,
                        };
                        unsafe { rt.FillRectangle(&rect, brush) };
                    }

                    if tspan.format.strikethrough {
                        let offset = scale * metrics.strikethroughPosition as f32;
                        let height = scale * metrics.strikethroughThickness as f32;

                        let y = y - offset;
                        let rect = D2D_RECT_F {
                            left:   x,
                            top:    y - height/2.0,
                            right:  x + vspan.width,
                            bottom: y + height/2.0,
                        };
                        unsafe { rt.FillRectangle(&rect, brush) };
                    }
                }

                x += vspan.width;
            }

            cursor += line.height;
        }
    }
}


#[derive(Clone, Copy, Debug, Default)]
struct PreSpan {
    text_end_utf8: u32,
    format: TextFormat,

    object_index: u32, // u32::MAX for None.
}

pub struct TextLayoutBuilder {
    ctx: Ctx,
    text: Vec<u8>,
    objects: Vec<Object>,

    base_format: TextFormat,
    format: TextFormat,
    format_begin: u32,
    pre_spans: Vec<PreSpan>,
}

impl TextLayoutBuilder {
    pub fn new(ctx: Ctx, format: TextFormat) -> Self {
        Self {
            ctx,
            text: vec![],
            objects: vec![],
            base_format: format,
            format,
            format_begin: 0,
            pre_spans: vec![],
        }
    }

    fn flush_format_ex(&mut self, object_index: u32) {
        let format_end = self.text.len() as u32;
        if format_end != self.format_begin {
            self.pre_spans.push(PreSpan {
                text_end_utf8:   format_end,
                format:          self.format,
                object_index,
            });
            self.format_begin = format_end;
        }
    }

    #[inline]
    fn flush_format(&mut self) {
        self.flush_format_ex(u32::MAX);
    }


    #[inline]
    pub fn add_string(&mut self, string: &str) {
        self.text.extend(string.as_bytes());
    }

    #[inline]
    pub fn add_line(&mut self, line: &str) {
        self.text.extend(line.as_bytes());
        self.text.push('\n' as u8);
    }

    pub fn add_object_ex(&mut self, size: [f32; 2], baseline: f32) {
        let index = self.objects.len() as u32;
        self.objects.push(Object {
            text_pos: self.text.len() as u32,
            index, pos: [0.0; 2], size, baseline,
        });

        self.flush_format();
        // NOTE: represent as null byte.
        // seems to have decent break behavior.
        // less confusing cursor positions than
        // the multi-byte object replacement char.
        self.text.push(0x00);
        self.flush_format_ex(index);
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn add_object(&mut self) {
        self.add_object_ex([0.0; 2], 0.0);
    }


    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn base_format(&self) -> TextFormat {
        self.base_format
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn current_format(&self) -> TextFormat {
        self.format
    }

    pub fn set_format(&mut self, format: TextFormat) {
        if format != self.format {
            self.flush_format();
            self.format = format;
        }
    }

    #[inline]
    pub fn reset_format(&mut self) {
        self.set_format(self.base_format);
    }


    pub fn set_font(&mut self, font: FontFamilyId) {
        if font != self.format.font {
            self.flush_format();
            self.format.font = font;
        }
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn reset_font(&mut self) {
        self.set_font(self.base_format.font);
    }

    pub fn set_font_size(&mut self, size: f32) {
        if size != self.format.font_size {
            self.flush_format();
            self.format.font_size = size;
        }
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn reset_font_size(&mut self) {
        self.set_font_size(self.base_format.font_size);
    }

    pub fn set_font_weight(&mut self, weight: u32) {
        if weight != self.format.font_weight {
            self.flush_format();
            self.format.font_weight = weight;
        }
    }

    #[inline]
    pub fn reset_font_weight(&mut self) {
        self.set_font_weight(self.base_format.font_weight);
    }

    #[inline]
    pub fn set_bold(&mut self, bold: bool) {
        // TODO: semantic values.
        if bold { self.set_font_weight(700) }
        else    { self.reset_font_weight()  }
    }

    pub fn set_italic(&mut self, italic: bool) {
        if italic != self.format.italic {
            self.flush_format();
            self.format.italic = italic;
        }
    }

    pub fn set_underline(&mut self, underline: bool) {
        if underline != self.format.underline {
            self.flush_format();
            self.format.underline = underline;
        }
    }

    pub fn set_strikethrough(&mut self, strikethrough: bool) {
        if strikethrough != self.format.strikethrough {
            self.flush_format();
            self.format.strikethrough = strikethrough;
        }
    }


    pub fn build(mut self) -> TextLayout {unsafe{
        if self.text.len() == 0 {
            return TextLayout {
                ctx: self.ctx,
                text: vec![],
                objects: vec![],
                spans: vec![],
                hard_lines: vec![],
                lines: vec![],
                break_options: vec![],
                layout_params: Default::default(),
            };
        }
        self.flush_format();
        assert!(self.pre_spans.len() > 0);

        let TextLayoutBuilder { ctx, text, objects, pre_spans, base_format, .. } = self;
        assert!(text.len() < (u32::MAX / 2) as usize);

        let (text16, utf16_to_utf8) = {
            let mut utf16 = vec![];
            let mut map   = vec![];

            let bytes = text.as_slice();
            let mut cursor = 0;
            while let Some((cp, new_cursor)) = utf8_next_code_point(bytes, cursor) {
                let mut buffer = [0; 2];
                let is_double = utf16_encode(cp, &mut buffer);

                utf16.push(buffer[0]);
                map.push(cursor as u32);

                if is_double {
                    utf16.push(buffer[1]);
                    map.push(cursor as u32);
                }

                cursor = new_cursor;
            }
            assert_eq!(cursor, bytes.len());
            map.push(bytes.len() as u32);

            (utf16, map)
        };

        let source: IDWriteTextAnalysisSource = DwSource {
            string: text16.as_slice(),
            locale: w!("en-us").as_ptr(), // TEMP.
        }.into();

        let dw_breaks = RefCell::new(DwSinkBreaks {
            pointer: 0,
            options: vec![0; (text.len() + 31) / 32],
            lines:   vec![],
        });
        let dw_spans = RefCell::new(DwSinkSpans {
            begin: 0,
            is_rtls: vec![],
            scripts: vec![],
        });

        let sink: IDWriteTextAnalysisSink = DwSink {
            utf16_to_utf8: utf16_to_utf8.as_slice(),
            breaks: &dw_breaks,
            spans:  &dw_spans,
        }.into();


        let analyzer = ctx.dw_factory.CreateTextAnalyzer().unwrap();

        analyzer.AnalyzeLineBreakpoints(&source, 0, text16.len() as u32, &sink).unwrap();

        let mut breaks = dw_breaks.borrow_mut();
        let breaks = &mut *breaks;
        breaks.lines.push(text16.len() as u32);

        let mut pspan_index = 0;
        let mut pspan = pre_spans[0];

        let mut hard_lines = vec![];
        let mut text_spans = vec![];
        let mut cursor = 0;
        for end in breaks.lines.iter() {
            let line_begin = cursor;
            let line_end   = *end;
            let line_len   = line_end - line_begin;
            cursor = line_end + 1;

            // empty line
            if line_len == 0 {
                let text_begin_utf8 = utf16_to_utf8[line_begin as usize];
                let text_end_utf8   = text_begin_utf8;
                text_spans.push(TextSpan {
                    text_begin_utf8, text_end_utf8,
                    object_index: u32::MAX,
                    ascent: base_format.font_size, // TEMP.
                    format: base_format,
                    .. Default::default()
                });

                hard_lines.push(text_spans.len() as u32);
                continue;
            }

            // reset spans sink.
            let mut spans = dw_spans.borrow_mut();
            spans.begin = line_begin;
            spans.is_rtls.clear();
            spans.is_rtls.resize(line_len as usize, false);
            spans.scripts.clear();
            spans.scripts.resize(line_len as usize, Default::default());
            drop(spans);

            // compute spans.
            analyzer.AnalyzeBidi  (&source, line_begin, line_len, &sink).unwrap();
            analyzer.AnalyzeScript(&source, line_begin, line_len, &sink).unwrap();

            #[derive(Default)]
            struct RawSpan {
                text_begin_utf16: u32,
                text_end_utf16:   u32,

                object_index: u32,

                format: TextFormat,
                is_rtl: bool,
                script: DWRITE_SCRIPT_ANALYSIS,
            }

            let raw_spans = {
                let spans = dw_spans.borrow();

                let mut result = vec![];
                let mut span = RawSpan::default();

                for pos in line_begin..line_end {
                    let i = (pos - line_begin) as usize;

                    let new_format = {
                        let mut new_format = false;
                        let pos_utf8 = utf16_to_utf8[pos as usize];
                        while pos_utf8 >= pspan.text_end_utf8 {
                            pspan_index += 1;
                            pspan = pre_spans[pspan_index];
                            new_format = true;
                        }
                        new_format
                    };

                    let is_rtl = spans.is_rtls[i];
                    let script = spans.scripts[i];

                    if is_rtl != span.is_rtl
                    || script.script != span.script.script
                    || script.shapes != span.script.shapes
                    || new_format
                    || i == 0 {
                        if span.text_begin_utf16 != span.text_end_utf16 {
                            result.push(span);
                        }
                        span = RawSpan {
                            text_begin_utf16: pos,
                            text_end_utf16:   pos + 1,
                            object_index: pspan.object_index,
                            format: pspan.format,
                            is_rtl,
                            script,
                        };
                    }
                    else {
                        span.text_end_utf16 = pos + 1;
                    }
                }
                if span.text_begin_utf16 != span.text_end_utf16 {
                    result.push(span);
                }

                result
            };

            for raw_span in &raw_spans {
                // inline object.
                if raw_span.object_index != u32::MAX {
                    let text_begin_utf8 = utf16_to_utf8[raw_span.text_begin_utf16 as usize];
                    let text_end_utf8   = utf16_to_utf8[raw_span.text_end_utf16 as usize];

                    // add break option before & after.
                    // TODO: maybe don't add one after if next char is whitespace?
                    breaks.options[text_begin_utf8 as usize / 32] |= 1 << (text_begin_utf8 % 32);
                    breaks.options[text_end_utf8   as usize / 32] |= 1 << (text_end_utf8   % 32);

                    text_spans.push(TextSpan {
                        text_begin_utf8, text_end_utf8,
                        object_index: raw_span.object_index,
                        .. Default::default()
                    });

                    continue;
                }

                let format = raw_span.format;
                let is_rtl = raw_span.is_rtl;
                let script = raw_span.script;

                let fonts = ctx.fonts.borrow();
                let font = fonts.font_name_utf16(format.font);
                let font_weight = DWRITE_FONT_WEIGHT(format.font_weight as i32);
                let font_style =
                    if format.italic { DWRITE_FONT_STYLE_ITALIC }
                    else             { DWRITE_FONT_STYLE_NORMAL };

                let mut text_cursor = raw_span.text_begin_utf16;
                while text_cursor < raw_span.text_end_utf16 {
                    let mut mapped_len = 0;
                    let mut mapped_font = None;
                    let mut scale = 0.0; // TODO: use this?
                    ctx.dw_system_fallback.MapCharacters(
                        &source,
                        text_cursor, raw_span.text_end_utf16 - text_cursor,
                        &ctx.dw_system_fonts,
                        PCWSTR(font.as_ptr()),
                        font_weight,
                        font_style,
                        DWRITE_FONT_STRETCH_NORMAL,
                        &mut mapped_len,
                        Some(&mut mapped_font),
                        &mut scale).unwrap();
                    assert!(mapped_len > 0);

                    let cov_begin = text_cursor as usize;
                    let cov_end   = cov_begin + mapped_len as usize;
                    text_cursor += mapped_len;

                    let text_begin_utf8 = utf16_to_utf8[cov_begin];
                    let text_end_utf8   = utf16_to_utf8[cov_end];
                    let text_utf8_len = text_end_utf8 - text_begin_utf8;


                    if mapped_font.is_none() {
                        continue;
                    }
                    let font = mapped_font.unwrap();
                    let face = font.CreateFontFace().unwrap();


                    let string = &text16[cov_begin .. cov_end];

                    let mut cluster_map  = vec![0; string.len()];
                    let mut text_props = vec![Default::default(); string.len()];

                    let max_len = 3 * string.len() / 2 + 16;

                    let mut glyph_indices = vec![0; max_len];
                    let mut glyph_props   = vec![Default::default(); max_len];

                    // TODO: loop.
                    let mut glyph_count = 0;
                    analyzer.GetGlyphs(
                        PCWSTR(string.as_ptr()),
                        string.len() as u32,
                        &face,
                        false, is_rtl, &script,
                        w!("en-us"), // TEMP
                        None, None, None, 0,
                        max_len as u32,
                        cluster_map.as_mut_ptr(),
                        text_props.as_mut_ptr(),
                        glyph_indices.as_mut_ptr(),
                        glyph_props.as_mut_ptr(),
                        &mut glyph_count).unwrap();

                    glyph_indices.truncate(glyph_count as usize);
                    glyph_props.truncate(glyph_count as usize);

                    let mut glyph_advances = vec![0.0; glyph_count as usize];
                    let mut glyph_offsets  = vec![Default::default(); glyph_count as usize];

                    analyzer.GetGlyphPlacements(
                        PCWSTR(string.as_ptr()),
                        cluster_map.as_ptr(),
                        text_props.as_mut_ptr(),
                        string.len() as u32,
                        glyph_indices.as_ptr(),
                        glyph_props.as_ptr(),
                        glyph_count,
                        &face,
                        format.font_size,
                        false, is_rtl, &script,
                        w!("en-us"),
                        None, None, 0,
                        glyph_advances.as_mut_ptr(),
                        glyph_offsets.as_mut_ptr()).unwrap();


                    let mut width = 0.0;
                    for dx in &glyph_advances {
                        width += dx;
                    }

                    let mut font_metrics = Default::default();
                    face.GetMetrics(&mut font_metrics);

                    let font_scale = format.font_size / font_metrics.designUnitsPerEm as f32;
                    let ascent = font_scale * font_metrics.ascent as f32;
                    let drop   = font_scale * (font_metrics.descent as f32 + font_metrics.lineGap as f32);

                    // convert utf16 glyph map to utf8.
                    // replace 1-2 entries with 1-4 entries.
                    let cluster_map = {
                        let mut map = Vec::with_capacity(text_utf8_len as usize);

                        let mut cursor = 0;
                        while cursor < cluster_map.len() {
                            let at16 = cov_begin + cursor;
                            let at8  = utf16_to_utf8[at16] as usize;

                            let cp = utf8_next_code_point(&text, at8).unwrap_unchecked().0;

                            let entry = cluster_map[cursor];
                            for _ in 0..utf8_len(cp) {
                                map.push(entry);
                            }

                            cursor += utf16_len(cp);
                        }
                        assert_eq!(map.len(), text_utf8_len as usize);

                        map
                    };

                    text_spans.push(TextSpan {
                        text_begin_utf8, text_end_utf8,
                        object_index: u32::MAX,
                        is_rtl, script,
                        format,
                        font_face: Some(face),
                        width, ascent, drop,
                        cluster_map,
                        glyph_indices,
                        glyph_props,
                        glyph_advances,
                        glyph_offsets,
                    });
                }
            }

            hard_lines.push(text_spans.len() as u32);
        }

        let break_options = core::mem::replace(&mut breaks.options, vec![]);

        return TextLayout {
            ctx,
            text,
            objects,
            spans: text_spans,
            hard_lines,
            lines: vec![],
            break_options,
            layout_params: Default::default(),
        };
    }}
}


#[windows::core::implement(IDWriteTextAnalysisSource)]
struct DwSource {
    string: *const [u16],
    locale: *const u16,
}

impl IDWriteTextAnalysisSource_Impl for DwSource {
    fn GetLocaleName(&self, _pos: u32, _len: *mut u32, locale: *mut *mut u16) -> windows::core::Result<()> {unsafe{
        *locale = self.locale as *mut _;
        Ok(())
    }}

    fn GetNumberSubstitution(&self, _pos: u32, _len: *mut u32, subst: *mut Option<IDWriteNumberSubstitution>) -> windows::core::Result<()> {unsafe{
        *subst = None;
        Ok(())
    }}

    fn GetParagraphReadingDirection(&self) -> DWRITE_READING_DIRECTION {
        DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
    }

    fn GetTextAtPosition(&self, pos: u32, text: *mut *mut u16, len: *mut u32) -> windows::core::Result<()> {unsafe{
        let string = &*self.string;
        let sub = &string[pos as usize ..];
        *text = sub.as_ptr() as *mut _;
        *len  = sub.len() as u32;
        Ok(())
    }}

    fn GetTextBeforePosition(&self, pos: u32, text: *mut *mut u16, len: *mut u32) -> windows::core::Result<()> {unsafe{
        let string = &*self.string;
        let sub = &string[.. pos as usize];
        *text = sub.as_ptr() as *mut _;
        *len  = sub.len() as u32;
        Ok(())
    }}
}


#[windows::core::implement(IDWriteTextAnalysisSink)]
struct DwSink {
    utf16_to_utf8: *const [u32],
    breaks: *const RefCell<DwSinkBreaks>,
    spans:  *const RefCell<DwSinkSpans>,
}

struct DwSinkBreaks {
    pointer: u32,
    options: Vec<u32>,
    lines:   Vec<u32>,
}

struct DwSinkSpans {
    begin: u32,
    is_rtls: Vec<bool>,
    scripts: Vec<DWRITE_SCRIPT_ANALYSIS>,
}

impl DwSinkSpans {
    fn set_bidi(&mut self, pos: u32, len: u32, is_rtl: bool) {
        let begin = (pos - self.begin) as usize;
        self.is_rtls[begin .. begin + len as usize].fill(is_rtl);
    }

    fn set_script(&mut self, pos: u32, len: u32, script: DWRITE_SCRIPT_ANALYSIS) {
        let begin = (pos - self.begin) as usize;
        self.scripts[begin .. begin + len as usize].fill(script);
    }
}

impl IDWriteTextAnalysisSink_Impl for DwSink {
    fn SetLineBreakpoints(&self, pos: u32, len: u32, breaks: *const DWRITE_LINE_BREAKPOINT) -> windows::core::Result<()> {
        let utf16_to_utf8 = unsafe { &*self.utf16_to_utf8 };
        let mut this = unsafe { (*self.breaks).borrow_mut() };

        // ensure calls are monotonic.
        // otherwise `break_lines` won't be sorted.
        // docs don't guarantee anything.
        if pos < this.pointer {
            return Err(windows::Win32::Foundation::E_INVALIDARG.into());
        }
        this.pointer = pos + len;

        let breaks = unsafe { core::slice::from_raw_parts(breaks, len as usize) };
        for (i, brk) in breaks.iter().enumerate() {
            let bits = brk._bitfield;
            let break_before = DWRITE_BREAK_CONDITION(((bits >> 0) & 0b11) as i32);
            let break_after  = DWRITE_BREAK_CONDITION(((bits >> 2) & 0b11) as i32);

            // for hard line breaks, we want to exclude the break
            // character from the line (it shouldn't be rendered).
            // so the line "end" is the current character position.
            let is_hard = break_after == DWRITE_BREAK_CONDITION_MUST_BREAK;

            // for soft break options, we want to include the last
            // character. so the break "end" is the next character.
            let is_soft = break_before == DWRITE_BREAK_CONDITION_CAN_BREAK;

            if is_hard {
                this.lines.push(pos + i as u32);
            }
            else if is_soft {
                let at = utf16_to_utf8[pos as usize + i] as usize;
                let word = at / 32;
                let bit  = at % 32;
                this.options[word] |= 1 << bit;
            }
        }

        Ok(())
    }


    fn SetBidiLevel(&self, pos: u32, len: u32, _explicit_level: u8, resolved_level: u8) -> windows::core::Result<()> {
        let mut this = unsafe { (*self.spans).borrow_mut() };
        this.set_bidi(pos, len, (resolved_level & 1) != 0);
        Ok(())
    }

    fn SetScriptAnalysis(&self, pos: u32, len: u32, script: *const DWRITE_SCRIPT_ANALYSIS) -> windows::core::Result<()> {
        let mut this = unsafe { (*self.spans).borrow_mut() };
        this.set_script(pos, len, unsafe { *script });
        Ok(())
    }


    fn SetNumberSubstitution(&self, _pos: u32, _len: u32, _subst: &Option<IDWriteNumberSubstitution>) -> windows::core::Result<()> {
        return Err(windows::Win32::Foundation::E_NOTIMPL.into());
    }
}


struct BreakIter {
    at:  u32,
    end: u32,
}

impl BreakIter {
    fn next(&mut self, break_options: &[u32]) -> u32 {
        while self.at < self.end {
            let word = self.at / 32;
            let bit  = self.at % 32;

            let mut mask = break_options[word as usize];

            // clear bits up to (and including) at.
            mask &= !((1 << bit) - 1) << 1;

            if mask != 0 {
                let offset = mask.trailing_zeros();
                self.at = 32*word + offset;
                if self.at < self.end {
                    return self.at;
                }
                else {
                    return self.end;
                }
            }
            else {
                self.at = 32*(word + 1);
            }
        }

        return self.end;
    }
}


struct LineBreaker {
    breaks: BreakIter,

    prev_break: u32,

    text_begin:    u32,
    span_begin:    usize,
    cluster_begin: u32,

    segment: BreakSegment,
}

struct BreakSegment {
    text_cursor:    u32,
    span_cursor:    usize,
    cluster_cursor: u32,

    line_width: f32,
    span_width: f32,
    width:      f32,
}

impl LineBreaker {
    fn next_segment(&mut self, tl: &TextLayout) -> Option<BreakSegment> {
        let prev_break = self.prev_break;
        let next_break = self.breaks.next(&tl.break_options);
        if next_break == prev_break {
            return None;
        }
        self.prev_break = next_break;

        let mut line_width = self.segment.line_width;
        let mut span_width = self.segment.span_width;
        let mut seg_width  = 0.0;

        let mut text_cursor    = self.segment.text_cursor;
        let mut span_cursor    = self.segment.span_cursor;
        let mut cluster_cursor = self.segment.cluster_cursor;

        // skip full spans.
        while text_cursor < next_break {
            let span = &tl.spans[span_cursor];
            if next_break < span.text_end_utf8 {
                break;
            }

            let width = span.width - span_width;
            line_width += width;
            span_width  = 0.0;
            seg_width  += width;

            text_cursor    = span.text_end_utf8;
            span_cursor   += 1;
            cluster_cursor = 0;
        }

        // TODO: is this actually correct for inline objects?
        let span = &tl.spans[span_cursor];
        if text_cursor < next_break && span.object_index == u32::MAX {
            let break_rel = next_break - span.text_begin_utf8;
            let cluster_end = span.cluster_map[break_rel as usize] as u32;

            for i in cluster_cursor as usize .. cluster_end as usize {
                let width = span.glyph_advances[i];
                line_width += width;
                span_width += width;
                seg_width  += width;
            }

            cluster_cursor = cluster_end;
        }
        text_cursor = next_break;

        return Some(BreakSegment {
            text_cursor, span_cursor, cluster_cursor,
            line_width, span_width, width: seg_width
        });
    }

    fn add_to_line(&mut self, segment: BreakSegment) {
        self.segment = segment;
    }

    fn finish_line(&mut self, tl: &TextLayout, lines: &mut Vec<VisualLine>) {
        // TODO: empty lines?
        if self.segment.text_cursor == self.text_begin {
            return;
        }

        let text_begin  = self.text_begin;
        let text_end    = self.segment.text_cursor;
        let span_last   = self.segment.span_cursor;
        let cluster_end = self.segment.cluster_cursor;

        let mut span_cursor = self.span_begin;

        let mut spans = vec![];
        let mut max_ascent = 0.0f32;
        let mut max_drop   = 0.0f32;

        // incomplete leading span.
        if self.cluster_begin != 0 {
            let span = &tl.spans[span_cursor];

            let glyph_begin = self.cluster_begin;

            let (text_end_utf8, glyph_end);
            if span_cursor < span_last {
                text_end_utf8 = span.text_end_utf8;
                glyph_end     = span.glyph_indices.len() as u32;
            }
            else {
                text_end_utf8 = text_end;
                glyph_end     = cluster_end;
            }

            let mut width = 0.0;
            for i in glyph_begin as usize .. glyph_end as usize {
                width += span.glyph_advances[i];
            }

            spans.push(VisualSpan {
                text_begin_utf8: text_begin,
                text_end_utf8,
                span_index: span_cursor as u32,
                glyph_begin, glyph_end,
                width,
            });

            max_ascent = max_ascent.max(span.ascent);
            max_drop   = max_drop  .max(span.drop);

            span_cursor += 1;
        }

        // complete middle spans.
        while span_cursor < span_last {
            let span = &tl.spans[span_cursor];

            spans.push(VisualSpan {
                text_begin_utf8: span.text_begin_utf8,
                text_end_utf8:   span.text_end_utf8,
                span_index: span_cursor as u32,
                glyph_begin: 0,
                glyph_end: span.glyph_indices.len() as u32,
                width: span.width,
            });

            max_ascent = max_ascent.max(span.ascent);
            max_drop   = max_drop  .max(span.drop);

            span_cursor += 1;
        }

        // incomplete trailing span.
        if span_cursor == span_last && cluster_end != 0 {
            let span = &tl.spans[span_last];

            let mut width = 0.0;
            for i in 0 .. cluster_end as usize {
                width += span.glyph_advances[i];
            }

            spans.push(VisualSpan {
                text_begin_utf8: span.text_begin_utf8,
                text_end_utf8:   text_end,
                span_index: span_last as u32,
                glyph_begin: 0,
                glyph_end: cluster_end,
                width,
            });

            max_ascent = max_ascent.max(span.ascent);
            max_drop   = max_drop  .max(span.drop);
        }


        let width    = self.segment.line_width;
        let height   = max_ascent + max_drop;
        let baseline = max_ascent;

        lines.push(VisualLine {
            text_begin_utf8: text_begin,
            text_end_utf8:   text_end,
            spans,
            width, height, baseline,
        });

        self.text_begin    = text_end;
        self.span_begin    = span_last;
        self.cluster_begin = cluster_end;
    }

    fn start_new_line(&mut self, tl: &TextLayout, segment: BreakSegment, lines: &mut Vec<VisualLine>) {
        self.finish_line(tl, lines);
        self.segment = segment;
        self.segment.line_width = self.segment.width;
    }

    fn finalize(&mut self, tl: &TextLayout, lines: &mut Vec<VisualLine>) {
        self.finish_line(tl, lines);
    }
}


