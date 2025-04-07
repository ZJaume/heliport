use std::process::exit;

use lazy_static::lazy_static;
use log::error;
use regex::Regex;
use unicode_blocks;

lazy_static! {
    pub static ref RE_NON_ALPHA: Regex = Regex::new(r#"[^#gc\p{L}\p{M}′'’´ʹािीुूृेैोौंँः् া ি ী ু ূ ৃ ে ৈ ো ৌ।্্্я̄\u07A6\u07A7\u07A8\u07A9\u07AA\u07AB\u07AC\u07AD\u07AE\u07AF\u07B0\u0A81\u0A82\u0A83\u0ABC\u0ABD\u0ABE\u0ABF\u0AC0\u0AC1\u0AC2\u0AC3\u0AC4\u0AC5\u0AC6\u0AC7\u0AC8\u0AC9\u0ACA\u0ACB\u0ACC\u0ACD\u0AD0\u0AE0\u0AE1\u0AE2\u0AE3\u0AE4\u0AE5\u0AE6\u0AE7\u0AE8\u0AE9\u0AEA\u0AEB\u0AEC\u0AED\u0AEE\u0AEF\u0AF0\u0AF1]"#)
            .expect("Error compiling non-alpha regex for Idenfifier");
}

// Trait that extracts the contained ok value or aborts if error
// sending the error message to the log
pub trait Abort<T> {
    fn or_abort(self, exit_code: i32) -> T;
}

impl<T, E: std::fmt::Display> Abort<T> for Result<T, E> {
    fn or_abort(self, exit_code: i32) -> T {
        match self {
            Ok(v) => v,
            // Print the whole error context with :#
            Err(e) => {
                error!("{e:#}");
                exit(exit_code);
            }
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
        None => {
            return Err(());
        }
    };

    for i in 0..CJK_BLOCKS.len() {
        if CJK_BLOCKS[i] == charset {
            return Ok(true);
        }
    }
    Ok(false)
}
