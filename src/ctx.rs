use crate::win::*;


pub struct CtxData {
    pub dw_factory: IDWriteFactory2,
    pub dw_system_fonts:    IDWriteFontCollection,
    pub dw_system_fallback: IDWriteFontFallback,
}

pub type Ctx = &'static CtxData;

