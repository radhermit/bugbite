use std::io::{self, Write};

use crate::objects::redmine::*;

use super::*;

impl Render for Comment {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: Write,
    {
        if self.count != 0 {
            write!(f, "Comment #{} ", self.count)?;
        } else {
            write!(f, "Description ")?;
        }
        writeln!(f, "by {}, {}", self.user, self.created)?;
        writeln!(f, "{}", "-".repeat(width))?;
        // wrap comment text
        let wrapped = textwrap::wrap(self.text.trim(), width);
        writeln!(f, "{}", wrapped.iter().join("\n"))
    }
}

impl Render for Issue {
    fn render<W>(&self, f: &mut W, width: usize) -> io::Result<()>
    where
        W: Write,
    {
        output_field_wrapped!(f, "Subject", &self.subject, width);
        output_field!(f, "Assignee", &self.assigned_to, width);
        output_field!(f, "Reporter", &self.author, width);
        output_field!(f, "Status", &self.status, width);
        output_field!(f, "Tracker", &self.tracker, width);
        output_field!(f, "Priority", &self.priority, width);
        output_field!(f, "Closed", &self.closed, width);
        output_field!(f, "Created", &self.created, width);
        output_field!(f, "Updated", &self.updated, width);
        writeln!(f, "{:<12} : {}", "ID", self.id)?;

        if let Some(values) = &self.custom_fields {
            for field in values {
                match &field.value {
                    CustomFieldValue::String(value) => {
                        if !value.is_empty() {
                            output_field!(f, &field.name, Some(value), width);
                        }
                    }
                    CustomFieldValue::Array(values) => {
                        if !values.is_empty() {
                            wrapped_csv(f, &field.name, values, width)?;
                        }
                    }
                }
            }
        }

        if !self.comments.is_empty() {
            writeln!(f, "{:<12} : {}", "Comments", self.comments.len())?;
        }

        // render both comments
        for comment in &self.comments {
            writeln!(f)?;
            comment.render(f, width)?;
        }

        Ok(())
    }
}
