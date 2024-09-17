use std::process::exit;

use log::error;
use unicode_blocks;

// Trait that extracts the contained ok value or aborts if error
// sending the error message to the log
pub trait Abort<T> {
    fn or_abort(self, exit_code: i32) -> T;
}

impl<T, E: std::fmt::Display> Abort<T> for Result<T, E>
{
    fn or_abort(self, exit_code: i32) -> T {
        match self {
            Ok(v) => v,
            Err(e) => { error!("{e}"); exit(exit_code); },
        }
    }
}

const CJK_BLOCKS: [unicode_blocks::UnicodeBlock; 17] = [
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_A,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_B,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_C,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_D,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_E,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_F,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_G,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_H,
    unicode_blocks::CJK_UNIFIED_IDEOGRAPHS_EXTENSION_I,
    unicode_blocks::CJK_COMPATIBILITY,
    unicode_blocks::CJK_COMPATIBILITY_FORMS,
    unicode_blocks::CJK_COMPATIBILITY_IDEOGRAPHS,
    unicode_blocks::CJK_COMPATIBILITY_IDEOGRAPHS_SUPPLEMENT,
    unicode_blocks::CJK_RADICALS_SUPPLEMENT,
    unicode_blocks::CJK_STROKES,
    unicode_blocks::CJK_SYMBOLS_AND_PUNCTUATION,
];

/// Return if char belongs to CJK_* unicode blocks
///
/// Beware that this will not return true for Hangul or Kana, since they are
/// in a different block. The CJK_* includes the unified ideographs and common
/// background of the three languages, not the current.
pub fn is_cjk_block(c: char) -> Result<bool, ()> {
    let charset = match unicode_blocks::find_unicode_block(c) {
        Some(charset) => charset,
        None => { return Err(()); }
    };

    for i in 0..CJK_BLOCKS.len() {
        if CJK_BLOCKS[i] == charset {
            return Ok(true);
        }
    }
    Ok(false)
}
