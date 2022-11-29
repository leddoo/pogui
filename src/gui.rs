use core::cell::*;
use core::num::NonZeroU32;
use std::rc::Rc;

use crate::win::*;
use crate::common::*;
use crate::ctx::Ctx;
use crate::node::*;


pub struct Gui {
    ctx: Ctx,

    pub(crate) nodes: Vec<NodeWrapper>,
    pub root: Option<Node>,

    hover:  Option<Node>,
    active: Option<Node>,

    window_size: [f32; 2],
}

pub(crate) struct NodeWrapper {
    data: RefCell<NodeData>,
    gen:  NonZeroU32,
    used: bool,
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NodeKind {
    Div,
    Button,
    Span,
    Text,
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Node {
    index: u32,
    gen:   NonZeroU32,
}

impl Node {
    #[inline]
    pub(crate) fn get(self, nodes: &[NodeWrapper]) -> &NodeWrapper {
        let result = &nodes[self.index as usize];
        assert_eq!(self.gen, result.gen);
        assert!(result.used);
        result
    }

    #[inline]
    pub(crate) fn borrow(self, gui: &Gui) -> Ref<NodeData> {
        self.get(&gui.nodes).data.borrow()
    }

    #[inline]
    pub(crate) fn borrow_mut(self, gui: &Gui) -> RefMut<NodeData> {
        self.get(&gui.nodes).data.borrow_mut()
    }
}


pub use crate::common::Cursor;


pub struct Event {
}

pub trait EventHandler: Fn(&mut Gui, &mut Event) + 'static {}

impl<T: Fn(&mut Gui, &mut Event) + 'static> EventHandler for T {}


pub trait IGui {
    fn create_node(&mut self, kind: NodeKind) -> Node;
    fn create_text(&mut self, value: &str) -> Node;

    fn set_children<C: IntoIterator<Item=Node>>(&mut self, parent: Node, children: C);
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
            nodes: vec![],
            root: None,
            hover:  None,
            active: None,
            window_size: [0.0; 2],
        }
    }
}

impl Gui {
    pub(crate) fn create_node(&mut self, kind: NodeKind) -> Node {
        println!("create {:?}", kind);
        for (i, n) in self.nodes.iter_mut().enumerate() {
            if !n.used {
                println!("reuse");
                let node = Node { index: i as u32, gen: n.gen };
                let mut d = n.data.borrow_mut();
                d.kind = kind;
                d.this = Some(node);
                n.used = true;
                return node;
            }
        }

        let gen = NonZeroU32::new(1).unwrap();
        let node = Node { index: self.nodes.len() as u32, gen };
        self.nodes.push(NodeWrapper { 
            data: RefCell::new(NodeData::new(kind)),
            gen, 
            used: true,
        });
        node
    }

    pub(crate) fn destroy_node(&mut self, node: Node) {
        let n = &mut self.nodes[node.index as usize];
        assert_eq!(n.gen, node.gen);
        assert!(n.used);

        let mut d = n.data.borrow_mut();
        println!("destory {:?}", d.kind);
        *d = NodeData::new(NodeKind::Div);
        n.gen = NonZeroU32::new(n.gen.get() + 1).unwrap();
        n.used = false;
    }
}

impl IGui for Gui {
    fn create_node(&mut self, kind: NodeKind) -> Node {
        Gui::create_node(self, kind)
    }

    fn create_text(&mut self, value: &str) -> Node {
        let node = Gui::create_node(self, NodeKind::Text);
        let mut d = node.borrow_mut(self);
        d.text = value.into();
        node
    }

    fn set_children<C: IntoIterator<Item=Node>>(&mut self, parent: Node, children: C) {
        NodeData::set_children(self, parent, children.into_iter().collect());
    }

    fn set_style(&mut self, node: Node, style: Style) {
        let mut d = node.get(&self.nodes).data.borrow_mut();
        d.set_style(style);
    }

    fn set_text(&mut self, node: Node, text: String) {
        let mut d = node.get(&self.nodes).data.borrow_mut();
        d.set_text(text);
    }

    fn set_on_click<H: EventHandler>(&mut self, node: Node, handler: H) {
        let mut d = node.get(&self.nodes).data.borrow_mut();
        d.set_on_click(Rc::new(handler));
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        let old_hover = self.hover;
        let new_hover = {
            let root = self.root.unwrap();
            let hit = NodeData::hit_test(self, root, x, y, NodeData::pointer_events);
            hit.map(|(el, _)| el)
        };

        // detect no change.
        if old_hover.is_none() && new_hover.is_none() {
            return;
        }
        if let (Some(old), Some(new)) = (old_hover, new_hover) {
            if old == new {
                let mut hover = old.borrow_mut(self);
                hover.on_mouse_move(x, y);
                return;
            }
        }

        if let Some(old) = old_hover {
            let mut old = old.borrow_mut(self);
            old.hover = false;
            old.on_hover_stop();
        }

        if let Some(new) = new_hover {
            let mut new = new.borrow_mut(self);
            new.hover = true;
            new.on_hover_start();
        }

        self.hover = new_hover;
    }

    fn on_mouse_down(&mut self) {
        assert!(self.active.is_none());

        if let Some(hover) = self.hover {
            let mut h = hover.borrow_mut(self);
            h.on_mouse_down();
        }

        let new_active = self.hover;

        if let Some(new) = new_active {
            let mut new = new.borrow_mut(self);
            new.active = true;
            new.on_active_start();
        }

        self.active = new_active;
    }

    fn on_mouse_up(&mut self) {
        if let Some(hover) = self.hover.clone() {
            let handler = hover.borrow(self).get_on_click();
            if let Some(handler) = handler {
                handler(self, &mut Event {});
            }
        }

        if let Some(old) = self.active {
            let mut old = old.borrow_mut(self);
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
        let mut root = self.root.unwrap().borrow_mut(self);
        root.style(self, &Style::new());
        root.render_children(self.ctx, self);
        root.layout(self, LayoutBox::tight([w/2.0, h]));
        root.paint(self, rt);
    }

    fn get_cursor(&mut self) -> Cursor {
        self.hover
        .map(|h| h.borrow(self).cursor())
        .unwrap_or(Cursor::Default)
    }
}


