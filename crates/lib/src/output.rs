use std::borrow::Cow;
use std::cmp::Ordering;
use std::io::{self, IsTerminal, Write};
use std::sync::{atomic::AtomicBool, LazyLock};

use crossterm::terminal;
use futures_util::{pin_mut, Stream, TryStreamExt};
use itertools::Itertools;
use serde::Serialize;
use unicode_segmentation::UnicodeSegmentation;

use crate::traits::RenderSearch;
use crate::utils::is_terminal;

mod bugzilla;
mod github;
mod redmine;

pub static COLUMNS: LazyLock<usize> = LazyLock::new(|| {
    let (cols, _rows) = terminal::size().unwrap_or((90, 24));
    // use a static width when testing is enabled
    if cfg!(feature = "test") {
        90
    } else {
        cols.into()
    }
});

// indentation for text-wrapping header field values
static INDENT: LazyLock<String> = LazyLock::new(|| " ".repeat(15));

/// Control output verbosity.
pub static VERBOSE: AtomicBool = AtomicBool::new(false);
#[macro_export]
macro_rules! verbose {
    ($dst:expr, $($arg:tt)+) => {
        if $crate::output::VERBOSE.load(std::sync::atomic::Ordering::Acquire) {
            writeln!($dst, $($arg)+)
        } else {
            Ok(())
        }
    };
    ($enable:expr) => {
        $crate::output::VERBOSE.store($enable, std::sync::atomic::Ordering::SeqCst);
    };
    () => {
        $crate::output::VERBOSE.load(std::sync::atomic::Ordering::Acquire)
    };
}
pub use verbose;

/// Render an item for output to the terminal.
pub trait Render {
    fn render<W: Write>(&self, f: &mut W, width: usize) -> io::Result<()>;
}

/// Implement std::fmt::Display trait for given types using the Render trait.
#[macro_export]
macro_rules! impl_render_display {
    ($($type:ty),+) => {$(
        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                let mut buf = vec![];
                Render::render(self, &mut buf, *COLUMNS).unwrap();
                let s = String::from_utf8(buf).unwrap();
                s.fmt(f)
            }
        }
    )+};
}
use impl_render_display;

/// Truncate a string to the requested width of graphemes.
fn truncate(data: &str, width: usize) -> Cow<'_, str> {
    if data.len() > width {
        let mut iter = UnicodeSegmentation::graphemes(data, true).take(*COLUMNS);
        Cow::Owned(iter.join(""))
    } else {
        Cow::Borrowed(data)
    }
}

/// Output an iterable field in truncated list format.
fn truncated_list<W, I>(f: &mut W, name: &str, data: I, width: usize) -> io::Result<()>
where
    W: Write,
    I: IntoIterator,
    I::IntoIter: ExactSizeIterator,
    I::Item: std::fmt::Display,
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

/// Output an iterable field in wrapped CSV format.
fn wrapped_csv<I, W, S>(f: &mut W, name: &str, data: I, width: usize) -> io::Result<()>
where
    I: IntoIterator<Item = S>,
    W: Write,
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

macro_rules! output_field {
    ($f:expr, $name:expr, $value:expr, $width:expr) => {
        if let Some(value) = $value {
            let line = format!("{:<12} : {value}", $name);
            let data = $crate::output::truncate(&line, $width);
            writeln!($f, "{data}")?;
        }
    };
}
use output_field;

macro_rules! output_field_wrapped {
    ($f:expr, $name:expr, $value:expr, $width:expr) => {
        if let Some(value) = $value {
            let options =
                textwrap::Options::new($width - 15).subsequent_indent(&$crate::output::INDENT);
            let mut wrapped = textwrap::wrap(value, &options).into_iter();
            let data = itertools::Itertools::join(&mut wrapped, "\n");
            writeln!($f, "{:<12} : {data}", $name)?;
        }
    };
}
use output_field_wrapped;

pub async fn render_search<I, V, T, W>(
    f: &mut W,
    items: I,
    fields: &[T],
    json: bool,
) -> crate::Result<()>
where
    I: Stream<Item = crate::Result<V>>,
    V: RenderSearch<T> + Serialize,
    W: Write,
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
                let data = truncate(&line, *COLUMNS);
                writeln!(f, "{data}")?;
            }
        }
    }

    if count > 0 {
        verbose!(f, " * {count} found")?;
    }

    Ok(())
}

pub fn render_items<'a, I, T, W>(f: &mut W, items: I) -> crate::Result<()>
where
    I: IntoIterator<Item = &'a T>,
    T: Render + 'a,
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
        item.render(f, width)?;
    }

    Ok(())
}
