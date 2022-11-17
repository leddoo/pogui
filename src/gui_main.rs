use core::cell::{RefCell, Ref, RefMut};

use std::rc::Rc;
use std::collections::HashMap;

use crate::win::*;
use crate::ctx::*;
use crate::gui::text_layout::*;



#[derive(Clone, Copy, PartialEq, Debug)]
pub struct LayoutBox {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

#[allow(dead_code)] // TEMP
impl LayoutBox {
    #[inline]
    fn check_size(size: [f32; 2]) {
        // TODO: is this correct?
        assert!(size[0] >= 0.0);
        assert!(size[1] >= 0.0);
    }

    #[inline]
    fn check_size_finite(size: [f32; 2]) {
        // TODO: is this correct?
        assert!(size[0] >= 0.0 && size[0] < f32::INFINITY);
        assert!(size[1] >= 0.0 && size[1] < f32::INFINITY);
    }


    #[inline]
    pub fn min_size(min: [f32; 2]) -> LayoutBox {
        Self::check_size_finite(min);
        LayoutBox { min, max: [f32::INFINITY, f32::INFINITY] }
    }

    #[inline]
    pub fn max_size(max: [f32; 2]) -> LayoutBox {
        Self::check_size(max);
        LayoutBox { min: [0.0, 0.0], max }
    }

    #[inline]
    pub fn tight(size: [f32; 2]) -> LayoutBox {
        Self::check_size_finite(size);
        LayoutBox { min: size, max: size }
    }

    #[inline]
    pub fn any() -> LayoutBox {
        LayoutBox { min: [0.0, 0.0], max: [f32::INFINITY, f32::INFINITY] }
    }

    #[inline]
    pub fn with_max(self, max: [f32; 2]) -> LayoutBox {
        Self::check_size(max);
        LayoutBox { min: self.min, max }
    }

    #[inline]
    pub fn clamp(self, size: [f32; 2]) -> [f32; 2] {
        [size[0].clamp(self.min[0], self.max[0]),
         size[1].clamp(self.min[1], self.max[1])]
    }

    #[inline]
    pub fn clamp_axis(self, size: f32, axis: usize) -> f32 {
        size.clamp(self.min[axis], self.max[axis])
    }

    #[inline]
    pub fn clamp_width(self, size: f32) -> f32 {
        self.clamp_axis(size, 0)
    }

    #[inline]
    pub fn clamp_height(self, size: f32) -> f32 {
        self.clamp_axis(size, 1)
    }


    #[inline]
    pub fn axis_is_tight(self, axis: usize) -> bool {
        self.min[axis] == self.max[axis]
    }

    #[inline]
    pub fn width_is_tight(self) -> bool {
        self.axis_is_tight(0)
    }

    #[inline]
    pub fn height_is_tight(self) -> bool {
        self.axis_is_tight(1)
    }
}




#[derive(Clone, Copy, PartialEq, Debug)]
enum ElementKind {
    Div,
    Span,
    Text,
}


enum Layout {
    Lines,
}

type Style = HashMap<String, String>;


struct Element {
    kind: ElementKind,

    ctx: Rc<Ctx>,

    this:         Option<ElementRef>,
    parent:       Option<ElementRef>,
    first_child:  Option<ElementRef>,
    last_child:   Option<ElementRef>,
    next_sibling: Option<ElementRef>,
    prev_sibling: Option<ElementRef>,

    pos:  [f32; 2],
    size: [f32; 2],

    style: Style,
    computed_style: Style,

    render_children: Vec<RenderElement>,

    text: String,
}


#[derive(Clone)]
struct ElementRef (Rc<RefCell<Element>>);

impl ElementRef {
    #[inline]
    fn borrow(&self) -> Ref<Element> {
        self.0.borrow()
    }

    #[allow(dead_code)] // TEMP
    #[inline]
    fn borrow_with<R, F: FnOnce(&Element) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow())
    }

    #[inline]
    fn borrow_mut(&self) -> RefMut<Element> {
        self.0.borrow_mut()
    }

    #[inline]
    fn borrow_mut_with<R, F: FnOnce(&mut Element) -> R>(&self, f: F) -> R {
        f(&mut self.0.borrow_mut())
    }

    fn with_style(self, style: Style) -> Self {
        let mut this = self.borrow_mut();
        assert!(this.kind == ElementKind::Div || this.kind == ElementKind::Span);
        this.style = style;
        drop(this);
        self
    }
}


