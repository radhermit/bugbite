use std::cmp::Ordering;
use std::io::{stdout, IsTerminal, Write};

use bugbite::traits::RenderSearch;
use itertools::Itertools;
use once_cell::sync::Lazy;
use serde::Serialize;
use tracing::info;

use crate::service::Render;
use crate::utils::{truncate, COLUMNS};

// indentation for text-wrapping header field values
pub(crate) static INDENT: Lazy<String> = Lazy::new(|| " ".repeat(15));

/// Output an iterable field in wrapped CSV format.
pub(crate) fn wrapped_csv<I, W, S>(
    f: &mut W,
    name: &str,
    data: I,
    width: usize,
) -> std::io::Result<()>
where
    I: IntoIterator<Item = S>,
    W: std::io::Write,
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

pub(crate) fn render_search<I, V, T, W>(
    mut f: W,
    items: I,
    fields: &[T],
    json: bool,
) -> Result<(), bugbite::Error>
where
    I: IntoIterator<Item = V>,
    V: RenderSearch<T> + Serialize,
    W: std::io::Write,
{
    let mut count = 0;

    for item in items {
        count += 1;
        if json {
            let data = serde_json::to_string(&item).expect("failed serializing item");
            writeln!(f, "{data}")?;
        } else {
            let line = item.render(fields);
            if !line.is_empty() {
                let data = truncate(&line, *COLUMNS);
                writeln!(f, "{data}")?;
            }
        }
    }

    if count > 0 {
        info!(" * {count} found");
    }

    Ok(())
}

pub(crate) fn render_items<I, R>(items: I) -> Result<(), bugbite::Error>
where
    I: IntoIterator<Item = R>,
    R: Render,
{
    let mut stdout = stdout().lock();

    // text wrap width
    let width = if stdout.is_terminal() && *COLUMNS <= 90 && *COLUMNS >= 50 {
        *COLUMNS
    } else {
        90
    };

    for item in items {
        writeln!(stdout, "{}", "=".repeat(width))?;
        item.render(&mut stdout, width)?;
    }

    Ok(())
}

/// Output an iterable field in truncated list format.
pub(crate) fn truncated_list<W, I, S>(
    f: &mut W,
    name: &str,
    data: I,
    width: usize,
) -> std::io::Result<()>
where
    W: std::io::Write,
    I: IntoIterator<Item = S>,
    <I as IntoIterator>::IntoIter: ExactSizeIterator,
    S: std::fmt::Display,
{
    let mut values = data.into_iter();
    match values.len().cmp(&1) {
        Ordering::Equal => {
            let value = values.next().unwrap();
            let line = format!("{name:<12} : {value}");
            writeln!(f, "{}", truncate(&line, width))?;
        }
        Ordering::Greater => {
            writeln!(f, "{name:<12} :")?;
            for value in values {
                let line = format!("  {value}");
                writeln!(f, "{}", truncate(&line, width))?;
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
            let data = $crate::utils::truncate(&line, $width);
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
