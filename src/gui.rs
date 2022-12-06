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
    root: Node,

    hover:  Option<Node>,
    active: Option<Node>,
    focus:  Option<Node>,
    passive_focus: Option<(Node, usize)>,

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
        debug_assert_eq!(result.data.borrow().this, self);
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
    pub target: Node,
}

pub trait EventHandler: Fn(&mut Gui, &mut Event) + 'static {}

impl<T: Fn(&mut Gui, &mut Event) + 'static> EventHandler for T {}


pub trait IGui {
    fn create_node(&mut self, kind: NodeKind) -> Node;
    fn create_text(&mut self, value: &str) -> Node;

    fn set_children<C: IntoIterator<Item=Node>>(&mut self, parent: Node, children: C);

    fn prepend_child(&mut self, parent: Node, new_child: Node);
    fn append_child(&mut self, parent: Node, new_child: Node);

    fn insert_before_child(&mut self, parent: Node, ref_child: Option<Node>, new_child: Node);
    fn insert_after_child(&mut self, parent: Node, ref_child: Option<Node>, new_child: Node);

    fn replace_child(&mut self, parent: Node, old_child: Node, new_child: Node, keep_alive: bool);

    fn remove_child(&mut self, parent: Node, child: Node, keep_alive: bool);
    fn remove_node(&mut self, node: Node, keep_alive: bool);
    fn destroy_node(&mut self, node: Node);

    fn swap_nodes(&mut self, a: Node, b: Node);

    fn get_parent(&self, node: Node) -> Option<Node>;
    fn get_first_child(&self, node: Node) -> Option<Node>;
    fn get_last_child(&self, node: Node) -> Option<Node>;
    fn get_prev_sibling(&self, node: Node) -> Option<Node>;
    fn get_next_sibling(&self, node: Node) -> Option<Node>;

    fn next_node_pre_order(&self, node: Node) -> Option<Node>;
    fn prev_node_pre_order(&self, node: Node) -> Option<Node>;
    fn next_node_post_order(&self, node: Node) -> Option<Node>;
    fn prev_node_post_order(&self, node: Node) -> Option<Node>;

    fn set_style(&mut self, node: Node, style: Style);
    fn set_text(&mut self, node: Node, text: String);

    fn set_on_click<H: EventHandler>(&mut self, node: Node, handler: H);

    fn on_key_down(&mut self, vk: u32);
    fn on_key_up(&mut self, vk: u32);
    fn on_char(&mut self, cp: char, shift_down: bool);

    fn on_mouse_move(&mut self, x: f32, y: f32);
    fn on_mouse_down(&mut self, x: f32, y: f32);
    fn on_mouse_up(&mut self);
    fn on_mouse_wheel(&mut self, delta: f32, shift_down: bool);

    fn set_window_size(&mut self, w: f32, h: f32);

    fn paint(&mut self, rt: &ID2D1RenderTarget);

    fn get_cursor(&self) -> Cursor;

    fn root(&self) -> Node;
}


impl Gui {
    pub fn new(ctx: Ctx) -> Gui {
        let fake_root = Node {
            index: u32::MAX,
            gen: NonZeroU32::new(u32::MAX).unwrap(),
        };

        let mut gui = Gui {
            ctx,
            nodes: vec![],
            root: fake_root,
            hover:  None,
            active: None,
            focus:  None,
            passive_focus: None,
            window_size: [0.0; 2],
        };
        gui.root = gui.alloc_node(NodeKind::Div);
        // TEMP: invariant: all nodes in the tree have a parent.
        // todo: use a special constant? use a valid, dummy parent root node?
        gui.root.borrow_mut(&gui).parent = Some(fake_root);
        gui
    }
}

