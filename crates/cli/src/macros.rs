/// Define a parsing function for a given type used with clap's derived value_parser.
macro_rules! parse_as {
    ($type:ty) => {
        |s: &str| -> Result<$type, <$type as std::str::FromStr>::Err> { s.parse() }
    };
}
pub(crate) use parse_as;

/// Wrap a docstring to remove consistent indentation from the output.
macro_rules! wrapped_doc {
    ($content:expr) => {{
        let options = textwrap::Options::new(80)
            .break_words(false)
            .word_splitter(textwrap::WordSplitter::NoHyphenation);
        textwrap::wrap(indoc::indoc!($content).trim(), &options).join("\n")
    }};
    ($content:expr, $($args:tt)*) => {{
        let options = textwrap::Options::new(80)
            .break_words(false)
            .word_splitter(textwrap::WordSplitter::NoHyphenation);
        textwrap::wrap(indoc::formatdoc!($content, $($args)*).trim(), &options).join("\n")
    }};
}
pub(crate) use wrapped_doc;
