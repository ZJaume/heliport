use std::process::exit;
use log::error;

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
