use std::rc::Rc;

use crate::win::*;


pub struct CtxData {
    pub dw_factory: IDWriteFactory2,
    pub dw_system_fonts:    IDWriteFontCollection,
    pub dw_system_fallback: IDWriteFontFallback,
}

#[derive(Clone)]
pub struct Ctx (pub Rc<CtxData>);

impl core::ops::Deref for Ctx {
    type Target = CtxData;
    fn deref(&self) -> &Self::Target { &self.0 }
}

