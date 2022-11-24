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
mod fonts;
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
    preferred_x: Option<f32>,
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


        let ctx = Ctx::new();

        let default_format = TextFormat {
            font: ctx.font_query("Roboto").unwrap(),
            font_size: 36.0,
            .. Default::default()
        };

        let mut text_layout = {
            let mut b = TextLayoutBuilder::new(ctx, default_format);
            b.set_italic(true);
            b.add_string("Eng");
            b.set_italic(false);
            b.set_bold(true);
            b.add_string("lish");
            b.set_bold(false);
            b.add_line(" tea slaps.");

            b.add_line("hi there, this_is_a_super_long_word_that_is_longer_than_thirtytwo_bytes_to_test_the_break_iterator. hot damn!");
            b.add_line("just_a_single_word");

            b.set_font(ctx.font_query("Cambria").unwrap());
            b.add_line("fit â â œ̃");
            b.reset_font();
            b.add_line("");

            b.set_font(ctx.font_query("Comic Sans MS").unwrap());
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

            b.add_string("😀🧱");
            b.build()
        };
        text_layout.layout();

        Main {
            window,
            d2d_factory,
            dw_factory,
            text_layout,
            rt, rt_size,
            brush,
            cursor: 0, anchor: 0,
            preferred_x: None,
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

            self.text_layout.set_layout_width(rt_size.width as f32);
            self.text_layout.set_layout_height(rt_size.height as f32);
            // TEMP.
            self.text_layout.layout();
        }

        self.rt.BeginDraw();

        self.rt.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }));

        struct D2dTextRenderer<'a> {
            rt:    &'a ID2D1HwndRenderTarget,
            brush: &'a ID2D1SolidColorBrush,
        }

        impl<'a> TextRenderer for D2dTextRenderer<'a> {
            fn glyphs(&self, data: &DrawGlyphs) {
                let run = DWRITE_GLYPH_RUN {
                    fontFace: Some(data.font_face.clone()),
                    fontEmSize: data.format.font_size,
                    glyphCount: data.indices.len() as u32,
                    glyphIndices: data.indices.as_ptr(),
                    glyphAdvances: data.advances.as_ptr(),
                    glyphOffsets: data.offsets.as_ptr() as *const _,
                    isSideways: false.into(),
                    bidiLevel: data.is_rtl as u32,
                };

                let pos = D2D_POINT_2F {
                    x: data.pos[0],
                    y: data.pos[1],
                };
                unsafe { self.rt.DrawGlyphRun(pos, &run, self.brush, Default::default()) };
            }

            fn line(&self, data: &DrawLine, _kind: DrawLineKind) {
                let rect = D2D_RECT_F {
                    left:   data.x0,
                    top:    data.y - data.thickness/2.0,
                    right:  data.x1,
                    bottom: data.y + data.thickness/2.0,
                };
                unsafe { self.rt.FillRectangle(&rect, self.brush) };
            }

            fn object(&self, data: &DrawObject) {
                let rect = D2D_RECT_F {
                    left:   data.pos[0],
                    top:    data.pos[1] - data.baseline,
                    right:  data.pos[0] + data.size[0],
                    bottom: data.pos[1] + data.size[1] - data.baseline,
                };
                unsafe { self.rt.FillRectangle(&rect, self.brush) };
            }
        }

        // text
        let renderer = D2dTextRenderer {
            rt:    &self.rt,
            brush: &self.brush,
        };
        self.text_layout.draw([0.0, 0.0], &renderer);

        // selection
        if self.anchor != self.cursor {
            let begin = self.anchor.min(self.cursor);
            let end   = self.anchor.max(self.cursor);

            let brush = self.rt.CreateSolidColorBrush(&D2D1_COLOR_F { r: 0.3, g: 0.5, b: 0.8, a: 0.25 }, None).unwrap();

            let old_aa = self.rt.GetAntialiasMode();
            self.rt.SetAntialiasMode(D2D1_ANTIALIAS_MODE_ALIASED);

            self.text_layout.hit_test_range(begin, end, |metrics| {
                let rect = D2D_RECT_F {
                    left:   metrics.pos[0],
                    top:    metrics.pos[1],
                    right:  metrics.pos[0] + metrics.size[0],
                    bottom: metrics.pos[1] + metrics.size[1],
                };
                self.rt.FillRectangle(&rect, &brush);
            });

            self.rt.SetAntialiasMode(old_aa);
        }

        // caret
        let cursor_rect = self.cursor_rect();
        self.rt.FillRectangle(&cursor_rect, &self.brush);

        self.rt.EndDraw(None, None).unwrap();
    }

    unsafe fn cursor_rect(&mut self) -> D2D_RECT_F {
        let metrics = self.text_layout.hit_test_offset(self.cursor);

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

            let hit = main.text_layout.hit_test_pos(x as f32, y as f32);
            if hit.fraction < 0.5 {
                main.cursor = hit.text_pos_left as usize;
            }
            else {
                main.cursor = hit.text_pos_right as usize;
            }

            if !shift_down { main.anchor = main.cursor }
            main.preferred_x = None;

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
                    main.preferred_x = None;

                    InvalidateRect(window, None, false);
                }
            }
            else if key == VK_RIGHT {
                if main.cursor < main.text_layout.text().len() {
                    // TEMP: graphemes.
                    main.cursor += 1;

                    if !shift_down { main.anchor = main.cursor }
                    main.preferred_x = None;

                    InvalidateRect(window, None, false);
                }
            }
            else if key == VK_DOWN || key == VK_UP {
                let pos = main.text_layout.hit_test_offset(main.cursor);

                let mut line = pos.line_index;
                if key == VK_UP {
                    if line <= 0 {
                        return LRESULT(0);
                    }
                    line -= 1;
                }
                else {
                    if line + 1 >= main.text_layout.line_count() {
                        return LRESULT(0);
                    }
                    line += 1;
                }

                if main.preferred_x.is_none() {
                    main.preferred_x = Some(pos.x);
                }
                let query_x = main.preferred_x.unwrap();

                let hit = main.text_layout.hit_test_line(line, query_x);
                if hit.fraction < 0.5 {
                    main.cursor = hit.text_pos_left as usize;
                }
                else {
                    main.cursor = hit.text_pos_right as usize;
                }

                if !shift_down { main.anchor = main.cursor }

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

