use core::cell::*;
use std::rc::Rc;

use crate::win::*;
use crate::ctx::*;
use crate::common::*;
use crate::text::*;


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ElementKind {
    Div,
    Button,
    Span,
    Text,
}

impl ElementKind {
    #[inline]
    pub const fn is_container(self) -> bool {
        use ElementKind::*;
        match self {
            Div | Button => true,
            Span | Text => false,
        }
    }

    #[inline]
    pub const fn default_display(self) -> Display {
        use ElementKind::*;
        use Display::*;
        match self {
            Div     => Block,
            Button  => Inline,
            Span    => Inline,
            Text    => Inline,
        }
    }
}


pub struct Element {
    kind: ElementKind,

    ctx: Ctx,

    this:         Option<ElementRef>,
    parent:       Option<ElementRef>,
    first_child:  Option<ElementRef>,
    last_child:   Option<ElementRef>,
    next_sibling: Option<ElementRef>,
    prev_sibling: Option<ElementRef>,

    pub pos:  [f32; 2],
    size: [f32; 2],
    baseline: f32,

    style: Style,
    computed_style: Style,

    render_children: Vec<RenderElement>,

    text: String,
}


#[derive(Clone)]
pub struct ElementRef (pub Rc<RefCell<Element>>);

impl ElementRef {
    #[inline]
    pub fn borrow(&self) -> Ref<Element> {
        self.0.borrow()
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    pub fn borrow_with<R, F: FnOnce(&Element) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow())
    }

    #[inline]
    pub fn borrow_mut(&self) -> RefMut<Element> {
        self.0.borrow_mut()
    }

    #[inline]
    pub fn borrow_mut_with<R, F: FnOnce(&mut Element) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow_mut())
    }

    pub fn with_style(self, style: Style) -> Self {
        let mut this = self.borrow_mut();
        assert!(this.kind == ElementKind::Div
            || this.kind == ElementKind::Button
            || this.kind == ElementKind::Span);
        this.style = style;
        drop(this);
        self
    }
}

impl Element {
    pub fn display(&self) -> Display {
        self.computed_style.get("display")
        .map(|display| {
            match display.as_str() {
                "none"   => Display::None,
                "inline" => Display::Inline,
                "block"  => Display::Block,
                _ => unimplemented!(),
            }
        })
        .unwrap_or(self.kind.default_display())
    }
}


enum RenderElement {
    Element { ptr: ElementRef },
    Text {
        pos: [f32; 2],
        layout: TextLayout,
        objects: Vec<ElementRef>,
    },
}



impl Element {
    pub fn new(kind: ElementKind, ctx: Ctx) -> Element {
        Element {
            kind, ctx,
            this: None,
            parent: None,
            first_child: None, last_child: None,
            next_sibling: None, prev_sibling: None,
            pos: [0.0, 0.0], size: [0.0, 0.0],
            baseline: 0.0,
            style: Style::new(),
            computed_style: Style::new(),
            render_children: vec![],
            text: String::new(),
        }
    }

    pub fn visit_children<F: FnMut(&ElementRef)>(first_child: &Option<ElementRef>, mut f: F) {
        let mut at = first_child.clone();
        while let Some(child) = at {
            f(&child);
            at = child.borrow().next_sibling.clone();
        }
    }
}



// STYLE

impl Element {
    pub fn style(&mut self, parent: &Style) {
        fn is_inherited_style(name: &str) -> bool {
            match name {
                "text_color" => true,
                _ => false,
            }
        }

        let mut computed = Style::new();

        // inherited props.
        for (k, v) in parent {
            if is_inherited_style(k) {
                computed.insert(k.clone(), v.clone());
            }
        }

        // element props.
        for (k, v) in self.style.iter() {
            computed.insert(k.clone(), v.clone());
        }

        self.computed_style = computed;

        Self::visit_children(&self.first_child, |child| {
            child.borrow_mut().style(&self.computed_style)
        })
    }

    pub fn render_children(&mut self) {
        struct ChildRenderer<'a> {
            ctx: Ctx,
            children: &'a mut Vec<RenderElement>,
            builder: TextLayoutBuilder,
            objects: Vec<ElementRef>,
        }

