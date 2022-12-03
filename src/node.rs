use std::rc::Rc;

use crate::win::*;
use crate::ctx::*;
use crate::common::*;
use crate::text::*;
use crate::gui::*;


pub const SCROLLBAR_WIDTH: f32 = 20.0;
pub fn scrollbar_size(enabled: bool) -> f32 {
    enabled as i32 as f32 * SCROLLBAR_WIDTH
}


impl NodeKind {
    #[inline]
    pub const fn is_container(self) -> bool {
        use NodeKind::*;
        match self {
            Div | Button => true,
            Span | Text => false,
        }
    }

    #[inline]
    pub const fn default_display(self) -> Display {
        use NodeKind::*;
        use Display::*;
        match self {
            Div     => Block,
            Button  => Inline,
            Span    => Inline,
            Text    => Inline,
        }
    }

    #[inline]
    pub const fn takes_focus(self) -> bool {
        use NodeKind::*;
        match self {
            Div     => false,
            Button  => true,
            Span    => false,
            Text    => false,
        }
    }
}


pub(crate) struct NodeData {
    pub kind: NodeKind,

    pub this:         Node,
    pub parent:       Option<Node>,
    pub first_child:  Option<Node>,
    pub last_child:   Option<Node>,
    pub next_sibling: Option<Node>,
    pub prev_sibling: Option<Node>,

    pub pos:  [f32; 2],
    pub size: [f32; 2],
    pub baseline: f32,

    pub scroll_pos:   [f32; 2],
    pub content_size: [f32; 2],
    pub scrolling:    [bool; 2],

    pub hover:  bool,
    pub active: bool,
    pub focus:  bool,

    pub style: Style,
    pub computed_style: Style,

    render_children: Vec<RenderElement>,

    pub text: String,

    pub on_click: Option<Rc<dyn EventHandler>>,
}


impl NodeData {
    pub fn set_style(&mut self, style: Style) {
        assert!(self.kind == NodeKind::Div
            || self.kind == NodeKind::Button
            || self.kind == NodeKind::Span);
        self.style = style;
    }

    pub fn set_text(&mut self, text: String) {
        assert!(self.kind == NodeKind::Text);
        self.text = text;
    }

    pub fn set_on_click(&mut self, on_click: Rc<dyn EventHandler>) {
        assert!(self.kind == NodeKind::Button);
        self.on_click = Some(on_click);
    }

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

    pub fn takes_focus(&self) -> bool {
        self.kind.takes_focus()
    }
}


enum RenderElement {
    Element { ptr: Node },
    Text {
        pos: [f32; 2],
        layout: TextLayout,
        objects: Vec<Node>,
    },
}



impl NodeData {
    pub fn new(kind: NodeKind, this: Node) -> NodeData {
        NodeData {
            kind,
            this,
            parent: None,
            first_child: None, last_child: None,
            next_sibling: None, prev_sibling: None,
            pos: [0.0, 0.0], size: [0.0, 0.0],
            baseline: 0.0,
            scroll_pos: [0.0, 0.0],
            content_size: [0.0, 0.0],
            scrolling: [false, false],
            hover: false,
            active: false,
            focus: false,
            style: Style::new(),
            computed_style: Style::new(),
            render_children: vec![],
            text: String::new(),
            on_click: None,
        }
    }

    pub fn visit_children<F: FnMut(Node)>(gui: &Gui, first_child: Option<Node>, mut f: F) {
        let mut at = first_child;
        while let Some(child) = at {
            f(child);
            at = child.borrow(gui).next_sibling;
        }
    }
}



// TREE STRUCTURE

impl NodeData {
    // TODO: handle duplicates.
    // TODO: new child was old child (don't destroy).
    pub fn set_children(gui: &mut Gui, this: Node, children: Vec<Node>) {
        // destroy old children.
        let mut at = this.borrow(gui).first_child;
        while let Some(child) = at {
            let c = child.borrow(gui);
            let next = c.next_sibling;
            assert_eq!(c.parent, Some(this));
            drop(c);

            gui.destroy_node(child);
            at = next;
        }

        let mut first_child = None;
        let mut prev_child: Option<Node> = None;
        for child in children {
            child.borrow_mut(gui).parent = Some(this.clone());

            if let Some(prev) = prev_child {
                prev.borrow_mut(gui).next_sibling  = Some(child.clone());
                child.borrow_mut(gui).prev_sibling = Some(prev);
                prev_child = Some(child);
            }
            else {
                child.borrow_mut(gui).prev_sibling = None;
                first_child = Some(child.clone());
                prev_child  = Some(child);
            }
        }
        if let Some(last_child) = &prev_child {
            last_child.borrow_mut(gui).next_sibling = None;
        }

        let mut me = this.borrow_mut(gui);
        me.first_child = first_child;
        me.last_child  = prev_child;
    }
}



