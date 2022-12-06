use core::cell::RefCell;
use crate::win::*;

use crate::ctx::Ctx;
use crate::common::Cursor;
use crate::gui::{Gui, IGui};


pub struct NativeGui {
    data: Box<RefCell<NativeGuiData>>,
}

struct NativeGuiData {
    gui: Gui,

    cursor_default: HCURSOR,
    cursor_pointer: HCURSOR,
    cursor_text:    HCURSOR,

    #[allow(dead_code)]
    d2d_factory: ID2D1Factory,

    rt: ID2D1HwndRenderTarget,
    rt_size: D2D_SIZE_U,
}

impl NativeGui {
    pub fn new() -> NativeGui {unsafe {
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


        // TEMP
        let ctx = Ctx::new();
        let gui = Gui::new(ctx);

        let data = Box::new(RefCell::new(NativeGuiData {
            gui,

            cursor_default,
            cursor_pointer,
            cursor_text,
            d2d_factory,
            rt, rt_size,
        }));

        SetWindowLongPtrW(window, GWLP_USERDATA, data.as_ptr() as isize);

        NativeGui { data }
    }}

    #[inline]
    pub fn with_gui<R, F: FnOnce(&mut Gui) -> R>(&mut self, f: F) -> R {
        f(&mut self.data.borrow_mut().gui)
    }

    pub fn run_message_loop(&mut self) {
        std::panic::set_hook(Box::new(|info| {
            println!("panic: {}", info);
            loop {}
        }));



        // event loop.
        loop {unsafe {
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
        }}
    }
}


unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    fn lo_u16(a: isize) -> u32 { (a as usize as u32) & 0xffff }
    fn hi_u16(a: isize) -> u32 { ((a as usize as u32) >> 16) & 0xffff }

    let mut data = {
        let data = GetWindowLongPtrW(window, GWLP_USERDATA) as *const RefCell<NativeGuiData>;
        if data == core::ptr::null() {
            return DefWindowProcW(window, message, wparam, lparam);
        }

        (*data).borrow_mut()
    };

    let message = message as u32;
    match message {
        WM_CLOSE => {
            PostQuitMessage(0);
            LRESULT(0)
        },

        WM_KEYDOWN => {
            data.gui.on_key_down(wparam.0 as u32);
            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_KEYUP => {
            data.gui.on_key_up(wparam.0 as u32);
            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_CHAR => {
            let shift_down = unsafe { GetKeyState(VK_SHIFT.0 as i32) & (1u16 << 15) as i16 != 0 };

            data.gui.on_char((wparam.0 as u32).try_into().unwrap(), shift_down);

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_LBUTTONDOWN => {
            let x = lo_u16(lparam.0);
            let y = hi_u16(lparam.0);

            data.gui.on_mouse_down(x as f32, y as f32);

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_LBUTTONUP => {
            data.gui.on_mouse_up();

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_MOUSEMOVE => {
            let x = lo_u16(lparam.0);
            let y = hi_u16(lparam.0);

            data.gui.on_mouse_move(x as f32, y as f32);

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

            data.gui.on_mouse_wheel(delta, shift);

            InvalidateRect(window, None, false);
            LRESULT(0)
        }

        WM_SIZE => {
            let w = lo_u16(lparam.0);
            let h = hi_u16(lparam.0);

            data.gui.set_window_size(w as f32, h as f32);

            InvalidateRect(window, None, false);
            LRESULT(0)
        },

        WM_SETCURSOR => {
            let nc_hit = lo_u16(lparam.0);
            if nc_hit != HTCLIENT {
                return DefWindowProcW(window, message, wparam, lparam);
            }

            let cursor = match data.gui.get_cursor() {
                Cursor::Default => data.cursor_default,
                Cursor::Pointer => data.cursor_pointer,
                Cursor::Text    => data.cursor_text,
            };
            SetCursor(cursor);
            LRESULT(1)
        }

        WM_PAINT => {
            let mut rect = RECT::default();
            GetClientRect(window, &mut rect);

            let size = [
                (rect.right - rect.left) as u32,
                (rect.bottom - rect.top) as u32,
            ];

            let rt_size = D2D_SIZE_U { width: size[0], height: size[1] };
            if rt_size != data.rt_size {
                data.rt.Resize(&rt_size).unwrap();
                data.rt_size = rt_size;
            }

            let data = &mut *data;

            data.rt.BeginDraw();

            data.rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

            data.gui.set_window_size(size[0] as f32, size[1] as f32);
            data.gui.paint((&data.rt).into());

            data.rt.EndDraw(None, None).unwrap();

            ValidateRect(window, None);
            LRESULT(0)
        },

        _ => {
            drop(data);
            DefWindowProcW(window, message, wparam, lparam)
        }
    }
}