        impl<'a> ChildRenderer<'a> {
            fn flush(&mut self) {
                if self.builder.text().len() == 0 {
                    return;
                }

                let mut new_builder = TextLayoutBuilder::new(self.ctx, self.builder.base_format());
                new_builder.set_format(self.builder.current_format());

                let builder = core::mem::replace(&mut self.builder, new_builder);
                let layout  = builder.build();
                let objects = core::mem::replace(&mut self.objects, vec![]);
                self.children.push(RenderElement::Text { pos: [0.0; 2], layout, objects });
            }

            fn with_style<F: FnOnce(&mut Self)>(&mut self, style: &Style, f: F) {
                let old_format = self.builder.current_format();

                let color =
                    style.get("text_color")
                    .map(|color| {
                        assert!(color.len() == 6);
                        u32::from_str_radix(color, 16).unwrap()
                    })
                    .unwrap_or(0x000000);
                self.builder.set_effect(color as usize);

                f(self);

                self.builder.set_format(old_format);
            }

            fn visit(&mut self, el: &ElementRef) {
                let mut e = el.borrow_mut();

                if e.kind == ElementKind::Text {
                    self.builder.add_string(&e.text);
                    return;
                }

                match e.display() {
                    Display::None => {}

                    Display::Inline => {
                        if e.kind.is_container() {
                            e.render_children();
                            self.builder.add_object();
                            self.objects.push(el.clone());
                        }
                        else {
                            self.with_style(&e.computed_style, |this| {
                                Element::visit_children(&e.first_child, |child| {
                                    this.visit(child);
                                });
                            })
                        }
                    }

                    Display::Block => {
                        e.render_children();
                        self.flush();
                        self.children.push(RenderElement::Element { ptr: el.clone() });
                    }
                }
            }
        }

        self.render_children.clear();

        let format = TextFormat {
            font: self.ctx.font_query("Roboto").unwrap(),
            font_size: 24.0,
            ..Default::default()
        };

        let mut cr = ChildRenderer {
            ctx: self.ctx,
            children: &mut self.render_children,
            builder: TextLayoutBuilder::new(self.ctx, format),
            objects: vec![],
        };

        cr.with_style(&self.computed_style, |cr| {
            Self::visit_children(&self.first_child, |child|
                cr.visit(child));
        });
        cr.flush();
    }
}



// LAYOUT

impl Element {
    pub fn max_width(&mut self) -> f32 {
        assert!(self.kind == ElementKind::Div);

        let layout = Layout::Lines;
        match layout {
            Layout::Lines => {
                // NOTE: duplicated bc we'll have to add padding here.
                // but during layout, we want the max width of the children
                // without the parent padding. if you know what i mean.
                let mut max_width = 0f32;
                for child in &mut self.render_children {
                    match child {
                        RenderElement::Element { ptr } => {
                            // assume "elements" are block elements.
                            let mut child = ptr.borrow_mut();
                            max_width = max_width.max(child.max_width());
                        }

                        RenderElement::Text { pos: _, layout, objects } => {
                            // TODO: duplicated. also, want to cache.
                            for (i, obj) in objects.iter().enumerate() {
                                let mut o = obj.borrow_mut();
                                o.layout(LayoutBox::any());

                                layout.set_object_size(i, o.size);
                                layout.set_object_baseline(i, o.baseline);
                            }

                            // TODO: dedicated max_width.
                            layout.set_layout_width(f32::INFINITY);
                            layout.layout();

                            max_width = max_width.max(layout.actual_size()[0]);
                        }
                    }
                }

                if let Some(max_width_prop) = self.computed_style.get("max_width") {
                    let max_width_prop = max_width_prop.parse::<f32>().unwrap();
                    max_width = max_width.min(max_width_prop);
                }

                max_width
            }
        }
    }

