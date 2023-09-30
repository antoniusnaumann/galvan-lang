use std::fmt::{Debug, Display};

use annotate_snippets::display_list::{DisplayList, FormatOptions};
use annotate_snippets::snippet::{Annotation, AnnotationType, Slice, Snippet, SourceAnnotation};
use derive_more::{Display, From};

use crate::Error;

pub trait FormattedOutput {
    fn formatted_output<'a>(&'a self, source: &'a str) -> DisplayList;
}

impl FormattedOutput for Error {
    fn formatted_output<'a>(&'a self, source: &'a str) -> DisplayList {
        let snippet = Snippet {
            title: Some(Annotation {
                label: Some(&self.0),
                id: None,
                annotation_type: AnnotationType::Error,
            }),
            footer: vec![],
            slices: vec![Slice {
                source,
                line_start: self.1.start,
                origin: None,
                annotations: vec![SourceAnnotation {
                    range: (self.1.start, self.1.end),
                    label: &self.0,
                    annotation_type: AnnotationType::Error,
                }],
                fold: false,
            }],
            opt: FormatOptions {
                color: true,
                ..Default::default()
            },
        };

        DisplayList::from(snippet)
    }
}

/// Converts a Result into a displayable result with a source string
pub trait DisplayWithSource {
    type Success;
    /// Converts the error case into a formatted error and leaks both the underlying error
    /// Note: Only use this for tests
    fn leak_with_source(self, src: &'static str) -> DisplayResult<'static, Self::Success>;
}

impl<T> DisplayWithSource for crate::Result<T> {
    type Success = T;
    fn leak_with_source(self, src: &'static str) -> DisplayResult<'static, Self::Success> {
        self.map_err(|e| Box::leak(e.into()).formatted_output(src).into())
    }
}

pub type DisplayResult<'a, T> = std::result::Result<T, DisplayedError<'a>>;

#[derive(From, Display)]
pub struct DisplayedError<'a>(DisplayList<'a>);

impl Debug for DisplayedError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "\n\n{}\n", self)
    }
}
