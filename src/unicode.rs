// taken from the nightly stdlib.
// modified to be useful.


/// Mask of the value bits of a continuation byte.
pub const UTF8_CONT_MASK: u8 = 0b0011_1111;

/// Returns the initial codepoint accumulator for the first byte.
/// The first byte is special, only want bottom 5 bits for width 2, 4 bits
/// for width 3, and 3 bits for width 4.
#[inline]
pub const fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7F >> width)) as u32
}

/// Returns the value of `ch` updated with continuation byte `byte`.
#[inline]
pub const fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & UTF8_CONT_MASK) as u32
}

/// Reads the next code point out of a slice at a given index (assuming a
/// UTF-8-like encoding).
///
/// # Safety
///   - `bytes[cursor..]` must produce a valid UTF-8-like (UTF-8 or WTF-8) string.
#[inline]
pub unsafe fn utf8_next_code_point(bytes: &[u8], mut cursor: usize) -> Option<(u32, usize)> {
    if cursor >= bytes.len() {
        return None;
    }

    #[inline]
    unsafe fn next_unck(bytes: &[u8], cursor: &mut usize) -> u8 {
        let result = *bytes.get_unchecked(*cursor);
        *cursor += 1;
        result
    }

    // Decode UTF-8
    let x = next_unck(bytes, &mut cursor);
    if x < 128 {
        return Some((x as u32, cursor));
    }

    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    // NOTE: Performance is sensitive to the exact formulation here
    let init = utf8_first_byte(x, 2);
    // SAFETY: `bytes` produces an UTF-8-like string,
    // so the iterator must produce a value here.
    let y = next_unck(bytes, &mut cursor);
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        // SAFETY: `bytes` produces an UTF-8-like string,
        // so the iterator must produce a value here.
        let z = next_unck(bytes, &mut cursor);
        let y_z = utf8_acc_cont_byte((y & UTF8_CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            // SAFETY: `bytes` produces an UTF-8-like string,
            // so the iterator must produce a value here.
            let w = next_unck(bytes, &mut cursor);
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    Some((ch, cursor))
}


pub const UTF8_END_CP_1B: u32 = 0x80;
pub const UTF8_END_CP_2B: u32 = 0x800;
pub const UTF8_END_CP_3B: u32 = 0x10000;

#[inline]
pub const fn utf8_len(cp: u32) -> usize {
    if      cp < UTF8_END_CP_1B { 1 }
    else if cp < UTF8_END_CP_2B { 2 }
    else if cp < UTF8_END_CP_3B { 3 }
    else                        { 4 }
}


pub const UTF16_END_CP_1C: u32 = 0x1_0000;

#[inline]
pub const fn utf16_len(cp: u32) -> usize {
    if   cp < UTF16_END_CP_1C { 1 }
    else                      { 2 }
}


/// Encodes cp as utf16 into buffer, returning whether cp requires two u16s.
/// # Safety:
///   - requires `buffer.len() >= 2`
#[inline]
pub unsafe fn utf16_encode(mut cp: u32, buffer: &mut [u16]) -> bool {
    if cp < UTF16_END_CP_1C {
        *buffer.get_unchecked_mut(0) = cp as u16;
        false
    }
    else {
        // Supplementary planes break into surrogates.
        cp -= 0x1_0000;
        *buffer.get_unchecked_mut(0) = 0xD800 | ((cp >> 10) as u16);
        *buffer.get_unchecked_mut(1) = 0xDC00 | ((cp as u16) & 0x3FF);
        true
    }
}