    pub fn layout(&mut self, lbox: LayoutBox) {
        assert!(self.kind == ElementKind::Div
            || self.kind == ElementKind::Button);

        let layout = Layout::Lines;
        match layout {
            Layout::Lines => {
                let this_width = {
                    if lbox.width_is_tight() {
                        lbox.max[0]
                    }
                    else {
                        let mut max_width = 0f32;
                        for child in &mut self.render_children {
                            match child {
                                RenderElement::Element { ptr } => {
                                    // assume "elements" are block elements.
                                    let mut child = ptr.borrow_mut();
                                    max_width = max_width.max(child.max_width());
                                }

                                RenderElement::Text { pos: _, layout, objects } => {
                                    // TODO: duplicated. also, want to cache.
                                    for (i, obj) in objects.iter().enumerate() {
                                        let mut o = obj.borrow_mut();
                                        o.layout(LayoutBox::any());

                                        layout.set_object_size(i, o.size);
                                        layout.set_object_baseline(i, o.baseline);
                                    }

                                    // TODO: dedicated max_width.
                                    layout.set_layout_width(f32::INFINITY);
                                    layout.layout();

                                    max_width = max_width.max(layout.actual_size()[0]);
                                }
                            }
                        }

                        max_width = lbox.clamp_width(max_width);
                        max_width
                    }
                };

                let mut last_baseline = 0.0;

                let mut cursor = 0.0;
                for child in &mut self.render_children {
                    match child {
                        RenderElement::Element { ptr } => {
                            // assume "elements" are block elements.
                            let mut child = ptr.borrow_mut();

                            let mut child_lbox = LayoutBox {
                                min: [this_width, 0.0],
                                max: [this_width, f32::INFINITY],
                            };


                            // TODO: what about fit-content?
                            // should this really be here?
                            // if not, what layout box to pass down & how does child know
                            // that it doesn't have to fit in the lbox?

                            // TODO: are loose layout boxes even a thing?
                            // maybe with other layouts?

                            let width_prop     = child.computed_style.get("width").map(|v| v.parse::<f32>().unwrap());
                            let min_width_prop = child.computed_style.get("min_width").map(|v| v.parse::<f32>().unwrap());
                            let max_width_prop = child.computed_style.get("max_width").map(|v| v.parse::<f32>().unwrap());

                            let child_min_width = min_width_prop.unwrap_or(0.0);
                            let child_max_width = max_width_prop.unwrap_or(f32::INFINITY);
                            // catch invalid props.
                            let child_max_width = child_max_width.max(child_min_width);

                            // if width is specified.
                            if let Some(width) = width_prop {
                                // use that width, clamped to child's min/max props.
                                let width = width.clamp(child_min_width, child_max_width);
                                child_lbox.min[0] = width;
                                child_lbox.max[0] = width;
                            }
                            else {
                                // use parent (this) width, clamped to child's min/max props.
                                let width = this_width.clamp(child_min_width, child_max_width);
                                child_lbox.min[0] = width;
                                child_lbox.max[0] = width;
                            }

                            let height_prop     = child.computed_style.get("height").map(|v| v.parse::<f32>().unwrap());
                            let min_height_prop = child.computed_style.get("min_height").map(|v| v.parse::<f32>().unwrap());
                            let max_height_prop = child.computed_style.get("max_height").map(|v| v.parse::<f32>().unwrap());

                            let child_min_height = min_height_prop.unwrap_or(0.0);
                            let child_max_height = max_height_prop.unwrap_or(f32::INFINITY);
                            // catch invalid props.
                            let child_max_height = child_max_height.max(child_min_height);

                            if let Some(height) = height_prop {
                                let height = height.clamp(child_min_height, child_max_height);
                                child_lbox.min[1] = height;
                                child_lbox.max[1] = height;
                            }
                            else {
                                child_lbox.min[1] = child_min_height;
                                child_lbox.max[1] = child_max_height;
                            }

                            child.layout(child_lbox);

                            let height = child.size[1];
                            child.pos = [0.0, cursor];
                            cursor += height;

                            last_baseline = cursor - child.baseline;
                        }

                        RenderElement::Text { pos, layout, objects } => {
                            for (i, obj) in objects.iter().enumerate() {
                                let mut o = obj.borrow_mut();
                                o.layout(LayoutBox::any());

                                layout.set_object_size(i, o.size);
                                layout.set_object_baseline(i, o.baseline);
                            }

                            layout.set_layout_width(this_width);
                            layout.layout();

                            for (i, obj) in objects.iter().enumerate() {
                                let mut o = obj.borrow_mut();
                                o.pos = layout.get_object_pos(i);
                            }

                            let last_line = layout.line_metrics(layout.line_count() - 1);
                            last_baseline = cursor + last_line.pos[1] + last_line.baseline;

                            let height = layout.actual_size()[1];
                            *pos = [0.0, cursor];
                            cursor += height;
                        }
                    }
                }

                let height = lbox.clamp_height(cursor.ceil());
                self.size = [this_width, height];
                self.baseline = cursor - last_baseline;
            }
        }
    }
}



