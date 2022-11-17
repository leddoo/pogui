#![allow(unused_imports)]

use std::ffi::c_void;
use std::cell::RefCell;

use windows::{
    w,
    core::{
        PCSTR,
        PCWSTR,
        HSTRING,
    },
    Win32::{
        Foundation::{
            BOOL,
            HANDLE,
            HWND,
            LRESULT,
            LPARAM, WPARAM,
            RECT,

            WAIT_OBJECT_0,
        },
        Graphics::{
            Gdi::*,
            Direct2D::*,
            Direct2D::Common::*,
            DirectWrite::*,
            Dxgi::*,
            Dxgi::Common::*,
        },
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{
                CreateThread,
                CreateSemaphoreW,
                WaitForSingleObject,
                ReleaseSemaphore,
            },
        },
        UI::{
            WindowsAndMessaging::*,
            Input::KeyboardAndMouse::*,
        },
    },
};



mod unicode;
mod win;
mod ctx;
mod gui;
mod gui_main;


use std::rc::Rc;

use ctx::*;
use gui::text_layout::*;


fn main() {
    if 1==1 {
        unsafe { _main() }
    }
    else {
        gui_main::main()
    }
}


#[allow(dead_code)]
struct Main {
    window: HWND,
    d2d_factory: ID2D1Factory,
    dw_factory: IDWriteFactory2,

    rt: ID2D1HwndRenderTarget,
    rt_size: D2D_SIZE_U,
    brush: ID2D1SolidColorBrush,

    text_layout: TextLayout,

    cursor: usize,
    anchor: usize,
}

impl Main {
    unsafe fn init(window: HWND) -> Main {
        let d2d_factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();

        let dw_factory: IDWriteFactory2 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();


        let mut rect = RECT::default();
        GetClientRect(window, &mut rect);
        let rt_size = D2D_SIZE_U {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        };

        let rt = d2d_factory.CreateHwndRenderTarget(
            &Default::default(),
            &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: window,
                pixelSize: rt_size,
                ..Default::default()
            }).unwrap();

        let brush = rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }, None).unwrap();


        let mut dw_system_fonts = None;
        dw_factory.GetSystemFontCollection(&mut dw_system_fonts, false).unwrap();

        let ctx = Ctx(Rc::new(CtxData { 
            dw_factory: dw_factory.clone(),
            dw_system_fonts:    dw_system_fonts.unwrap(),
            dw_system_fallback: dw_factory.GetSystemFontFallback().unwrap(),
        }));

        let default_format = TextFormat {
            font: "Roboto",
            font_size: 36.0,
            .. Default::default()
        };

        let text_layout = {
            let mut b = TextLayoutBuilder::new(&ctx, default_format);
            b.set_font("Cambria");
            b.add_line("fit â â œ̃");
            b.reset_font();
            b.add_line("");

            b.set_font("Comic Sans MS");
            b.set_font_size(24.0);
            b.set_strikethrough(true);
            b.add_string("insaneo style");
            b.reset_format();
            b.add_string(" ");
            b.add_object_ex([48.0, 64.0], 56.0);
            b.add_string(" ");
            b.set_underline(true);
            b.add_line("bunko town!");
            b.set_underline(false);

            b.set_italic(true);
            b.add_string("Eng");
            b.set_italic(false);
            b.set_bold(true);
            b.add_string("lish");
            b.set_bold(false);
            b.add_line(" العربية 中国");

            b.add_line("😀🧱");
            b.build()
        };

        let cursor = 0;
        let anchor = cursor;

        Main {
            window,
            d2d_factory,
            dw_factory,
            text_layout,
            rt, rt_size,
            brush,
            cursor, anchor,
        }
    }

    unsafe fn paint(&mut self) {
        let mut rect = RECT::default();
        GetClientRect(self.window, &mut rect);

        let rt_size = D2D_SIZE_U {
            width: (rect.right - rect.left) as u32,
            height: (rect.bottom - rect.top) as u32,
        };
        if rt_size != self.rt_size {
            self.rt.Resize(&rt_size).unwrap();
            self.rt_size = rt_size;

            // TEMP
            //self.text_layout.SetMaxWidth(rt_size.width as f32).unwrap();
            //self.text_layout.SetMaxHeight(rt_size.height as f32).unwrap();
        }

        self.rt.BeginDraw();

        self.rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

        // text
        self.text_layout.draw([rect.left as f32, rect.top as f32], (&self.rt).into(), (&self.brush).into());

        // selection
        // TEMP
        /*
        if self.anchor != self.cursor {
            let begin = self.anchor.min(self.cursor);
            let end   = self.anchor.max(self.cursor);

            let mut rect_count = 0;
            drop(self.text_layout.HitTestTextRange(
                begin as u32, (end - begin) as u32,
                pos.x, pos.y,
                None,
                &mut rect_count));

            let mut metrics = vec![Default::default(); rect_count as usize];
            self.text_layout.HitTestTextRange(
                begin as u32, (end - begin) as u32,
                pos.x, pos.y,
                Some(&mut metrics),
                &mut rect_count).unwrap();

            let brush = self.rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 0.3, g: 0.5, b: 0.8, a: 0.25 }, None).unwrap();

            let old_aa = self.rt.GetAntialiasMode();
            self.rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED);
            for metrics in metrics.iter() {
                let rect = D2D_RECT_F {
                    left:   metrics.left,
                    top:    metrics.top,
                    right:  metrics.left + metrics.width,
                    bottom: metrics.top + metrics.height,
                };
                self.rt.FillRectangle(&rect, &brush);
            }
            self.rt.SetAntialiasMode(old_aa);
        }
        */

        // caret
        let cursor_rect = self.cursor_rect();
        self.rt.FillRectangle(&cursor_rect, &self.brush);

        self.rt.EndDraw(None, None).unwrap();
    }

    unsafe fn cursor_rect(&mut self) -> D2D_RECT_F {
        let metrics = self.text_layout.offset_to_pos(self.cursor);

        let x = metrics.x;
        let y = metrics.y;
        let w = 2.0;
        let h = metrics.line_height;

        let left   = (x - w/2.0).floor();
        let right  = left + w;
        let top    = y.floor();
        let bottom = top + h;
        D2D_RECT_F { left, top, right, bottom }
    }

    // TEMP
    /*

    unsafe fn pos_from_coord(&mut self, x: f32, y: f32) -> usize {
        let (mut trailing, mut inside, mut metrics) = Default::default();
        self.text_layout.HitTestPoint(x, y, &mut trailing, &mut inside, &mut metrics).unwrap();

        let pos = metrics.textPosition as usize;
        let offset = if trailing.as_bool() { metrics.length as usize } else { 0 };
        pos + offset
    }

    unsafe fn delete_selection(&mut self) {
        let begin = self.anchor.min(self.cursor);
        let end   = self.anchor.max(self.cursor);
        self.text.drain(begin..end);
        self.cursor = begin;
        self.anchor = begin;

        let old_layout = &self.text_layout;
        let new_layout = self.dw_factory.CreateTextLayout(
            &self.text,
            old_layout,
            old_layout.GetMaxWidth(),
            old_layout.GetMaxHeight()).unwrap();
        new_layout.SetTextAlignment(old_layout.GetTextAlignment()).unwrap();
        new_layout.SetParagraphAlignment(old_layout.GetParagraphAlignment()).unwrap();
        self.text_layout = new_layout;
    }
    */
}


