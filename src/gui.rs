use core::cell::RefCell;
use std::rc::Rc;

use crate::win::*;
use crate::common::*;
use crate::ctx::Ctx;
use crate::element::*;


pub struct Gui {
    ctx: Ctx,

    pub root: Option<ElementRef>,

    hover:  Option<ElementRef>,
    active: Option<ElementRef>,

    window_size: [f32; 2],
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NodeKind {
    Div,
    Button,
    Span,
    Text,
}


#[derive(Clone, Copy)]
pub struct Node (pub *const RefCell<Element>);


pub use crate::common::Cursor;


pub struct Event {
}

pub trait EventHandler: FnMut(&mut Gui, &mut Event) + 'static {}

impl<T: FnMut(&mut Gui, &mut Event) + 'static> EventHandler for T {}


pub trait IGui {
    fn create_node(&mut self, kind: NodeKind) -> Node;
    fn create_text(&mut self, value: &str) -> Node;

    fn set_children<C: Iterator<Item=Node>>(&mut self, parent: Node, children: C);
    fn set_style(&mut self, node: Node, style: Style);
    fn set_text(&mut self, node: Node, text: String);

    fn set_on_click<H: EventHandler>(&mut self, node: Node, handler: H);

    fn on_mouse_move(&mut self, x: f32, y: f32);
    fn on_mouse_down(&mut self);
    fn on_mouse_up(&mut self);

    fn set_window_size(&mut self, w: f32, h: f32);

    fn paint(&mut self, rt: &ID2D1RenderTarget);

    fn get_cursor(&mut self) -> Cursor;
}


impl Gui {
    pub fn new(ctx: Ctx) -> Gui {
        Gui {
            ctx,
            root: None,
            hover:  None,
            active: None,
            window_size: [0.0; 2],
        }
    }
}

impl IGui for Gui {
    fn create_node(&mut self, kind: NodeKind) -> Node {
        let node = Ctx::to_ref(Element::new(kind), vec![]);
        let result = Node(&*node.0);
        core::mem::forget(result);
        result
    }

    fn create_text(&mut self, value: &str) -> Node {
        let mut e = Element::new(NodeKind::Text);
        e.text = value.into();

        let node = Ctx::to_ref(e, vec![]);
        let result = Node(&*node.0);
        core::mem::forget(result);
        result
    }

    fn set_children<C: Iterator<Item=Node>>(&mut self, parent: Node, children: C) {
        let parent = unsafe { ElementRef(Rc::from_raw(parent.0)) };
        let children =
            children.into_iter()
            .map(|c| {
                let c = unsafe { ElementRef(Rc::from_raw(c.0)) };
                let r = c.clone();
                core::mem::forget(c);
                r
            })
            .collect();
        Element::set_children(&parent, children);
        core::mem::forget(parent);
    }

    fn set_style(&mut self, node: Node, style: Style) {
        let node = unsafe { ElementRef(Rc::from_raw(node.0)) };
        node.set_style(style);
        core::mem::forget(node);
    }

    fn set_text(&mut self, node: Node, text: String) {
        let node = unsafe { ElementRef(Rc::from_raw(node.0)) };
        node.set_text(text);
        core::mem::forget(node);
    }

    fn set_on_click<H: EventHandler>(&mut self, node: Node, handler: H) {
        let node = unsafe { ElementRef(Rc::from_raw(node.0)) };
        node.set_on_click(Box::new(handler));
        core::mem::forget(node);
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        let old_hover = self.hover.as_ref();
        let new_hover = {
            let root = self.root.as_ref().unwrap();
            let hit = Element::hit_test(root, x, y, Element::pointer_events);
            hit.map(|(el, _)| el)
        };

        // detect no change.
        if old_hover.is_none() && new_hover.is_none() {
            return;
        }
        if let (Some(old), Some(new)) = (old_hover, &new_hover) {
            if Rc::ptr_eq(&old.0, &new.0) {
                let mut hover = old.borrow_mut();
                hover.on_mouse_move(x, y);
                return;
            }
        }

        if let Some(old) = old_hover {
            let mut old = old.borrow_mut();
            old.hover = false;
            old.on_hover_stop();
        }

        if let Some(new) = &new_hover {
            let mut new = new.borrow_mut();
            new.hover = true;
            new.on_hover_start();
        }

        self.hover = new_hover;
    }

    fn on_mouse_down(&mut self) {
        assert!(self.active.is_none());

        if let Some(hover) = self.hover.as_ref() {
            let mut h = hover.borrow_mut();
            h.on_mouse_down();
        }

        let new_active = self.hover.as_ref();

        if let Some(new) = new_active {
            let mut new = new.borrow_mut();
            new.active = true;
            new.on_active_start();
        }

        self.active = new_active.cloned();
    }

    fn on_mouse_up(&mut self) {
        if let Some(hover) = self.hover.clone() {
            let mut h = hover.borrow_mut();
            h.on_mouse_up(self);
        }

        if let Some(old) = self.active.as_ref() {
            let mut old = old.borrow_mut();
            old.active = false;
            old.on_active_stop();
        }

        self.active = None;
    }

    fn set_window_size(&mut self, w: f32, h: f32) {
        if self.root.is_none() { return }

        let new_size = [w, h];
        if new_size == self.window_size {
            return
        }

        /*
        let mut root = self.root.as_ref().unwrap().borrow_mut();
        // TEMP
        if self.window_size == [0.0, 0.0] {
            let t0 = std::time::Instant::now();
            root.style(&Style::new());
            root.render_children();
            println!("style {:?}", t0.elapsed());
        }

        let t0 = std::time::Instant::now();
        root.layout(LayoutBox::tight([w/2.0, h]));
        println!("layout {:?}", t0.elapsed());
        */

        self.window_size = new_size;
    }

    fn paint(&mut self, rt: &ID2D1RenderTarget) {
        if self.root.is_none() { return }

        let [w, h] = self.window_size;

        // TEMP
        let mut root = self.root.as_ref().unwrap().borrow_mut();
        root.style(&Style::new());
        root.render_children(self.ctx);
        root.layout(LayoutBox::tight([w/2.0, h]));
        root.paint(rt);
    }

    fn get_cursor(&mut self) -> Cursor {
        self.hover.as_ref()
        .map(|h| h.borrow().cursor())
        .unwrap_or(Cursor::Default)
    }
}