enum RenderElement {
    Element { ptr: ElementRef },
    Text  { pos: [f32; 2], layout: IDWriteTextLayout },
}



impl Element {
    fn new(kind: ElementKind, ctx: Rc<Ctx>) -> Element {
        Element {
            kind, ctx,
            this: None,
            parent: None,
            first_child: None, last_child: None,
            next_sibling: None, prev_sibling: None,
            pos: [0.0, 0.0], size: [0.0, 0.0],
            style: Style::new(),
            computed_style: Style::new(),
            render_children: vec![],
            text: String::new(),
        }
    }

    fn visit_children<F: FnMut(&ElementRef)>(first_child: &Option<ElementRef>, mut f: F) {
        let mut at = first_child.clone();
        while let Some(child) = at {
            f(&child);
            at = child.borrow().next_sibling.clone();
        }
    }

    fn style(&mut self, parent: &Style) {
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

    fn render_children(&mut self, rt: &ID2D1RenderTarget) {
        self.render_children.clear();

        let mut ren = ChildRenderer {
            ctx: &self.ctx, rt,
            render_children: &mut self.render_children,
            text: vec![],
            text_prev_style_end: 0,
            text_styles: vec![],
            current_style: 0,
            styles: vec![self.computed_style.clone()],
        };

        Self::visit_children(&self.first_child, |child|
            ren.visit(child));
        ren.flush();

        for child in &mut self.render_children {
            if let RenderElement::Element { ptr } = child {
                ptr.borrow_mut().render_children(rt);
            }
        }
    }

    fn max_width(&mut self) -> f32 {
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

                        RenderElement::Text { pos: _, layout } => {
                            let width = unsafe {
                                // yeah, whatever, this will all change.
                                let old_box_width = layout.GetMaxWidth();
                                layout.SetMaxWidth(f32::INFINITY).unwrap();
                                let width = layout.GetMetrics().unwrap().width;
                                layout.SetMaxWidth(old_box_width).unwrap();
                                width
                            };
                            max_width = max_width.max(width);
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

    fn layout(&mut self, lbox: LayoutBox) {
        assert!(self.kind == ElementKind::Div);

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

                                RenderElement::Text { pos: _, layout } => {
                                    let width = unsafe {
                                        layout.SetMaxWidth(f32::INFINITY).unwrap();
                                        layout.GetMetrics().unwrap().width
                                    };
                                    max_width = max_width.max(width);
                                }
                            }
                        }

                        max_width = lbox.clamp_width(max_width);
                        max_width
                    }
                };

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
                        }

                        RenderElement::Text { pos, layout } => {
                            let height = unsafe {
                                layout.SetMaxWidth(this_width).unwrap();
                                layout.GetMetrics().unwrap().height
                            };
                            *pos = [0.0, cursor];
                            cursor += height;
                        }
                    }
                }

                let height = lbox.clamp_height(cursor);
                self.size = [this_width, height];
            }
        }
    }

    fn paint(&mut self, rt: &ID2D1RenderTarget) {
        assert!(self.kind == ElementKind::Div);

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

                RenderElement::Text { pos, layout } => {
                    let pos = D2D_POINT_2F { x: pos[0], y: pos[1] };
                    let color = D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
                    unsafe {
                        let brush = rt.CreateSolidColorBrush(&color, None).unwrap();
                        rt.DrawTextLayout(pos, &*layout, &brush, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT);
                    }
                }
            }
        }

        unsafe {
            rt.SetTransform(&old_tfx);
        }
    }
}


struct ChildRenderer<'a> {
    ctx: &'a Ctx,
    rt: &'a ID2D1RenderTarget,
    render_children: &'a mut Vec<RenderElement>,
    text: Vec<u16>,
    text_prev_style_end: usize,
    text_styles: Vec<(usize, usize,  usize)>,
    current_style: usize,
    styles: Vec<Style>,
}

