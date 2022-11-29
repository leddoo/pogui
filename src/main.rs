#![allow(dead_code)]

use core::cell::{Cell, RefCell};
use std::rc::Rc;

mod win;
mod unicode;
mod common;
mod ctx;
mod gui;
mod text;
mod element;


use crate::win::*;
use crate::ctx::*;
use crate::gui::*;



#[allow(dead_code)]
struct Main {
    window: HWND,

    cursor_default: HCURSOR,
    cursor_pointer: HCURSOR,
    cursor_text:    HCURSOR,

    d2d_factory: ID2D1Factory,

    rt: ID2D1HwndRenderTarget,
    rt_size: D2D_SIZE_U,

    size: [u32; 2],

    ctx: Ctx,
    gui: Gui,
}

impl Main {
    unsafe fn init(window: HWND, ctx: Ctx, gui: Gui) -> Main {
        let cursor_default = LoadCursorW(None, IDC_ARROW).unwrap();
        let cursor_pointer = LoadCursorW(None, IDC_HAND).unwrap();
        let cursor_text    = LoadCursorW(None, IDC_IBEAM).unwrap();

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
            cursor_default,
            cursor_pointer,
            cursor_text,
            window,
            d2d_factory,
            rt, rt_size,
            size,
            ctx,
            gui,
        }
    }
}

pub fn main() {
    let ctx = Ctx::new();

    let state = Rc::new(Cell::new(1));

    let mut gui = Gui::new(ctx);

    fn mk_node<C: IntoIterator<Item=Node>>(kind: NodeKind, children: C, style: &[(&str, &str)], gui: &mut Gui) -> Node {
        let node = gui.create_node(kind);
        gui.set_children(node, children.into_iter());
        gui.set_style(node, style.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        node
    }

    fn div<C: IntoIterator<Item=Node>>(children: C, style: &[(&str, &str)], gui: &mut Gui) -> Node {
        mk_node(NodeKind::Div, children, style, gui)
    }

    fn span<C: IntoIterator<Item=Node>>(children: C, style: &[(&str, &str)], gui: &mut Gui) -> Node {
        mk_node(NodeKind::Span, children, style, gui)
    }

    fn text(value: &str, gui: &mut Gui) -> Node {
        gui.create_text(value)
    }

    fn button<C: IntoIterator<Item=Node>, H: FnMut(&mut Gui, &mut Event) + 'static>(children: C, style: &[(&str, &str)], on_click: H, gui: &mut Gui) -> Node {
        let node = mk_node(NodeKind::Button, children, style, gui);
        gui.set_on_click(node, on_click);
        node
    }

    let g = &mut gui;

    let the_text = text(&state.get().to_string(), g);

    let root =
        div([
            text("hello, ", g),
            text("weirdo!", g),
            div([
                text("new line cause div", g),
                div([
                    text("div in div with inherited text color.", g),
                    div([
                        text("ADivInADivInADiv", g),
                    ], &[], g),
                ], &[
                    ("min_width", "190"),
                    ("max_width", "400"),
                    ("min_height", "70"),
                    ("max_height", "100"),
                    ("background_color", "d040a0"),
                ], g),
                div([], &[
                    ("width",  "50"),
                    ("height", "50"),
                    ("background_color", "807060"),
                ], g),
                div([
                    text("nested div with a ", g),
                    span([text("different", g)], &[
                        ("text_color", "40b040"),
                    ], g),
                    text(" text color.", g),
                ], &[
                    ("text_color", "306080"),
                ], g),
                text("more of the outer div", g),
            ], &[
                ("font_size", "69"),
                ("text_color", "802020"),
                ("background_color", "eeeeff"),
                ("min_height", "250"),
            ], g),
            div([
                text("count: ", g),
                the_text,
                text(" ", g),
                button([text("increment", g)], &[
                    ("background_color", "ffffdd"),
                ], { let state = state.clone(); move |gui: &mut Gui, _e| {
                    state.set(state.get() + 1);
                    gui.set_text(the_text, state.get().to_string());
                }}, g),
                text(" ", g),
                div([
                    div([text("hi", g)], &[], g),
                    div([text("there", g)], &[], g),
                ], &[
                    ("display", "inline"),
                    ("background_color", "ddaadd"),
                ], g),
            ], &[
                ("background_color", "ddddff"),
            ], g),
        ], &[], g);
    gui.root = Some(
        unsafe {
            let r = Rc::from_raw(root.0);
            let result = crate::element::ElementRef(r.clone());
            core::mem::forget(r);
            result
        });

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

        let main = RefCell::new(Main::init(window, ctx, gui));
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

        WM_LBUTTONDOWN => {
            main.gui.on_mouse_down();

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_LBUTTONUP => {
            main.gui.on_mouse_up();

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            let x = lo_u16(lparam.0);
            let y = hi_u16(lparam.0);

            main.gui.on_mouse_move(x as f32, y as f32);

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_SIZE => {
            let w = lo_u16(lparam.0);
            let h = hi_u16(lparam.0);

            main.gui.set_window_size(w as f32, h as f32);

            InvalidateRect(window, None, false);
            LRESULT(0)
        },

        WM_SETCURSOR => {
            let nc_hit = lo_u16(lparam.0);
            if nc_hit != HTCLIENT {
                return DefWindowProcW(window, message, wparam, lparam);
            }

            let cursor = match main.gui.get_cursor() {
                Cursor::Default => main.cursor_default,
                Cursor::Pointer => main.cursor_pointer,
                Cursor::Text    => main.cursor_text,
            };
            SetCursor(cursor);
            LRESULT(1)
        }

        WM_PAINT => {
            let mut rect = RECT::default();
            GetClientRect(main.window, &mut rect);

            let size = [
                (rect.right - rect.left) as u32,
                (rect.bottom - rect.top) as u32,
            ];

            let rt_size = D2D_SIZE_U { width: size[0], height: size[1] };
            if rt_size != main.rt_size {
                main.rt.Resize(&rt_size).unwrap();
                main.rt_size = rt_size;
            }

            let main = &mut *main;

            main.rt.BeginDraw();

            main.rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

            main.gui.set_window_size(size[0] as f32, size[1] as f32);
            main.gui.paint((&main.rt).into());

            main.rt.EndDraw(None, None).unwrap();

            ValidateRect(window, None);
            LRESULT(0)
        },

        _ => {
            drop(main);
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}