// STYLE

impl NodeData {
    pub fn style(&mut self, gui: &Gui, parent: &Style) {
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

        Self::visit_children(gui, self.first_child, |child| {
            child.borrow_mut(gui).style(gui, &self.computed_style)
        })
    }

    pub fn render_children(&mut self, ctx: Ctx, gui: &Gui) {
        struct ChildRenderer<'a> {
            ctx: Ctx,
            gui: &'a Gui,
            children: &'a mut Vec<RenderElement>,
            builder: TextLayoutBuilder,
            objects: Vec<Node>,
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

            fn visit(&mut self, el: Node) {
                let mut e = el.borrow_mut(self.gui);

                if e.kind == NodeKind::Text {
                    self.builder.add_string(&e.text);
                    return;
                }

                match e.display() {
                    Display::None => {}

                    Display::Inline => {
                        if e.kind.is_container() {
                            e.render_children(self.ctx, self.gui);
                            self.builder.add_object();
                            self.objects.push(el.clone());
                        }
                        else {
                            self.with_style(&e.computed_style, |this| {
                                NodeData::visit_children(self.gui, e.first_child, |child| {
                                    this.visit(child);
                                });
                            })
                        }
                    }

                    Display::Block => {
                        e.render_children(self.ctx, self.gui);
                        self.flush();
                        self.children.push(RenderElement::Element { ptr: el.clone() });
                    }
                }
            }
        }

        self.render_children.clear();

        let format = TextFormat {
            font: ctx.font_query("Roboto").unwrap(),
            font_size: 24.0,
            ..Default::default()
        };

        let mut cr = ChildRenderer {
            ctx, gui,
            children: &mut self.render_children,
            builder: TextLayoutBuilder::new(ctx, format),
            objects: vec![],
        };

        cr.with_style(&self.computed_style, |cr| {
            Self::visit_children(gui, self.first_child, |child|
                cr.visit(child));
        });
        cr.flush();
    }
}



// LAYOUT

impl NodeData {
    pub fn max_width(&mut self, gui: &Gui) -> f32 {
        assert!(self.kind == NodeKind::Div);

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
                            let mut child = ptr.borrow_mut(gui);
                            max_width = max_width.max(child.max_width(gui));
                        }

