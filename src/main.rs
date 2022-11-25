#![allow(dead_code)]

use core::cell::RefCell;

mod win;
mod unicode;
mod common;
mod ctx;
mod gui;
mod text;
mod element;


use crate::win::*;
use crate::ctx::*;
use crate::common::Cursor;



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
}

impl Main {
    unsafe fn init(window: HWND, ctx: Ctx) -> Main {
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
        }
    }

    fn paint(&mut self) {
    }
}

pub fn main() {
    let ctx = Ctx::new();

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
            ctx.div(vec![
                ctx.text("count: 0 "),
                ctx.button(vec![ctx.text("increment")]).with_style([
                    ("background_color".into(), "ffffdd".into()),
                ].into()),
                ctx.text(" "),
                ctx.div(vec![
                    ctx.div(vec![ctx.text("hi")]),
                    ctx.div(vec![ctx.text("there")]),
                ]).with_style([
                    ("display".into(), "inline".into()),
                    ("background_color".into(), "ddaadd".into()),
                ].into()),
            ]).with_style([
                ("background_color".into(), "ddddff".into()),
            ].into()),
        ]);
    ctx.gui.borrow_mut().root = Some(root);

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

        let main = RefCell::new(Main::init(window, ctx));
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

        WM_MOUSEMOVE => {
            let x = lo_u16(lparam.0);
            let y = hi_u16(lparam.0);

            let mut gui = main.ctx.gui.borrow_mut();
            gui.on_mouse_move(x as f32, y as f32);

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_SIZE => {
            let w = lo_u16(lparam.0);
            let h = hi_u16(lparam.0);

            let mut gui = main.ctx.gui.borrow_mut();
            gui.set_window_size(w as f32, h as f32);

            InvalidateRect(window, None, false);
            LRESULT(0)
        },

        WM_SETCURSOR => {
            let gui = main.ctx.gui.borrow();

            let nc_hit = lo_u16(lparam.0);
            if nc_hit != HTCLIENT {
                return DefWindowProcW(window, message, wparam, lparam);
            }

            if let Some(hover) = gui.hover.as_ref() {
                let cursor = match hover.0.borrow().cursor() {
                    Cursor::Default => main.cursor_default,
                    Cursor::Pointer => main.cursor_pointer,
                    Cursor::Text    => main.cursor_text,
                };
                SetCursor(cursor);

                LRESULT(1)
            }
            else {
                SetCursor(main.cursor_default);
                LRESULT(1)
            }
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


            main.rt.BeginDraw();

            main.rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

            let mut gui = main.ctx.gui.borrow_mut();
            gui.paint(size[0] as f32, size[1] as f32, (&main.rt).into());

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