unsafe fn _main() {
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

    let main = RefCell::new(Main::init(window));
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


unsafe extern "system" fn window_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    fn low_u16(a: isize) -> u32 {
        (a as usize as u32) & 0xffff
    }

    fn high_u16(a: isize) -> u32 {
        ((a as usize as u32) >> 16) & 0xffff
    }

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
            //let x = low_u16(lparam.0);
            //let y = high_u16(lparam.0);
            LRESULT(0)
        },

        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
            let x = low_u16(lparam.0);
            let y = high_u16(lparam.0);
            let shift_down = (GetKeyState(VK_SHIFT.0 as i32) & 0x80) != 0;

            // TEMP
            let _ = (x, y);
            //main.cursor = main.pos_from_coord(x as f32, y as f32);
            if !shift_down { main.anchor = main.cursor }
            InvalidateRect(window, None, false);
            LRESULT(0)
        },

        WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => {
            //let x = low_u16(lparam.0);
            //let y = high_u16(lparam.0);
            LRESULT(0)
        },

        WM_MOUSEWHEEL => {
            //let delta = high_u16(wparam.0 as isize) as i16 as i32 / 120;
            LRESULT(0)
        }

        WM_KEYDOWN => {
            let key = VIRTUAL_KEY(wparam.0 as usize as u16);

            let shift_down = (GetKeyState(VK_SHIFT.0 as i32) & 0x80) != 0;

            if key == VK_LEFT {
                if main.cursor > 0 {
                    // TEMP: graphemes.
                    main.cursor -= 1;
                    if !shift_down { main.anchor = main.cursor }
                    InvalidateRect(window, None, false);
                }
            }
            else if key == VK_RIGHT {
                if main.cursor < main.text_layout.text().len() {
                    // TEMP: graphemes.
                    main.cursor += 1;
                    if !shift_down { main.anchor = main.cursor }
                    InvalidateRect(window, None, false);
                }
            }
            else if key == VK_DOWN || key == VK_UP {
                // TEMP
                /*
                let pos = main.cursor;
                let lines = main.line_metrics();

                let (line, mut line_pos) = main.line_from_pos(&lines, pos);
                if key == VK_UP {
                    if line <= 0 {
                        return LRESULT(0);
                    }
                    line_pos -= lines[line - 1].length as usize;
                }
                else {
                    if line >= lines.len() - 1 {
                        return LRESULT(0);
                    }
                    line_pos += lines[line].length as usize;
                }

                let old_x = main.coord_from_pos(pos).0;
                let new_y = main.coord_from_pos(line_pos).1;

                main.cursor = main.pos_from_coord(old_x, new_y);
                if !shift_down { main.anchor = main.cursor }
                */
                InvalidateRect(window, None, false);
            }
            else if key == VK_BACK || key == VK_DELETE {
                if main.anchor != main.cursor {
                    // TEMP
                    //main.delete_selection();
                    InvalidateRect(window, None, false);
                }
            }

            LRESULT(0)
        },

        WM_KEYUP => {
            //let key = VIRTUAL_KEY(wparam.0 as usize as u16);
            LRESULT(0)
        },

        WM_CHAR => {
            //let chr = wparam.0 as usize as u16;
            LRESULT(0)
        },

        WM_SIZE => {
            //let w = low_u16(lparam.0);
            //let h = high_u16(lparam.0);
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