impl Gui {
    pub(crate) fn alloc_node(&mut self, kind: NodeKind) -> Node {
        println!("create {:?}", kind);
        for (i, n) in self.nodes.iter_mut().enumerate() {
            if !n.used {
                println!("reuse");
                let node = Node { index: i as u32, gen: n.gen };
                let mut d = n.data.borrow_mut();
                d.kind = kind;
                d.this = node;
                n.used = true;
                return node;
            }
        }

        let gen = NonZeroU32::new(1).unwrap();
        let node = Node { index: self.nodes.len() as u32, gen };
        self.nodes.push(NodeWrapper {
            data: RefCell::new(NodeData::new(kind, node)),
            gen,
            used: true,
        });
        node
    }

    pub(crate) fn free_node(&mut self, node: Node) {
        // clear hover/active.
        if self.hover  == Some(node) { self.hover  = None; }
        if self.active == Some(node) { self.active = None; }
        if self.focus  == Some(node) { self.focus  = None; }

        // free children.
        let mut at = node.borrow(self).first_child;
        while let Some(child) = at {
            let c = child.borrow(self);
            let next = c.next_sibling;
            assert_eq!(c.parent, Some(node));
            drop(c);

            self.free_node(child);
            at = next;
        }

        let n = &mut self.nodes[node.index as usize];
        assert_eq!(n.gen, node.gen);
        assert!(n.used);

        let mut d = n.data.borrow_mut();
        println!("destroy {:?}", d.kind);
        *d = NodeData::new(NodeKind::Div, Node { index: u32::MAX, gen: NonZeroU32::new(u32::MAX).unwrap() });
        n.gen = NonZeroU32::new(n.gen.get() + 1).unwrap();
        n.used = false;
    }


    fn check_tree(&self) -> bool {
        // check hover/active refs are valid.
        if let Some(hover)  = self.hover  { let h = hover.borrow(self);  assert_ne!(h.parent, None); }
        if let Some(active) = self.active { let a = active.borrow(self); assert_ne!(a.parent, None); }
        if let Some(focus)  = self.focus  { let f = focus.borrow(self);  assert_ne!(f.parent, None); }

        let mut visited = vec![false; self.nodes.len()];

        for (i, n) in self.nodes.iter().enumerate() {
            if !n.used { continue }

            let this = Node { index: i as u32, gen: n.gen };
            let d = n.data.borrow();
            assert_eq!(d.this, this);

            // check active/hover.
            if d.hover  { assert_eq!(self.hover,  Some(this)) }
            if d.active { assert_eq!(self.active, Some(this)) }

            // check siblings (technically redundant).
            if d.parent.is_some() {
                if let Some(next) = d.next_sibling {
                    let next = next.borrow(self);
                    assert_eq!(next.prev_sibling, Some(this));
                }
                if let Some(prev) = d.prev_sibling {
                    let prev = prev.borrow(self);
                    assert_eq!(prev.next_sibling, Some(this));
                }
            }
            else {
                assert_eq!(d.next_sibling, None);
                assert_eq!(d.prev_sibling, None);
            }

            // check children.
            //  - correct parent.
            //  - correct siblings.
            //  - acyclic.
            if let Some(first) = d.first_child {
                let last = d.last_child.unwrap();

                let mut at = first;
                let mut it = at.borrow(self);
                assert_eq!(it.prev_sibling, None);
                loop {
                    assert_eq!(it.parent, Some(this));
                    assert!(!visited[at.index as usize]);
                    visited[at.index as usize] = true;

                    if at == last {
                        break;
                    }

                    let next_at = it.next_sibling.unwrap();
                    let next_it = next_at.borrow(self);
                    assert_eq!(next_it.prev_sibling, Some(at));
                    at = next_at;
                    it = next_it;
                }
                assert_eq!(it.next_sibling, None);
            }
            else {
                assert_eq!(d.last_child, None);
            }
        }

        let root_index = self.root.index as usize;
        assert!(visited[root_index] == false);
        visited[root_index] = true;

        for (i, n) in self.nodes.iter().enumerate() {
            if n.used {
                let d = n.data.borrow();
                assert!(d.parent.is_none() || visited[i]);
            }
        }

        true
    }

