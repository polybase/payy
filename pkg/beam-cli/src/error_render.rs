use std::error::Error as StdError;

use crate::error::Error;

pub(crate) fn format_error_chain(err: &Error) -> String {
    let mut message = err.to_string();
    let mut source = StdError::source(err);
    while let Some(cause) = source {
        let cause_message = cause.to_string();
        if !cause_message.is_empty() {
            message.push_str("; caused by: ");
            message.push_str(&cause_message);
        }
        source = cause.source();
    }
    message
}
