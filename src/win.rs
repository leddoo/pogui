
pub use windows::{
    w,
    core::{HSTRING, PCWSTR},
    Foundation::Numerics::Matrix3x2,
    Win32::{
        Foundation::{HWND, WPARAM, LPARAM, RECT, LRESULT},
        Graphics::{
            Direct2D::{Common::{D2D_SIZE_U, D2D_RECT_F, D2D1_COLOR_F, D2D_POINT_2F}, ID2D1Factory, ID2D1HwndRenderTarget, D2D1CreateFactory, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES, ID2D1RenderTarget, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, ID2D1Brush, ID2D1SolidColorBrush},
            DirectWrite::{IDWriteFactory, IDWriteFactory2, DWriteCreateFactory, DWRITE_FACTORY_TYPE_SHARED, IDWriteTextLayout, DWRITE_FONT_WEIGHT_REGULAR, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, IDWriteTextFormat, DWRITE_TEXT_RANGE, IDWriteTextAnalysisSource, IDWriteTextAnalysisSink, DWRITE_READING_DIRECTION, IDWriteNumberSubstitution, IDWriteTextAnalysisSource_Impl, DWRITE_READING_DIRECTION_LEFT_TO_RIGHT, IDWriteTextAnalysisSink_Impl, DWRITE_LINE_BREAKPOINT, DWRITE_SCRIPT_ANALYSIS, DWRITE_BREAK_CONDITION, DWRITE_SHAPING_GLYPH_PROPERTIES, DWRITE_GLYPH_OFFSET, DWRITE_BREAK_CONDITION_MUST_BREAK, DWRITE_BREAK_CONDITION_CAN_BREAK, IDWriteFontCollection, IDWriteFontFallback, DWRITE_GLYPH_RUN, IDWriteFontFace, DWRITE_FONT_WEIGHT, DWRITE_FONT_STYLE_ITALIC, IDWriteFontFamily},
            Gdi::{InvalidateRect, ValidateRect},
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            WindowsAndMessaging::{WNDCLASSW, LoadIconW, IDI_APPLICATION, LoadCursorW, IDC_ARROW, RegisterClassW, WS_OVERLAPPEDWINDOW, WS_VISIBLE, CW_USEDEFAULT, CreateWindowExW, GetClientRect, SetWindowLongPtrW, GWLP_USERDATA, MSG, GetMessageW, TranslateMessage, DispatchMessageW, GetWindowLongPtrW, DefWindowProcW, PostQuitMessage, WM_LBUTTONDOWN, WM_CLOSE, WM_SIZE, WM_PAINT},
            Input::KeyboardAndMouse::{GetKeyState, VK_SHIFT}
        },
    },
};