    fn next_pre_order<P: Fn(&NodeData) -> bool>(&self, node: Node, p: P) -> Option<Node> {
        let mut at = node;
        loop {
            let d = at.borrow(self);
            let first_child = d.first_child;
            let mut parent = d.parent.unwrap();
            let mut next   = d.next_sibling;
            drop(d);

            if let Some(first_child) = first_child {
                at = first_child;
            }
            else {
                if at == self.root {
                    return None;
                }

                // go up, until we can go right.
                while next.is_none() {
                    at = parent;
                    if at == self.root {
                        return None;
                    }

                    let d = at.borrow(self);
                    parent = d.parent.unwrap();
                    next   = d.next_sibling;
                }
                at = next.unwrap();
            }

            if p(&at.borrow(self)) {
                return Some(at);
            }
        }
    }

    fn prev_pre_order<P: Fn(&NodeData) -> bool>(&self, node: Node, p: P) -> Option<Node> {
        let mut at = node;
        while at != self.root {
            let d = at.borrow(self);
            let parent = d.parent.unwrap();
            let prev   = d.prev_sibling;
            drop(d);

            if let Some(prev) = prev {
                at = prev;

                while let Some(last_child) = at.borrow(self).last_child {
                    at = last_child;
                }
            }
            else {
                at = parent;
            }

            if p(&at.borrow(self)) {
                return Some(at);
            }
        }
        None
    }

    fn next_post_order<P: Fn(&NodeData) -> bool>(&self, node: Node, p: P) -> Option<Node> {
        let mut at = node;
        while at != self.root {
            let d = at.borrow(self);
            let parent = d.parent.unwrap();
            let next   = d.next_sibling;
            drop(d);

            if let Some(next) = next {
                at = next;

                while let Some(first_child) = at.borrow(self).first_child {
                    at = first_child;
                }
            }
            else {
                at = parent;
            }

            if p(&at.borrow(self)) {
                return Some(at);
            }
        }
        None
    }

    fn prev_post_order<P: Fn(&NodeData) -> bool>(&self, node: Node, p: P) -> Option<Node> {
        let mut at = node;
        loop {
            let d = at.borrow(self);
            let last_child = d.last_child;
            let mut parent = d.parent.unwrap();
            let mut prev   = d.prev_sibling;
            drop(d);

            if let Some(last_child) = last_child {
                at = last_child;
            }
            else {
                if at == self.root {
                    return None;
                }

                // go up, until we can go left.
                while prev.is_none() {
                    at = parent;
                    if at == self.root {
                        return None;
                    }

                    let d = at.borrow(self);
                    parent = d.parent.unwrap();
                    prev   = d.prev_sibling;
                }
                at = prev.unwrap();
            }

            if p(&at.borrow(self)) {
                return Some(at);
            }
        }
    }

    fn clamp_scroll_offsets(&mut self) {
        for node in self.nodes.iter() {
            if !node.used {
                continue;
            }

            let mut me = node.data.borrow_mut();

            let viewport_x = me.size[0] - scrollbar_size(me.scrolling[1]);
            let viewport_y = me.size[1] - scrollbar_size(me.scrolling[0]);
            me.scroll_pos[0] = me.scroll_pos[0].clamp(0.0, (me.content_size[0] - viewport_x).max(0.0));
            me.scroll_pos[1] = me.scroll_pos[1].clamp(0.0, (me.content_size[1] - viewport_y).max(0.0));
        }
    }
}