// HIT TESTING & EVENTS

impl Element {
    pub fn hit_test<P: Fn(&Element) -> bool + Copy>(this: &ElementRef, x: f32, y: f32, p: P) -> Option<(ElementRef, usize)> {
        let me = this.borrow();
        assert!(me.kind == ElementKind::Div
            ||  me.kind == ElementKind::Button);

        let x = x - me.pos[0];
        let y = y - me.pos[1];

        let mut cursor = 0;
        // TODO: technically rev()
        // but have to find a proper solution for `cursor`.
        for child in me.render_children.iter() {
            match child {
                RenderElement::Element { ptr } => {
                    let result = Element::hit_test(ptr, x, y, p);
                    if result.is_some() {
                        return result;
                    }

                    cursor += 1;
                }

                RenderElement::Text { pos, layout, objects } => {
                    let x = x - pos[0];
                    let y = y - pos[1];

                    let hit = layout.hit_test_pos(x, y);
                    if !hit.out_of_bounds[0] && !hit.out_of_bounds[1] {
                        if let Some(index) = hit.object {
                            let hit = Element::hit_test(&objects[index], x, y, p);
                            if hit.is_some() {
                                return hit;
                            }
                        }
                        else {
                            let offset =
                                if hit.fraction < 0.5 { hit.text_pos_left  }
                                else                  { hit.text_pos_right };

                            return Some((this.clone(), cursor + offset as usize));
                        }
                    }

                    cursor += layout.text().len();
                }
            }
        }

        if !p(&me) {
            return None;
        }

        // TODO: closest cursor position.
        if x >= 0.0 && x < me.size[0]
        && y >= 0.0 && y < me.size[1] {
            return Some((this.clone(), 0));
        }

        None
    }

    pub fn pointer_events(&self) -> bool {
        // TODO: false by default for some elements?
        self.computed_style.get("pointer_events")
        .map(|value| value == "true")
        .unwrap_or(true)
    }

    pub fn cursor(&self) -> Cursor {
        match self.kind {
            ElementKind::Button => Cursor::Pointer,
            ElementKind::Text   => Cursor::Text,

            _ => Cursor::Default,
        }
    }
}



// PAINT