                        RenderElement::Text { pos: _, layout, objects } => {
                            // TODO: duplicated. also, want to cache.
                            for (i, obj) in objects.iter().enumerate() {
                                let mut o = obj.borrow_mut(gui);
                                o.layout(gui, LayoutBox::any());

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

    pub fn layout(&mut self, gui: &Gui, lbox: LayoutBox) {
        assert!(self.kind == NodeKind::Div
            || self.kind == NodeKind::Button);

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
                                    let mut child = ptr.borrow_mut(gui);
                                    max_width = max_width.max(child.max_width(gui));
                                }

                                RenderElement::Text { pos: _, layout, objects } => {
                                    // TODO: duplicated. also, want to cache.
                                    for (i, obj) in objects.iter().enumerate() {
                                        let mut o = obj.borrow_mut(gui);
                                        o.layout(gui, LayoutBox::any());

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

                        max_width = max_width.ceil();
                        max_width = lbox.clamp_width(max_width);
                        max_width
                    }
                };
                self.size[0] = this_width;

                self.scrolling = [false, false];
                loop {
                    let the_width = this_width - scrollbar_size(self.scrolling[1]);
                    self.lines_layout(gui, the_width, lbox);

                    if !self.scrolling[1] {
                        let viewport = self.size[1] - scrollbar_size(self.scrolling[0]);
                        if self.content_size[1] > viewport {
                            self.scrolling[1] = true;
                            continue;
                        }
                    }

                    if !self.scrolling[0] {
                        let viewport = self.size[0] - scrollbar_size(self.scrolling[1]);
                        if self.content_size[0] > viewport {
                            self.scrolling[0] = true;
                            continue;
                        }
                    }

                    break;
                }
            }
        }
    }

    fn lines_layout(&mut self, gui: &Gui, the_width: f32, lbox: LayoutBox) {
        let mut last_baseline = 0.0;
        let mut max_width = 0.0f32;

        let mut cursor = 0.0;
        for child in &mut self.render_children {
            match child {
                RenderElement::Element { ptr } => {
                    // assume "elements" are block elements.
                    let mut child = ptr.borrow_mut(gui);

                    let mut child_lbox = LayoutBox {
                        min: [the_width, 0.0],
                        max: [the_width, f32::INFINITY],
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
                        let width = the_width.clamp(child_min_width, child_max_width);
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

                    child.layout(gui, child_lbox);

                    max_width = max_width.max(child.size[0]);

                    let height = child.size[1];
                    child.pos = [0.0, cursor];
                    cursor += height;

                    last_baseline = cursor - child.baseline;
                }

                RenderElement::Text { pos, layout, objects } => {
                    for (i, obj) in objects.iter().enumerate() {
                        let mut o = obj.borrow_mut(gui);
                        o.layout(gui, LayoutBox::any());

                        layout.set_object_size(i, o.size);
                        layout.set_object_baseline(i, o.baseline);
                    }

                    layout.set_layout_width(the_width);
                    layout.layout();

                    for (i, obj) in objects.iter().enumerate() {
                        let mut o = obj.borrow_mut(gui);
                        o.pos = layout.get_object_pos(i);
                    }

                    let last_line = layout.line_metrics(layout.line_count() - 1);
                    last_baseline = cursor + last_line.pos[1] + last_line.baseline;

                    let size = layout.actual_size();

                    max_width = max_width.max(size[0]);

                    *pos = [0.0, cursor];
                    cursor += size[1];
                }
            }
        }

        let content_size = [max_width.ceil(), cursor.ceil()];

        self.size[1] = lbox.clamp_height(content_size[1]);
        self.baseline = cursor - last_baseline;

        self.content_size = content_size;
    }
}



// HIT TESTING & EVENTS