impl<'a> ChildRenderer<'a> {
    fn flush(&mut self) {
        if self.text.len() == 0 {
            return;
        }

        self.text_styles.push((self.text_prev_style_end, self.text.len(),  self.current_style));


        let layout = unsafe {
            let layout = self.ctx.dw_factory.CreateTextLayout(
                &self.text, 
                &self.ctx.text_format, 
                f32::INFINITY, f32::INFINITY).unwrap();

            for (begin, end,  style_idx) in self.text_styles.iter().cloned() {
                let style = &self.styles[style_idx];

                let color =
                    style.get("text_color")
                    .map(|color| {
                        assert!(color.len() == 6);
                        let hex = u32::from_str_radix(color, 16).unwrap();
                        let r = ((hex >> 16) & 0xff) as f32 / 255.0;
                        let g = ((hex >>  8) & 0xff) as f32 / 255.0;
                        let b = ((hex      ) & 0xff) as f32 / 255.0;
                        (r, g, b)
                    })
                    .unwrap_or((0.0, 0.0, 0.0));

                let color = D2D1_COLOR_F {
                    r: color.0,
                    g: color.1,
                    b: color.2,
                    a: 1.0
                };

                let brush = self.rt.CreateSolidColorBrush(&color, None).unwrap();

                let range = DWRITE_TEXT_RANGE {
                    startPosition: begin as u32,
                    length: (end - begin) as u32,
                };

                // this doesn't technically need to happen here.
                layout.SetDrawingEffect(&brush, range).unwrap();
            }

            layout
        };
        self.render_children.push(RenderElement::Text { pos: [0.0, 0.0], layout });
        self.text.clear();
        self.text_prev_style_end = 0;
        self.text_styles.clear();
    }

    fn set_style(&mut self, index: usize) {
        self.text_styles.push((self.text_prev_style_end, self.text.len(),  self.current_style));
        self.text_prev_style_end = self.text.len();
        self.current_style = index;
    }

    fn visit(&mut self, child: &ElementRef) {
        let child_el = child.borrow();
        match child_el.kind {
            ElementKind::Div => {
                // only support block divs for now.
                // don't know how to do inline objects.
                self.flush();
                self.render_children.push(RenderElement::Element { ptr: child.clone() });
            }

            ElementKind::Span => {
                let prev_style = self.current_style;

                let style_idx = self.styles.len();
                self.styles.push(child_el.computed_style.clone());
                self.set_style(style_idx);

                Element::visit_children(&child_el.first_child, |span_child| {
                    self.visit(span_child);
                });

                self.set_style(prev_style);
            }

            ElementKind::Text => {
                let utf16 = child_el.text.encode_utf16();
                self.text.extend(utf16);
            }
        }
    }
}




struct Ctx {
    dw_factory: IDWriteFactory2,
    text_format: IDWriteTextFormat,
}

struct RcCtx (Rc<Ctx>);

impl RcCtx {
    fn new() -> RcCtx {
        unsafe {
            let dw_factory: IDWriteFactory2 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();

            let text_format = dw_factory.CreateTextFormat(
                w!("Roboto"),
                None,
                DWRITE_FONT_WEIGHT_REGULAR,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                24.0,
                w!("en-us")).unwrap();

            RcCtx(Rc::new(Ctx { dw_factory, text_format }))
        }
    }

    fn to_ref(&self, element: Element, children: Vec<ElementRef>) -> ElementRef {
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

    fn div(&self, children: Vec<ElementRef>) -> ElementRef {
        self.to_ref(Element::new(ElementKind::Div, self.0.clone()), children)
    }

    fn span(&self, children: Vec<ElementRef>) -> ElementRef {
        self.to_ref(Element::new(ElementKind::Span, self.0.clone()), children)
    }

    fn text<Str: Into<String>>(&self, value: Str) -> ElementRef {
        let mut result = Element::new(ElementKind::Text, self.0.clone());
        result.text = value.into();
        self.to_ref(result, vec![])
    }
}



#[allow(dead_code)]
struct Main {
    window: HWND,
    d2d_factory: ID2D1Factory,

    rt: ID2D1HwndRenderTarget,
    rt_size: D2D_SIZE_U,

    size: [u32; 2],

    root: ElementRef,
}

impl Main {
    unsafe fn init(window: HWND, root: ElementRef) -> Main {
        let d2d_factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();

        let mut rect = RECT::default();
        GetClientRect(window, &mut rect);

        let size = [
            (rect.right - rect.left) as u32,
            (rect.bottom - rect.top) as u32,
        ];

        let rt_size = D2D_SIZE_U { width: size[0], height: size[1] };

        let rt = d2d_factory.CreateHwndRenderTarget(
            &Default::default(),
            &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: window,
                pixelSize: rt_size,
                ..Default::default()
            }).unwrap();

        Main {
            window,
            d2d_factory,
            rt, rt_size,
            size,
            root,
        }
    }

