#![allow(dead_code)]

use core::cell::{Cell, RefCell};
use std::rc::Rc;

use pogui::win::*;
use pogui::gui::*;
use pogui::Ctx;



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

    fn button<C: IntoIterator<Item=Node>, H: Fn(&mut Gui, &mut Event) + 'static>(children: C, style: &[(&str, &str)], on_click: H, gui: &mut Gui) -> Node {
        let node = mk_node(NodeKind::Button, children, style, gui);
        gui.set_on_click(node, on_click);
        node
    }

    let g = &mut gui;

    let the_list = div([], &[], g);
    let add_button = button([text("+", g)], &[], move |gui, _e| {
        let item = div([text("something ", gui)], &[], gui);
        let button = button([text("x", gui)], &[], move |gui, _e| {
            gui.destroy_node(item);
        }, gui);
        gui.append_child(item, button);
        gui.append_child(the_list, item);
    }, g);

    let active = Rc::new(Cell::new(None));

    fn mk_button_handler(active: &Rc<Cell<Option<Node>>>, hidden: &Rc<Cell<Node>>) -> impl EventHandler {
        let active = active.clone();
        let hidden = hidden.clone();
        move |gui: &mut Gui, e: &mut Event| {
            let this = e.target;
            if let Some(mut other) = active.get() {
                if other == this {
                    other = hidden.get();
                    hidden.set(this);
                }

                gui.swap_nodes(this, other);
                gui.set_children(other, []);
                active.set(None);
            }
            else {
                let x = text("x", gui);
                gui.set_children(this, [x]);
                active.set(Some(this));
            }
        }
    }
    fn mk_button(g: &mut Gui, color: &str, handler: impl EventHandler) -> Node {
        button([], &[("background_color", color), ("width", "30"), ("height", "30"), ("display", "block")], handler, g)
    }
    let bp = mk_button(g, "ff00ff", |_: &mut Gui, _: &mut Event| {});
    let hidden = Rc::new(Cell::new(bp));
    g.set_on_click(bp, mk_button_handler(&active, &hidden));
    let br = mk_button(g, "ff0000", mk_button_handler(&active, &hidden));
    let bg = mk_button(g, "00ff00", mk_button_handler(&active, &hidden));
    let bb = mk_button(g, "0000ff", mk_button_handler(&active, &hidden));
    let bw = mk_button(g, "ffffff", mk_button_handler(&active, &hidden));

    let the_span = span([text(&state.get().to_string(), g)], &[], g);

    let nodes =
        [
            text("hello, ", g),
            text("weirdo!", g),
            br,
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
                the_span,
                text(" ", g),
                button([text("increment", g)], &[
                    ("background_color", "ffffdd"),
                ], { let state = state.clone(); move |gui, _e| {
                    state.set(state.get() + 1);
                    let new_text = text(&state.get().to_string(), gui);
                    gui.set_children(the_span, [new_text]);
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
            the_list,
            add_button,
            bb, bg, bw,
            div([text("this bish has sum phat content. she wayy too thicc to fit. an dats whai da lines be scrollin. anyway, here's some more text: The high-order word indicates the distance the wheel is rotated, expressed in multiples or divisions of WHEEL_DELTA, which is 120. A positive value indicates that the wheel was rotated forward, away from the user; a negative value indicates that the wheel was rotated backward, toward the user.", g)],
                &[("width", "300"), ("height", "100"), ("background_color", "20c0d0"), ("overflow_y", "auto")], g),
        ];
    let root = g.root();
    g.set_children(root, nodes);

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

        WM_KEYDOWN => {
            main.gui.on_key_down(wparam.0 as u32);
            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_KEYUP => {
            main.gui.on_key_up(wparam.0 as u32);
            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_CHAR => {
            let shift_down = unsafe { GetKeyState(VK_SHIFT.0 as i32) & (1u16 << 15) as i16 != 0 };

            main.gui.on_char((wparam.0 as u32).try_into().unwrap(), shift_down);

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            let x = lo_u16(lparam.0);
            let y = hi_u16(lparam.0);

            main.gui.on_mouse_down(x as f32, y as f32);

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

        WM_MOUSEWHEEL => {
            let wheel_delta = 120.0;
            let scale = 30.0;
            let delta = hi_u16(wparam.0 as isize) as u16 as i16 as i32 as f32 / wheel_delta * scale;

            const MK_SHIFT: u32 = 0x4;
            let mask = lo_u16(wparam.0 as isize);
            let shift = mask & MK_SHIFT != 0;

            main.gui.on_mouse_wheel(delta, shift);

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