impl Element {
    pub fn paint(&mut self, rt: &ID2D1RenderTarget) {
        assert!(self.kind == ElementKind::Div
            || self.kind == ElementKind::Button);

        if let Some(color) = self.computed_style.get("background_color") {
            assert!(color.len() == 6);
            let hex = u32::from_str_radix(color, 16).unwrap();
            let r = ((hex >> 16) & 0xff) as f32 / 255.0;
            let g = ((hex >>  8) & 0xff) as f32 / 255.0;
            let b = ((hex      ) & 0xff) as f32 / 255.0;

            unsafe {
                let color = D2D1_COLOR_F { r, g, b, a: 1.0 };
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let rect = D2D_RECT_F {
                    left:   self.pos[0].round(),
                    top:    self.pos[1].round(),
                    right:  (self.pos[0] + self.size[0]).round(),
                    bottom: (self.pos[1] + self.size[1]).round(),
                };
                rt.FillRectangle(&rect, &brush);
            }
        }

        if self.kind == ElementKind::Button {
            unsafe {
                let color = D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let rect = D2D_RECT_F {
                    left:   self.pos[0].round() + 0.5,
                    top:    self.pos[1].round() + 0.5,
                    right:  (self.pos[0] + self.size[0]).round() - 0.5,
                    bottom: (self.pos[1] + self.size[1]).round() - 0.5,
                };
                rt.DrawRectangle(&rect, &brush, 1.0, None);
            }
        }

        let mut old_tfx = Default::default();
        unsafe {
            rt.GetTransform(&mut old_tfx);

            // need to round here, else rounding in children is meaningless.
            let new_tfx = Matrix3x2::translation(self.pos[0].round(), self.pos[1].round()) * old_tfx;
            rt.SetTransform(&new_tfx);
        }

        for child in &mut self.render_children {
            match child {
                RenderElement::Element { ptr } => {
                    ptr.0.borrow_mut().paint(rt);
                }

                RenderElement::Text { pos, layout, objects } => {
                    struct D2dTextRenderer<'a> {
                        rt:    &'a ID2D1RenderTarget,
                        brush: &'a ID2D1SolidColorBrush,
                        objects: &'a [ElementRef],
                    }

                    impl<'a> TextRenderer for D2dTextRenderer<'a> {
                        fn glyphs(&self, data: &DrawGlyphs) {
                            let run = DWRITE_GLYPH_RUN {
                                fontFace: Some(data.font_face.clone()),
                                fontEmSize: data.format.font_size,
                                glyphCount: data.indices.len() as u32,
                                glyphIndices: data.indices.as_ptr(),
                                glyphAdvances: data.advances.as_ptr(),
                                glyphOffsets: data.offsets.as_ptr() as *const _,
                                isSideways: false.into(),
                                bidiLevel: data.is_rtl as u32,
                            };

                            let color = data.format.effect as u32;
                            let color = D2D1_COLOR_F {
                                r: ((color >> 16) & 0xff) as f32 / 255.0,
                                g: ((color >>  8) & 0xff) as f32 / 255.0,
                                b: ((color >>  0) & 0xff) as f32 / 255.0,
                                a: 1.0,
                            };
                            unsafe { self.brush.SetColor(&color) };

                            let pos = D2D_POINT_2F {
                                x: data.pos[0],
                                y: data.pos[1],
                            };
                            unsafe { self.rt.DrawGlyphRun(pos, &run, self.brush, Default::default()) };
                        }

                        fn line(&self, data: &DrawLine, _kind: DrawLineKind) {
                            let rect = D2D_RECT_F {
                                left:   data.x0,
                                top:    data.y - data.thickness/2.0,
                                right:  data.x1,
                                bottom: data.y + data.thickness/2.0,
                            };
                            unsafe { self.rt.FillRectangle(&rect, self.brush) };
                        }

                        fn object(&self, data: &DrawObject) {
                            let mut o = self.objects[data.index as usize].borrow_mut();
                            o.paint(self.rt);
                        }
                    }

                    let color = D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
                    let brush = unsafe { rt.CreateSolidColorBrush(&color, None).unwrap() };

                    let r = D2dTextRenderer {
                        rt: rt.into(),
                        brush: &brush,
                        objects: &objects,
                    };
                    layout.draw(*pos, &r);
                }
            }
        }

        unsafe {
            rt.SetTransform(&old_tfx);
        }
    }
}




impl Ctx {
    pub fn to_ref(self, element: Element, children: Vec<ElementRef>) -> ElementRef {
        let this = ElementRef(Rc::new(RefCell::new(element)));

        let mut first_child = None;
        let mut prev_child: Option<ElementRef> = None;
        for child in children {
            child.borrow_mut().parent = Some(this.clone());

            if let Some(prev) = prev_child {
                prev.borrow_mut().next_sibling  = Some(child.clone());
                child.borrow_mut().prev_sibling = Some(prev);
                prev_child = Some(child);
            }
            else {
                child.borrow_mut().prev_sibling = None;
                first_child = Some(child.clone());
                prev_child  = Some(child);
            }
        }
        if let Some(last_child) = &prev_child {
            last_child.borrow_mut().next_sibling = None;
        }

        let this_ref = this.clone();
        this.borrow_mut_with(|this| {
            this.this = Some(this_ref);
            this.first_child = first_child;
            this.last_child  = prev_child;
        });

        this
    }

    pub fn div(self, children: Vec<ElementRef>) -> ElementRef {
        self.to_ref(Element::new(ElementKind::Div, self), children)
    }

    pub fn span(self, children: Vec<ElementRef>) -> ElementRef {
        self.to_ref(Element::new(ElementKind::Span, self), children)
    }

    pub fn text<Str: Into<String>>(self, value: Str) -> ElementRef {
        let mut result = Element::new(ElementKind::Text, self);
        result.text = value.into();
        self.to_ref(result, vec![])
    }

    pub fn button(self, children: Vec<ElementRef>) -> ElementRef {
        self.to_ref(Element::new(ElementKind::Button, self), children)
    }
}

