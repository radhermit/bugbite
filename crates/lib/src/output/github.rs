use std::io::{self, Write};

use crate::objects::github::*;

use super::*;

impl Render for Issue {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: Write,
    {
        output_field_wrapped!(f, "Title", &self.title, width);
        writeln!(f, "{:<12} : {}", "ID", self.id)?;

        Ok(())
    }
}
