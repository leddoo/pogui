use std::rc::Rc;

use crate::win::*;
use crate::common::*;
use crate::element::*;


pub struct Gui {
    pub root: Option<ElementRef>,

    pub hover:  Option<ElementRef>,
    pub active: Option<ElementRef>,

    pub window_size: [f32; 2],
}

impl Gui {
    pub fn new() -> Gui {
        Gui {
            root: None,
            hover:  None,
            active: None,
            window_size: [0.0; 2],
        }
    }

    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
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

    pub fn on_mouse_down(&mut self) {
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

    pub fn on_mouse_up(&mut self) {
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

    pub fn set_window_size(&mut self, w: f32, h: f32) {
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

    pub fn paint(&mut self, w: f32, h: f32, rt: &ID2D1RenderTarget) {
        if self.root.is_none() { return }

        self.set_window_size(w, h);

        // TEMP
        let mut root = self.root.as_ref().unwrap().borrow_mut();
        root.style(&Style::new());
        root.render_children();
        root.layout(LayoutBox::tight([w/2.0, h]));
        root.paint(rt);
    }


    pub fn get_element(&self, id: &str) -> Option<ElementRef> {
        if self.root.is_none() { return None }

        fn recurse(this: &ElementRef, id: &str) -> Option<ElementRef> {
            let me = this.borrow();
            if me.id == id {
                return Some(this.clone());
            }

            let mut at = me.first_child.clone();
            while let Some(child) = at {
                let result = recurse(&child, id);
                if result.is_some() {
                    return result;
                }

                at = child.borrow().next_sibling.clone();
            }
            None
        }
        recurse(self.root.as_ref().unwrap(), id)
    }
}