    fn paint(&mut self) {
        unsafe {
            let mut rect = RECT::default();
            GetClientRect(self.window, &mut rect);

            let size = [
                (rect.right - rect.left) as u32,
                (rect.bottom - rect.top) as u32,
            ];

            let rt_size = D2D_SIZE_U { width: size[0], height: size[1] };
            if rt_size != self.rt_size {
                self.rt.Resize(&rt_size).unwrap();
                self.rt_size = rt_size;
            }


            let mut root = self.root.borrow_mut();

            let t0 = std::time::Instant::now();

            root.style(&Default::default());

            root.render_children((&self.rt).into());

            root.layout(LayoutBox::tight([size[0] as f32 / 2.0, size[1] as f32]));
            root.pos = [0.0, 0.0];

            println!("styling & layout took {:?}", t0.elapsed());

            self.rt.BeginDraw();

            self.rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

            root.paint((&self.rt).into());

            self.rt.EndDraw(None, None).unwrap();
        }
    }
}

pub fn main() {
    let ctx = RcCtx::new();

    let root =
        ctx.div(vec![
            ctx.text("hello, "),
            ctx.text("weirdo!"),
            ctx.div(vec![
                ctx.text("new line cause div"),
                ctx.div(vec![
                    ctx.text("div in div with inherited text color."),
                    ctx.div(vec![
                        ctx.text("ADivInADivInADiv"),
                    ]),
                ]).with_style([
                    ("min_width".into(), "190".into()),
                    ("max_width".into(), "400".into()),
                    ("min_height".into(), "70".into()),
                    ("max_height".into(), "100".into()),
                    ("background_color".into(), "d040a0".into()),
                ].into()),
                ctx.div(vec![]).with_style([
                    ("width".into(),  "50".into()),
                    ("height".into(), "50".into()),
                    ("background_color".into(), "807060".into()),
                ].into()),
                ctx.div(vec![
                    ctx.text("nested div with a "),
                    ctx.span(vec![ctx.text("different")]).with_style([
                        ("text_color".into(), "40b040".into()),
                    ].into()),
                    ctx.text(" text color."),
                ]).with_style([
                    ("text_color".into(), "306080".into()),
                ].into()),
                ctx.text("more of the outer div"),
            ]).with_style([
                ("font_size".into(), "69".into()),
                ("text_color".into(), "802020".into()),
                ("background_color".into(), "eeeeff".into()),
                ("min_height".into(), "250".into()),
            ].into()),
        ]);

    unsafe {
        std::panic::set_hook(Box::new(|info| {
            println!("panic: {}", info);
            loop {}
        }));


        const WINDOW_CLASS_NAME: &HSTRING = w!("window_class");

        let instance = GetModuleHandleW(None).unwrap();

        // set up window class
        {
            let wc = WNDCLASSW {
                hInstance: instance,
                lpszClassName: WINDOW_CLASS_NAME.into(),
                lpfnWndProc: Some(window_proc),
                hIcon: LoadIconW(None, IDI_APPLICATION).unwrap(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
                ..Default::default()
            };

            let atom = RegisterClassW(&wc);
            assert!(atom != 0);
        }

        // create window.
        let window = CreateWindowExW(
            Default::default(),
            WINDOW_CLASS_NAME,
            w!("window"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT,
            CW_USEDEFAULT, CW_USEDEFAULT,
            None,
            None,
            GetModuleHandleW(None).unwrap(),
            None);
        assert!(window.0 != 0);

        let main = RefCell::new(Main::init(window, root));
        SetWindowLongPtrW(window, GWLP_USERDATA, &main as *const _ as isize);


        // event loop.
        loop {
            let mut message = MSG::default();
            let result = GetMessageW(&mut message, HWND(0), 0, 0).0;
            if result > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            else if result == 0 {
                break;
            }
            else {
                panic!();
            }
        }

    }
}


unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    fn lo_u16(a: isize) -> u32 { (a as usize as u32) & 0xffff }
    fn hi_u16(a: isize) -> u32 { ((a as usize as u32) >> 16) & 0xffff }

    let mut main = {
        let main = GetWindowLongPtrW(window, GWLP_USERDATA) as *const RefCell<Main>;
        if main == core::ptr::null() {
            return DefWindowProcW(window, message, wparam, lparam);
        }

        (*main).borrow_mut()
    };

    let message = message as u32;
    match message {
        WM_CLOSE => {
            PostQuitMessage(0);
            LRESULT(0)
        },

        WM_SIZE => {
            let _w = lo_u16(lparam.0);
            let _h = hi_u16(lparam.0);
            InvalidateRect(window, None, false);
            LRESULT(0)
        },

        WM_PAINT => {
            main.paint();
            ValidateRect(window, None);
            LRESULT(0)
        },

        _ => {
            drop(main);
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}