impl IGui for Gui {
    fn create_node(&mut self, kind: NodeKind) -> Node {
        self.alloc_node(kind)
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


    fn prepend_child(&mut self, parent: Node, new_child: Node) {
        // remove from old parent.
        if let Some(old_parent) = self.get_parent(new_child) {
            self.remove_child(old_parent, new_child, true);
        }

        let mut p = parent.borrow_mut(self);
        let mut n = new_child.borrow_mut(self);
        assert_eq!(n.parent, None);
        n.parent = Some(parent);
        n.prev_sibling = None;
        n.next_sibling = p.first_child;

        if let Some(first) = p.first_child {
            let mut first = first.borrow_mut(self);
            assert_eq!(first.prev_sibling, None);
            first.prev_sibling = Some(new_child);
        }
        else {
            assert_eq!(p.last_child, None);
            p.last_child = Some(new_child);
        }
        p.first_child = Some(new_child);

        drop((p, n));
        debug_assert!(self.check_tree());
    }

    fn append_child(&mut self, parent: Node, new_child: Node) {
        // remove from old parent.
        if let Some(old_parent) = self.get_parent(new_child) {
            self.remove_child(old_parent, new_child, true);
        }

        let mut p = parent.borrow_mut(self);
        let mut n = new_child.borrow_mut(self);
        assert_eq!(n.parent, None);
        n.parent = Some(parent);
        n.prev_sibling = p.last_child;
        n.next_sibling = None;

        if let Some(last) = p.last_child {
            let mut last = last.borrow_mut(self);
            assert_eq!(last.next_sibling, None);
            last.next_sibling = Some(new_child);
        }
        else {
            assert_eq!(p.first_child, None);
            p.first_child = Some(new_child);
        }
        p.last_child = Some(new_child);

        drop((p, n));
        debug_assert!(self.check_tree());
    }

    fn insert_before_child(&mut self, parent: Node, ref_child: Option<Node>, new_child: Node) {
        let Some(ref_child) = ref_child else {
            self.append_child(parent, new_child);
            return;
        };
        if ref_child == new_child {
            return;
        }

        // remove from old parent.
        if let Some(old_parent) = self.get_parent(new_child) {
            self.remove_child(old_parent, new_child, true);
        }

        let mut p = parent.borrow_mut(self);
        let mut r = ref_child.borrow_mut(self);
        let mut n = new_child.borrow_mut(self);
        assert_eq!(r.parent, Some(parent)); // TEMP

        assert_eq!(n.parent, None);
        n.parent = Some(parent);
        n.prev_sibling = r.prev_sibling;
        n.next_sibling = Some(ref_child);

        if let Some(prev) = r.prev_sibling {
            let mut prev = prev.borrow_mut(self);
            assert_eq!(prev.next_sibling, Some(ref_child));
            prev.next_sibling = Some(new_child);
        }
        else {
            assert_eq!(p.first_child, Some(ref_child));
            p.first_child = Some(new_child);
        }

        r.prev_sibling = Some(new_child);

        drop((p, r, n));
        debug_assert!(self.check_tree());
    }

    fn insert_after_child(&mut self, parent: Node, ref_child: Option<Node>, new_child: Node) {
        let Some(ref_child) = ref_child else {
            self.prepend_child(parent, new_child);
            return;
        };
        if ref_child == new_child {
            return;
        }

        // remove from old parent.
        if let Some(old_parent) = self.get_parent(new_child) {
            self.remove_child(old_parent, new_child, true);
        }

        let mut p = parent.borrow_mut(self);
        let mut r = ref_child.borrow_mut(self);
        let mut n = new_child.borrow_mut(self);
        assert_eq!(r.parent, Some(parent)); // TEMP

        assert_eq!(n.parent, None);
        n.parent = Some(parent);
        n.prev_sibling = Some(ref_child);
        n.next_sibling = r.next_sibling;

        if let Some(next) = r.next_sibling {
            let mut next = next.borrow_mut(self);
            assert_eq!(next.prev_sibling, Some(ref_child));
            next.prev_sibling = Some(new_child);
        }
        else {
            assert_eq!(p.last_child, Some(ref_child));
            p.last_child = Some(new_child);
        }

        r.next_sibling = Some(new_child);

        drop((p, r, n));
        debug_assert!(self.check_tree());
    }

    fn replace_child(&mut self, parent: Node, old_child: Node, new_child: Node, keep_alive: bool) {
        self.insert_after_child(parent, Some(old_child), new_child);
        self.remove_child(parent, old_child, keep_alive);
    }

    fn remove_child(&mut self, parent: Node, child: Node, keep_alive: bool) {
        // clear hover/active.
        if self.hover  == Some(child) { self.hover  = None; }
        if self.active == Some(child) { self.active = None; }
        if self.focus  == Some(child) { self.focus  = None; }

        let mut p = parent.borrow_mut(self);
        let mut c = child.borrow_mut(self);
        assert_eq!(c.parent, Some(parent)); // TEMP

        if let Some(prev) = c.prev_sibling {
            let mut prev = prev.borrow_mut(self);
            assert_eq!(prev.next_sibling, Some(child));
            prev.next_sibling = c.next_sibling;
        }
        else {
            assert_eq!(p.first_child, Some(child));
            p.first_child = c.next_sibling;
        }

        if let Some(next) = c.next_sibling {
            let mut next = next.borrow_mut(self);
            assert_eq!(next.prev_sibling, Some(child));
            next.prev_sibling = c.prev_sibling;
        }
        else {
            assert_eq!(p.last_child, Some(child));
            p.last_child = c.prev_sibling;
        }

        c.hover  = false;
        c.active = false;
        c.parent = None;
        c.next_sibling = None;
        c.prev_sibling = None;

        drop((c, p));

        if !keep_alive {
            self.free_node(child)
        }

        debug_assert!(self.check_tree());
    }

    fn remove_node(&mut self, node: Node, keep_alive: bool) {
        let parent = node.borrow(self).parent;
        if let Some(parent) = parent {
            self.remove_child(parent, node, keep_alive);
        }
        else if !keep_alive {
            self.free_node(node);
            debug_assert!(self.check_tree());
        }
    }

    fn destroy_node(&mut self, node: Node) {
        self.remove_node(node, false);
    }

    fn swap_nodes(&mut self, a: Node, b: Node) {
        if a == b {
            return;
        }

        // get parents & references.
        let (pa, ra) = {
            let a = a.borrow(self);
            (a.parent, a.prev_sibling)
        };
        let (pb, rb) = {
            let b = b.borrow(self);
            (b.parent, b.prev_sibling)
        };

        match (pa, pb) {
            (Some(pa), Some(pb)) => {
                self.insert_after_child(pa, ra, b);
                self.insert_after_child(pb, rb, a);
            }

            (Some(pa), None)     => self.replace_child(pa, a, b, true),
            (None,     Some(pb)) => self.replace_child(pb, b, a, true),

            (None, None) => (),
        }
    }


    #[inline]
    fn get_parent(&self, node: Node) -> Option<Node> {
        node.borrow(self).parent
    }
    #[inline]
    fn get_first_child(&self, node: Node) -> Option<Node> {
        node.borrow(self).first_child
    }
    #[inline]
    fn get_last_child(&self, node: Node) -> Option<Node> {
        node.borrow(self).last_child
    }
    #[inline]
    fn get_prev_sibling(&self, node: Node) -> Option<Node> {
        node.borrow(self).prev_sibling
    }
    #[inline]
    fn get_next_sibling(&self, node: Node) -> Option<Node> {
        node.borrow(self).next_sibling
    }


    #[inline]
    fn next_node_pre_order(&self, node: Node) -> Option<Node> {
        self.next_pre_order(node, |_| true)
    }
    #[inline]
    fn prev_node_pre_order(&self, node: Node) -> Option<Node> {
        self.prev_pre_order(node, |_| true)
    }
    #[inline]
    fn next_node_post_order(&self, node: Node) -> Option<Node> {
        self.next_post_order(node, |_| true)
    }
    #[inline]
    fn prev_node_post_order(&self, node: Node) -> Option<Node> {
        self.prev_post_order(node, |_| true)
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

    fn on_key_down(&mut self, vk: u32) {
        let _ = vk;
    }

    fn on_key_up(&mut self, vk: u32) {
        let _ = vk;
    }

    fn on_char(&mut self, cp: char, shift_down: bool) {
        // TEMP: why, just why, windows?
        if cp == '\r' {
            if let Some(focus) = self.focus {
                let handler = focus.borrow(self).get_on_click();
                if let Some(handler) = handler {
                    handler(self, &mut Event { target: focus });
                }
            }
        }

        if cp == '\t' {
            let (start_node, _start_offset) =
                self.focus.map(|node| (node, 0))
                .or(self.passive_focus)
                .unwrap_or((self.root, 0));

            let next_focus = 
                if !shift_down {
                    self.next_pre_order(start_node, NodeData::takes_focus)
                }
                else {
                    self.prev_pre_order(start_node, NodeData::takes_focus)
                };

            if let Some(next_focus) = next_focus {
                // TEMP
                if let Some(focus) = self.focus {
                    focus.borrow_mut(self).focus = false;
                }
                next_focus.borrow_mut(self).focus = true;
                self.focus = Some(next_focus);
            }
        }
    }

    fn on_mouse_move(&mut self, x: f32, y: f32) {
        // TEMP
        {
            let [w, h] = self.window_size;
            let mut root = self.root.borrow_mut(self);
            root.style(self, &Style::new());
            root.render_children(self.ctx, self);
            root.layout(self, LayoutBox::tight([(w/2.0).ceil(), h]));
        }
        self.clamp_scroll_offsets();

        let old_hover = self.hover;
        let new_hover = {
            let hit = NodeData::hit_test(self, self.root, x, y, NodeData::pointer_events);
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

    fn on_mouse_down(&mut self, x: f32, y: f32) {
        // TODO: handle this gracefully.
        // it did fire. not 100% sure why.
        // prob cause don't register for mouse leave events.
        // but shouldn't really assume valid input msgs anyway.
        // cause other programs can send them directly, right?
        assert!(self.active.is_none());

        // TEMP
        if let Some(focus) = self.focus {
            focus.borrow_mut(self).focus = false;
        }

        if let Some(hover) = self.hover {
            let mut h = hover.borrow_mut(self);
            h.on_mouse_down();

            if h.takes_focus() {
                h.focus = true;
                drop(h);
                self.focus = Some(hover);
            }
            else {
                drop(h);
                self.focus = None;
            }
        }
        else {
            self.focus = None;
        }

        let new_active = self.hover;

        if let Some(new) = new_active {
            let mut new = new.borrow_mut(self);
            new.active = true;
            new.on_active_start();
        }

        self.active = new_active;

        self.passive_focus = NodeData::hit_test(self, self.root, x, y, |_| true);
    }

    fn on_mouse_up(&mut self) {
        if let Some(hover) = self.hover {
            if self.active == Some(hover) {
                let handler = hover.borrow(self).get_on_click();
                if let Some(handler) = handler {
                    handler(self, &mut Event { target: hover });
                }
            }
        }

        if let Some(old) = self.active {
            let mut old = old.borrow_mut(self);
            old.active = false;
            old.on_active_stop();
        }

        self.active = None;
    }

    fn on_mouse_wheel(&mut self, delta: f32, shift_down: bool) {
        let Some(hover) = self.hover else { return };

        // TEMP: ad-hoc "propagation".
        let root = self.root;
        let mut at = hover;
        loop {
            let mut n = at.borrow_mut(self);
            if n.on_mouse_wheel(delta, shift_down) || at == root {
                break;
            }
            at = n.parent.unwrap();
        }
    }

    fn set_window_size(&mut self, w: f32, h: f32) {
        let new_size = [w, h];
        if new_size == self.window_size {
            return
        }

        self.window_size = new_size;
    }

    fn paint(&mut self, rt: &ID2D1RenderTarget) {
        let [w, h] = self.window_size;

        // TEMP
        let mut root = self.root.borrow_mut(self);
        root.style(self, &Style::new());
        root.render_children(self.ctx, self);
        root.layout(self, LayoutBox::tight([(w/2.0).ceil(), h]));
        drop(root);
        self.clamp_scroll_offsets();
        let mut root = self.root.borrow_mut(self);
        root.paint(self, rt);
    }

    fn get_cursor(&self) -> Cursor {
        self.hover
        .map(|h| h.borrow(self).cursor())
        .unwrap_or(Cursor::Default)
    }

    fn root(&self) -> Node {
        self.root
    }
}