impl NodeData {
    pub fn hit_test<P: Fn(&NodeData) -> bool + Copy>(gui: &Gui, this: Node, x: f32, y: f32, p: P) -> Option<(Node, usize)> {
        let me = this.borrow(gui);
        assert!(me.kind == NodeKind::Div
            ||  me.kind == NodeKind::Button);

        let x = x - me.pos[0];
        let y = y - me.pos[1];

        let hit_me =
               x >= 0.0 && x < me.size[0]
            && y >= 0.0 && y < me.size[1];

        // clip_content approximation.
        if !hit_me && (me.scrolling[0] || me.scrolling[1]) {
            return None;
        }

        let viewport_x = me.size[0] - scrollbar_size(me.scrolling[1]);
        let viewport_y = me.size[1] - scrollbar_size(me.scrolling[0]);

        let hit_viewport = hit_me && x < viewport_x && y < viewport_y;

        // hit scrollbar.
        // TODO: cursor position?
        if hit_me && !hit_viewport {
            return Some((this, 0));
        }

        let x = x + me.scroll_pos[0];
        let y = y + me.scroll_pos[1];

        let mut cursor = 0;
        // TODO: technically rev()
        // but have to find a proper solution for `cursor`.
        for child in me.render_children.iter() {
            match child {
                RenderElement::Element { ptr } => {
                    let result = NodeData::hit_test(gui, *ptr, x, y, p);
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
                            let hit = NodeData::hit_test(gui, objects[index], x, y, p);
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

        // TODO: cursor position?
        if hit_me {
            return Some((this, 0));
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
            NodeKind::Button => Cursor::Pointer,
            NodeKind::Text   => Cursor::Text,

            _ => Cursor::Default,
        }
    }

    pub fn on_hover_start(&mut self) {
        //println!("{:?} hover start", self as *const _);
    }

    pub fn on_hover_stop(&mut self) {
        //println!("{:?} hover stop", self as *const _);
    }

    pub fn on_active_start(&mut self) {
        //println!("{:?} active start", self as *const _);
    }

    pub fn on_active_stop(&mut self) {
        //println!("{:?} active stop", self as *const _);
    }

    pub fn on_mouse_down(&mut self) {
        //println!("{:?} mouse down", self as *const _);
    }

    pub fn on_mouse_wheel(&mut self, delta: f32, shift_down: bool) -> bool {
        let delta = delta.round();

        if !shift_down && self.scrolling[1] {
            let viewport = self.size[1] - scrollbar_size(self.scrolling[0]);

            let pos = self.scroll_pos[1] - delta;
            self.scroll_pos[1] = pos.clamp(0.0, self.content_size[1] - viewport);
            return true;
        }

        if shift_down && self.scrolling[0] {
            let viewport = self.size[0] - scrollbar_size(self.scrolling[1]);

            let pos = self.scroll_pos[0] - delta;
            self.scroll_pos[0] = pos.clamp(0.0, self.content_size[0] - viewport);
            return true;
        }

        false
    }

    pub fn on_mouse_up(&mut self, _gui: &mut Gui) {
        //println!("{:?} mouse up", self as *const _);
    }

    pub fn get_on_click(&self) -> Option<Rc<dyn EventHandler>> {
        self.on_click.clone()
    }

    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        let _ = (x, y);
        //println!("{:?} mouse move {} {}", self as *const _, x, y);
    }
}



// PAINT

impl NodeData {
    pub fn paint(&mut self, gui: &Gui, rt: &ID2D1RenderTarget) {
        assert!(self.kind == NodeKind::Div
            || self.kind == NodeKind::Button);

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

        if self.kind == NodeKind::Button {
            unsafe {
                let mut color = D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
                if self.hover && self.active {
                    color = D2D1_COLOR_F { r: 1.0, g: 0.5, b: 0.2, a: 1.0 };
                }
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let width = if self.hover { 2.0 } else { 1.0 };

                let rect = D2D_RECT_F {
                    left:   self.pos[0].round() + 0.5,
                    top:    self.pos[1].round() + 0.5,
                    right:  (self.pos[0] + self.size[0]).round() - 0.5,
                    bottom: (self.pos[1] + self.size[1]).round() - 0.5,
                };
                rt.DrawRectangle(&rect, &brush, width, None);
            }
        }

        // clip_content approximation.
        if self.scrolling[0] || self.scrolling[1] {unsafe{
            let rect = D2D_RECT_F {
                left:   self.pos[0].round(),
                top:    self.pos[1].round(),
                right:  (self.pos[0] + self.size[0]).round(),
                bottom: (self.pos[1] + self.size[1]).round(),
            };
            rt.PushAxisAlignedClip(&rect, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED);
        }}

        let mut old_tfx = Default::default();
        unsafe {
            rt.GetTransform(&mut old_tfx);

            // need to round here, else rounding in children is meaningless.
            let x = self.pos[0] - self.scroll_pos[0];
            let y = self.pos[1] - self.scroll_pos[1];
            let new_tfx = Matrix3x2::translation(x.round(), y.round()) * old_tfx;
            rt.SetTransform(&new_tfx);
        }

        for child in &mut self.render_children {
            match child {
                RenderElement::Element { ptr } => {
                    ptr.borrow_mut(gui).paint(gui, rt);
                }

                RenderElement::Text { pos, layout, objects } => {
                    struct D2dTextRenderer<'a> {
                        gui: &'a Gui,
                        rt:    &'a ID2D1RenderTarget,
                        brush: &'a ID2D1SolidColorBrush,
                        objects: &'a [Node],
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
                            let mut o = self.objects[data.index as usize].borrow_mut(self.gui);
                            o.paint(self.gui, self.rt);
                        }
                    }

                    let color = D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
                    let brush = unsafe { rt.CreateSolidColorBrush(&color, None).unwrap() };
        

                    // TODO: not sure this should be here.
                    let mut old_tfx = Default::default();
                    unsafe {
                        rt.GetTransform(&mut old_tfx);

                        // need to round here, else rounding in children is meaningless.
                        let new_tfx = Matrix3x2::translation(pos[0].round(), pos[1].round()) * old_tfx;
                        rt.SetTransform(&new_tfx);
                    }

                    let r = D2dTextRenderer {
                        gui,
                        rt: rt.into(),
                        brush: &brush,
                        objects: &objects,
                    };
                    layout.draw([0.0, 0.0], &r);

                    unsafe {
                        rt.SetTransform(&old_tfx);
                    }
                }
            }
        }

        unsafe {
            rt.SetTransform(&old_tfx);
        }

        // clip_content approximation.
        if self.scrolling[0] || self.scrolling[1] {unsafe{
            rt.PopAxisAlignedClip();
        }}


        // scroll bars.
        if self.scrolling[0] {
            unsafe {
                let color = D2D1_COLOR_F { r: 0.8, g: 0.8, b: 0.8, a: 1.0 };
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let offset_thing = scrollbar_size(self.scrolling[0]);

                let rect = D2D_RECT_F {
                    left:   self.pos[0].round(),
                    top:    (self.pos[1] + self.size[1]).round() - SCROLLBAR_WIDTH,
                    right:  (self.pos[0] + self.size[0]).round() - offset_thing,
                    bottom: (self.pos[1] + self.size[1]).round(),
                };
                rt.FillRectangle(&rect, &brush);
                
                let viewport = self.size[0] - scrollbar_size(self.scrolling[1]);
                let hi = self.scroll_pos[0] / self.content_size[0];
                let lo = (self.scroll_pos[0] + viewport) / self.content_size[0];

                let c2 = D2D1_COLOR_F { r: 0.6, g: 0.6, b: 0.6, a: 1.0 };
                brush.SetColor(&c2);
                let r2 = D2D_RECT_F {
                    left:  (1.0 - hi)*rect.left + hi*rect.right,
                    right: (1.0 - lo)*rect.left + lo*rect.right,
                    ..rect
                };
                rt.FillRectangle(&r2, &brush);
            }
        }
        if self.scrolling[1] {
            unsafe {
                let color = D2D1_COLOR_F { r: 0.8, g: 0.8, b: 0.8, a: 1.0 };
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let offset_thing = scrollbar_size(self.scrolling[0]);

                let rect = D2D_RECT_F {
                    left:   (self.pos[0] + self.size[0]).round() - SCROLLBAR_WIDTH,
                    top:    self.pos[1].round(),
                    right:  (self.pos[0] + self.size[0]).round(),
                    bottom: (self.pos[1] + self.size[1]).round() - offset_thing,
                };
                rt.FillRectangle(&rect, &brush);

                let viewport = self.size[1] - scrollbar_size(self.scrolling[0]);
                let hi = self.scroll_pos[1] / self.content_size[1];
                let lo = (self.scroll_pos[1] + viewport) / self.content_size[1];

                let c2 = D2D1_COLOR_F { r: 0.6, g: 0.6, b: 0.6, a: 1.0 };
                brush.SetColor(&c2);
                let r2 = D2D_RECT_F {
                    top:    (1.0 - hi)*rect.top + hi*rect.bottom,
                    bottom: (1.0 - lo)*rect.top + lo*rect.bottom,
                    ..rect
                };
                rt.FillRectangle(&r2, &brush);
            }
        }

        if self.scrolling[0] && self.scrolling[1] {
            unsafe {
                let color = D2D1_COLOR_F { r: 0.8, g: 0.8, b: 0.8, a: 1.0 };
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let rect = D2D_RECT_F {
                    left:   (self.pos[0] + self.size[0]).round() - SCROLLBAR_WIDTH,
                    top:    (self.pos[1] + self.size[1]).round() - SCROLLBAR_WIDTH,
                    right:  (self.pos[0] + self.size[0]).round(),
                    bottom: (self.pos[1] + self.size[1]).round(),
                };
                rt.FillRectangle(&rect, &brush);
            }
        }

        if self.focus {
            unsafe {
                let color = D2D1_COLOR_F { r: 0.5, g: 0.8, b: 1.0, a: 1.0 };
                let brush = rt.CreateSolidColorBrush(&color, None).unwrap();

                let width = 2.0;

                let rect = D2D_RECT_F {
                    left:   self.pos[0].round() - 1.0,
                    top:    self.pos[1].round() - 1.0,
                    right:  (self.pos[0] + self.size[0]).round() + 1.0,
                    bottom: (self.pos[1] + self.size[1]).round() + 1.0,
                };
                rt.DrawRectangle(&rect, &brush, width, None);
            }
        }
    }
}

