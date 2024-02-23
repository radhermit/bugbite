use itertools::Itertools;
use once_cell::sync::Lazy;

use crate::utils::truncate;

// indentation for text-wrapping header field values
pub(crate) static INDENT: Lazy<String> = Lazy::new(|| " ".repeat(15));

/// Output an iterable field in wrapped CSV format.
pub(crate) fn wrapped_csv<W, S>(
    f: &mut W,
    name: &str,
    data: &[S],
    width: usize,
) -> std::io::Result<()>
where
    W: std::io::Write,
    S: std::fmt::Display,
{
    if !data.is_empty() {
        let rendered = data.iter().join(", ");
        if rendered.len() + 15 <= width {
            writeln!(f, "{name:<12} : {rendered}")?;
        } else {
            let options = textwrap::Options::new(width - 15).subsequent_indent(&INDENT);
            let wrapped = textwrap::wrap(&rendered, &options);
            writeln!(f, "{name:<12} : {}", wrapped.iter().join("\n"))?;
        }
    }
    Ok(())
}

/// Output an iterable field in truncated list format.
pub(crate) fn truncated_list<W, S>(
    f: &mut W,
    name: &str,
    data: &[S],
    width: usize,
) -> std::io::Result<()>
where
    W: std::io::Write,
    S: AsRef<str>,
{
    match data {
        [] => Ok(()),
        [value] => writeln!(f, "{name:<12} : {}", truncate(value.as_ref(), width - 15)),
        values => {
            let list = values
                .iter()
                .map(|s| truncate(s.as_ref(), width - 2))
                .join("\n  ");
            writeln!(f, "{name:<12} :\n  {list}")
        }
    }
}

macro_rules! output_field {
    ($fmt:expr, $name:expr, $value:expr) => {
        if let Some(value) = $value {
            writeln!($fmt, "{:<12} : {value}", $name)?;
        }
    };
}
pub(crate) use output_field;

macro_rules! output_field_wrapped {
    ($fmt:expr, $name:expr, $value:expr, $width:expr) => {
        if let Some(value) = $value {
            let options = textwrap::Options::new($width - 15)
                .subsequent_indent(&$crate::service::output::INDENT);
            let wrapped = textwrap::wrap(value, &options);
            let data = wrapped.iter().join("\n");
            writeln!($fmt, "{:<12} : {data}", $name)?;
        }
    };
}
pub(crate) use output_field_wrapped;
