use crate::win::*;
use crate::common::*;
use crate::element::*;


pub struct Gui {
    pub root: Option<ElementRef>,

    pub hover: Option<(ElementRef, usize)>,

    pub window_size: [f32; 2],
}

impl Gui {
    pub fn new() -> Gui {
        Gui {
            root: None,
            hover: None,
            window_size: [0.0; 2],
        }
    }

    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        if self.root.is_none() { return }

        let root = self.root.as_ref().unwrap();
        let hit = Element::hit_test(root, x, y, Element::pointer_events);
        if let Some((el, offset)) = hit {
            println!("hit {:?}, {}", el.0.as_ptr(), offset);
            self.hover = Some((el, offset));
        }
    }

    pub fn set_window_size(&mut self, w: f32, h: f32) {
        if self.root.is_none() { return }

        let new_size = [w, h];
        if new_size == self.window_size {
            return
        }

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

        self.window_size = new_size;
    }

    pub fn paint(&mut self, w: f32, h: f32, rt: &ID2D1RenderTarget) {
        if self.root.is_none() { return }

        self.set_window_size(w, h);

        let mut root = self.root.as_ref().unwrap().borrow_mut();
        root.paint(rt);
    }
}


