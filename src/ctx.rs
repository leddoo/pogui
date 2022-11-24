use core::cell::RefCell;

use crate::win::*;
use crate::text::*;


pub struct CtxData {
    pub dw_factory: IDWriteFactory2,
    pub dw_system_fonts:    IDWriteFontCollection,
    pub dw_system_fallback: IDWriteFontFallback,

    pub fonts: RefCell<Fonts>,
}


#[derive(Clone, Copy)]
pub struct Ctx (pub &'static CtxData);

impl Ctx {
    pub fn new() -> Ctx {unsafe {
        let dw_factory: IDWriteFactory2 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();

        let mut dw_system_fonts = None;
        dw_factory.GetSystemFontCollection(&mut dw_system_fonts, false).unwrap();

        let ctx = Ctx(Box::leak(Box::new(CtxData {
            dw_factory: dw_factory.clone(),
            dw_system_fonts:    dw_system_fonts.unwrap(),
            dw_system_fallback: dw_factory.GetSystemFontFallback().unwrap(),

            fonts: RefCell::new(Fonts::new()),
        })));

        // TODO: how to set up default font?
        ctx.fonts.borrow_mut().query("Tahoma", ctx).unwrap();

        ctx
    }}


    #[inline]
    pub fn font_query(self, family_name: &str) -> Option<FontFamilyId> {
        self.fonts.borrow_mut().query(family_name, self)
    }
}

impl core::ops::Deref for Ctx {
    type Target = CtxData;
    #[inline] fn deref(&self) -> &Self::Target { &self.0 }
}

