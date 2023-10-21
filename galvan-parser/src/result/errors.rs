use crate::{AnnotationType, ParserMessage, Source, Span};

pub struct TokenError {
    pub msg: String,
    pub span: Span,
    pub annotation: String,
}

impl Into<ParserMessage<'_>> for (Source, TokenError) {
    fn into(self) -> ParserMessage<'static> {
        let (src, err) = self;
        let TokenError {
            msg,
            span,
            annotation,
        } = err;
        ParserMessage {
            issue: msg,
            hint: None,
            msg_type: AnnotationType::Error,
            src,
            // TODO: Create annotations
            annotations: vec![],
        }
    }
}
