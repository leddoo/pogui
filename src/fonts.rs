use crate::win::PCWSTR;
use crate::ctx::Ctx;


#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FontFamilyId (pub u32);

impl FontFamilyId {
    pub const DEFAULT: FontFamilyId = FontFamilyId(0);
}

impl Default for FontFamilyId {
    #[inline]
    fn default() -> Self { Self::DEFAULT }
}


pub struct Fonts {
    families: Vec<FontFamilyData>,
}

struct FontFamilyData {
    name_utf8:  String,
    name_utf16: Vec<u16>,
}


impl Fonts {
    pub fn new() -> Fonts {
        Fonts { families: vec![] }
    }

    pub fn query(&mut self, name: &str, ctx: Ctx) -> Option<FontFamilyId> {
        for (i, family) in self.families.iter().enumerate() {
            if family.name_utf8 == name {
                return Some(FontFamilyId(i as u32));
            }
        }

        let name_utf16 = {
            let mut utf16: Vec<u16> = name.encode_utf16().collect();
            utf16.push(0);
            utf16
        };

        let (mut index, mut exists) = Default::default();
        unsafe { ctx.dw_system_fonts.FindFamilyName(PCWSTR(name_utf16.as_ptr()), &mut index, &mut exists).ok()? }
        if exists.as_bool() {
            let id = self.families.len() as u32;
            self.families.push(FontFamilyData {
                name_utf8: name.into(),
                name_utf16,
            });
            return Some(FontFamilyId(id));
        }

        None
    }

    pub fn font_name_utf16(&self, id: FontFamilyId) -> &[u16] {
        &self.families[id.0 as usize].name_utf16
    }
}

