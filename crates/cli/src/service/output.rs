use std::cmp::Ordering;
use std::io::{self, IsTerminal, Write};

use bugbite::traits::RenderSearch;
use bugbite::utils::is_terminal;
use futures::{
    pin_mut,
    stream::{Stream, TryStreamExt},
};
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::service::Render;
use crate::utils::{truncate, verbose, COLUMNS};

// indentation for text-wrapping header field values
pub(crate) static INDENT: Lazy<String> = Lazy::new(|| " ".repeat(15));

/// Output an iterable field in wrapped CSV format.
pub(crate) fn wrapped_csv<I, W, S>(f: &mut W, name: &str, data: I, width: usize) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    W: IsTerminal + Write,
    S: std::fmt::Display,
{
    let rendered = data.into_iter().join(", ");
    if !rendered.is_empty() {
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

pub(crate) async fn render_search<I, V, T, W>(
    f: &mut W,
    items: I,
    fields: &[T],
    json: bool,
) -> Result<(), bugbite::Error>
where
    I: Stream<Item = bugbite::Result<V>>,
    V: RenderSearch<T> + Serialize,
    W: IsTerminal + Write,
{
    let mut count = 0;

    pin_mut!(items);
    while let Some(item) = items.try_next().await? {
        count += 1;
        if json {
            let data = serde_json::to_string(&item).expect("failed serializing item");
            writeln!(f, "{data}")?;
        } else {
            let line = item.render(fields);
            if !line.is_empty() {
                let data = truncate(f, &line, *COLUMNS);
                writeln!(f, "{data}")?;
            }
        }
    }

    if count > 0 {
        verbose!(f, " * {count} found")?;
    }

    Ok(())
}

pub(crate) fn render_items<I, S, T, W>(
    f: &mut W,
    service: &S,
    items: I,
) -> Result<(), bugbite::Error>
where
    I: IntoIterator<Item = T>,
    S: Render<T>,
    W: IsTerminal + Write,
{
    // text wrap width
    let width = if is_terminal!(f) && *COLUMNS <= 90 && *COLUMNS >= 50 {
        *COLUMNS
    } else {
        90
    };

    for item in items {
        writeln!(f, "{}", "=".repeat(width))?;
        service.render(item, f, width)?;
    }

    Ok(())
}

/// Output an iterable field in truncated list format.
pub(crate) fn truncated_list<W, I, S>(
    f: &mut W,
    name: &str,
    data: I,
    width: usize,
) -> io::Result<()>
where
    W: IsTerminal + Write,
    I: IntoIterator<Item = S>,
    <I as IntoIterator>::IntoIter: ExactSizeIterator,
    S: std::fmt::Display,
{
    let mut values = data.into_iter();
    match values.len().cmp(&1) {
        Ordering::Equal => {
            let value = values.next().unwrap();
            let line = format!("{name:<12} : {value}");
            writeln!(f, "{}", truncate(f, &line, width))?;
        }
        Ordering::Greater => {
            writeln!(f, "{name:<12} :")?;
            for value in values {
                let line = format!("  {value}");
                writeln!(f, "{}", truncate(f, &line, width))?;
            }
        }
        Ordering::Less => (),
    }

    Ok(())
}

macro_rules! output_field {
    ($fmt:expr, $name:expr, $value:expr, $width:expr) => {
        if let Some(value) = $value {
            let line = format!("{:<12} : {value}", $name);
            let data = $crate::utils::truncate($fmt, &line, $width);
            writeln!($fmt, "{data}")?;
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
