use std::fmt;

use thiserror::Error;

/// Represents the severity of a diagnostic message
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Represents a single diagnostic message with source location
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub span: Option<Span>,
    pub suggestion: Option<String>,
}

/// Span information for error reporting
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub file: String,
}

impl From<galvan_ast::Span> for Span {
    fn from(ast_span: galvan_ast::Span) -> Self {
        Self {
            start: ast_span.range.0,
            end: ast_span.range.1,
            file: "".to_string(), // AST span doesn't include file info
        }
    }
}

/// Main error type for the transpiler
#[derive(Debug, Error)]
pub enum TranspilerError {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("Unknown identifier: {name}")]
    UnknownIdentifier { name: String },

    #[error("Unknown type: {name}")]
    UnknownType { name: String },

    #[error("Cannot assign to immutable variable: {name}")]
    ImmutableAssignment { name: String },

    #[error("Invalid operation: {operation} on types {left} and {right}")]
    InvalidOperation { operation: String, left: String, right: String },

    #[error("Function {name} expects {expected} arguments, found {found}")]
    ArgumentCountMismatch { name: String, expected: usize, found: usize },

    #[error("Cannot infer type for expression")]
    TypeInferenceFailure,

    #[error("Unimplemented feature: {feature}")]
    Unimplemented { feature: String },

    #[error("Invalid syntax: {message}")]
    InvalidSyntax { message: String },

    #[error("Circular dependency detected")]
    CircularDependency,

    #[error("Invalid modifier: {modifier} is not allowed for {context}")]
    InvalidModifier { modifier: String, context: String },

    #[error("Missing argument: {operation} requires a {argument_type}")]
    MissingArgument { operation: String, argument_type: String },

    #[error("Invalid operation: {operation} can only be used on {allowed_types}")]
    InvalidOperationOnType { operation: String, allowed_types: String },

    #[error("Enum access error: {message}")]
    EnumAccessError { message: String },

    #[error("Member access error: {message}")]
    MemberAccessError { message: String },

    #[error("Incompatible ownership types: {message}")]
    IncompatibleOwnership { message: String },

    #[error("Unsupported assignment operation: {operation} is not supported on {type_name} types. Only plain assignment (=) is supported for indexed dictionary and set access.")]
    UnsupportedDictSetAssignment { operation: String, type_name: String },
}

/// Collects errors and warnings during compilation
#[derive(Debug, Default)]
pub struct ErrorCollector {
    diagnostics: Vec<Diagnostic>,
    error_count: usize,
    warning_count: usize,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error to the collector
    pub fn error(&mut self, error: TranspilerError) {
        self.error_with_span(error, None);
    }

    /// Add an error with span information
    pub fn error_with_span(&mut self, error: TranspilerError, span: Option<Span>) {
        self.diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: error.to_string(),
            span,
            suggestion: None,
        });
        self.error_count += 1;
    }

    /// Add an error with a suggestion
    pub fn error_with_suggestion(&mut self, error: TranspilerError, span: Option<Span>, suggestion: String) {
        self.diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: error.to_string(),
            span,
            suggestion: Some(suggestion),
        });
        self.error_count += 1;
    }

    /// Add a warning
    pub fn warning(&mut self, message: String, span: Option<Span>) {
        self.diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Warning,
            message,
            span,
            suggestion: None,
        });
        self.warning_count += 1;
    }

    /// Add an info message
    pub fn info(&mut self, message: String, span: Option<Span>) {
        self.diagnostics.push(Diagnostic {
            severity: DiagnosticSeverity::Info,
            message,
            span,
            suggestion: None,
        });
    }

    /// Check if any errors were collected
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Get the number of errors
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Get the number of warnings
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Get all diagnostics
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Get only errors
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Get only warnings
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.severity == DiagnosticSeverity::Warning)
    }

    /// Merge another ErrorCollector into this one
    pub fn merge(&mut self, other: ErrorCollector) {
        self.diagnostics.extend(other.diagnostics);
        self.error_count += other.error_count;
        self.warning_count += other.warning_count;
    }

    /// Create a Result type that fails if errors were collected
    pub fn into_result<T>(self, value: T) -> Result<T, Vec<Diagnostic>> {
        if self.has_errors() {
            Err(self.diagnostics)
        } else {
            Ok(value)
        }
    }

    /// Suggest similar identifiers using Levenshtein distance
    pub fn suggest_similar_identifier(&mut self, unknown: &str, available: &[String], span: Option<Span>) {
        if let Some(suggestion) = find_closest_match(unknown, available) {
            self.error_with_suggestion(
                TranspilerError::UnknownIdentifier { name: unknown.to_string() },
                span,
                format!("Did you mean '{}'?", suggestion),
            );
        } else {
            self.error_with_span(
                TranspilerError::UnknownIdentifier { name: unknown.to_string() },
                span,
            );
        }
    }

    /// Suggest similar types
    pub fn suggest_similar_type(&mut self, unknown: &str, available: &[String], span: Option<Span>) {
        if let Some(suggestion) = find_closest_match(unknown, available) {
            self.error_with_suggestion(
                TranspilerError::UnknownType { name: unknown.to_string() },
                span,
                format!("Did you mean '{}'?", suggestion),
            );
        } else {
            self.error_with_span(
                TranspilerError::UnknownType { name: unknown.to_string() },
                span,
            );
        }
    }
}

/// Find the closest match using Levenshtein distance
fn find_closest_match(target: &str, candidates: &[String]) -> Option<String> {
    let mut best_match = None;
    let mut best_distance = usize::MAX;
    
    for candidate in candidates {
        let distance = levenshtein_distance(target, candidate);
        // Only suggest if the distance is reasonable (less than half the length)
        if distance < target.len().max(candidate.len()) / 2 && distance < best_distance {
            best_distance = distance;
            best_match = Some(candidate.clone());
        }
    }
    
    best_match
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();
    
    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }
    
    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];
    
    // Initialize first row and column
    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }
    
    // Fill the matrix
    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[a_len][b_len]
}

impl fmt::Display for ErrorCollector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for diagnostic in &self.diagnostics {
            match diagnostic.severity {
                DiagnosticSeverity::Error => write!(f, "error: ")?,
                DiagnosticSeverity::Warning => write!(f, "warning: ")?,
                DiagnosticSeverity::Info => write!(f, "info: ")?,
            }
            
            writeln!(f, "{}", diagnostic.message)?;
            
            if let Some(ref suggestion) = diagnostic.suggestion {
                writeln!(f, "  help: {}", suggestion)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_collector_basic() {
        let mut collector = ErrorCollector::new();
        
        assert!(!collector.has_errors());
        assert_eq!(collector.error_count(), 0);
        
        collector.error(TranspilerError::UnknownIdentifier { name: "foo".to_string() });
        
        assert!(collector.has_errors());
        assert_eq!(collector.error_count(), 1);
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
        assert_eq!(levenshtein_distance("hello", "world"), 4);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("", "test"), 4);
        assert_eq!(levenshtein_distance("test", ""), 4);
    }

    #[test]
    fn test_suggest_similar_identifier() {
        let mut collector = ErrorCollector::new();
        let available = vec!["variable".to_string(), "function".to_string(), "method".to_string()];
        
        collector.suggest_similar_identifier("variabe", &available, None);
        
        assert!(collector.has_errors());
        let diagnostics = collector.diagnostics();
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].suggestion.as_ref().unwrap().contains("variable"));
    }
}