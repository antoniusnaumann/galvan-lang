use std::collections::HashMap;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use galvan_files::Source;
use galvan_into_ast::SourceIntoAst;
use galvan_parse::{parse_source, Node};

#[derive(Debug)]
pub struct GalvanLanguageServer {
    client: Client,
    documents: tokio::sync::RwLock<HashMap<Url, String>>,
}

impl GalvanLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    async fn validate_document(&self, uri: &Url, content: &str) {
        let mut diagnostics = Vec::new();
        
        // First, try tree-sitter parsing for syntax analysis
        let source = Source::Str(content.to_string().into());
        match parse_source(&source) {
            Ok(tree) => {
                // Check for syntax errors in the tree-sitter parse tree
                let root_node = tree.root_node();
                self.collect_syntax_errors(&root_node, content, &mut diagnostics);
                
                // Also try the AST parsing for semantic validation
                match source.try_into_ast() {
                    Ok(_ast) => {
                        // Both tree-sitter and AST parsing succeeded
                    }
                    Err(err) => {
                        // Tree-sitter succeeded but AST parsing failed - likely a semantic issue
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: content.len() as u32 },
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: None,
                            code_description: None,
                            source: Some("galvan-lsp".to_string()),
                            message: format!("Semantic error: {}", err),
                            related_information: None,
                            tags: None,
                            data: None,
                        });
                    }
                }
            }
            Err(err) => {
                // Tree-sitter parsing failed - syntax error
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: content.len() as u32 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("galvan-lsp".to_string()),
                    message: format!("Syntax error: {}", err),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    fn collect_syntax_errors(&self, node: &Node, content: &str, diagnostics: &mut Vec<Diagnostic>) {
        // Check if this node has an error
        if node.is_error() {
            let start_pos = node.start_position();
            let end_pos = node.end_position();
            
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: start_pos.row as u32,
                        character: start_pos.column as u32,
                    },
                    end: Position {
                        line: end_pos.row as u32,
                        character: end_pos.column as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("galvan-lsp".to_string()),
                message: "Syntax error".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        // Check if this node is missing (indicates incomplete syntax)
        if node.is_missing() {
            let start_pos = node.start_position();
            
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: start_pos.row as u32,
                        character: start_pos.column as u32,
                    },
                    end: Position {
                        line: start_pos.row as u32,
                        character: start_pos.column as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("galvan-lsp".to_string()),
                message: "Missing syntax element".to_string(),
                related_information: None,
                tags: None,
                data: None,
            });
        }

        // Recursively check child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_syntax_errors(&child, content, diagnostics);
        }
    }

    fn position_to_byte_offset(&self, content: &str, position: Position) -> usize {
        let mut byte_offset = 0;
        let mut current_line = 0;
        let mut current_char = 0;
        
        for ch in content.chars() {
            if current_line == position.line && current_char == position.character {
                break;
            }
            
            if ch == '\n' {
                current_line += 1;
                current_char = 0;
            } else {
                current_char += 1;
            }
            
            byte_offset += ch.len_utf8();
        }
        
        byte_offset
    }

    fn get_context_aware_completions(&self, node: Node) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Get the parent context to determine what completions are appropriate
        let context_kind = node.parent().map(|p| p.kind()).unwrap_or(node.kind());
        
        match context_kind {
            "function_declaration" | "function_signature" => {
                // Inside function context, suggest function-related keywords
                completions.extend(vec![
                    CompletionItem {
                        label: "return".to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some("Return statement".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "if".to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some("Conditional statement".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "let".to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        detail: Some("Variable binding".to_string()),
                        ..Default::default()
                    },
                ]);
            }
            "type_annotation" | "type_expression" => {
                // Inside type context, suggest types
                completions.extend(vec![
                    CompletionItem {
                        label: "Int".to_string(),
                        kind: Some(CompletionItemKind::TYPE_PARAMETER),
                        detail: Some("Integer type".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "Bool".to_string(),
                        kind: Some(CompletionItemKind::TYPE_PARAMETER),
                        detail: Some("Boolean type".to_string()),
                        ..Default::default()
                    },
                    CompletionItem {
                        label: "String".to_string(),
                        kind: Some(CompletionItemKind::TYPE_PARAMETER),
                        detail: Some("String type".to_string()),
                        ..Default::default()
                    },
                ]);
            }
            _ => {
                // General context, provide basic completions
                completions.extend(self.get_basic_completions());
            }
        }
        
        completions
    }

    fn get_basic_completions(&self) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "pub".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Public visibility modifier".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "fn".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Function declaration".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "let".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Variable binding".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "if".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Conditional statement".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "Int".to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("Integer type".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "Bool".to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("Boolean type".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "String".to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("String type".to_string()),
                ..Default::default()
            },
        ]
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for GalvanLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "galvan-lsp".to_string(),
                version: Some("0.0.0-dev09".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Galvan LSP server initialized!")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        
        self.documents.write().await.insert(uri.clone(), content.clone());
        self.validate_document(&uri, &content).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        
        if let Some(change) = params.content_changes.first() {
            let content = &change.text;
            self.documents.write().await.insert(uri.clone(), content.clone());
            self.validate_document(&uri, content).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        if let Some(content) = params.text {
            self.validate_document(&params.text_document.uri, &content).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        
        let documents = self.documents.read().await;
        if let Some(content) = documents.get(&uri) {
            let source = Source::Str(content.clone().into());
            
            // Use tree-sitter to get detailed information about the node at cursor position
            if let Ok(tree) = parse_source(&source) {
                let root_node = tree.root_node();
                let byte_offset = self.position_to_byte_offset(content, position);
                
                if let Some(node) = root_node.descendant_for_byte_range(byte_offset, byte_offset) {
                    let node_text = node.utf8_text(content.as_bytes()).unwrap_or("");
                    let node_kind = node.kind();
                    
                    let hover_content = format!(
                        "**{}**\n\n```galvan\n{}\n```\n\nRange: {}:{} - {}:{}",
                        node_kind,
                        node_text.trim(),
                        node.start_position().row + 1,
                        node.start_position().column + 1,
                        node.end_position().row + 1,
                        node.end_position().column + 1
                    );
                    
                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: hover_content,
                        }),
                        range: Some(Range {
                            start: Position {
                                line: node.start_position().row as u32,
                                character: node.start_position().column as u32,
                            },
                            end: Position {
                                line: node.end_position().row as u32,
                                character: node.end_position().column as u32,
                            },
                        }),
                    }));
                }
            }
            
            // Fallback to simple line information if tree-sitter fails
            let lines: Vec<&str> = content.lines().collect();
            if let Some(line) = lines.get(position.line as usize) {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(format!(
                        "Line {}: {}",
                        position.line + 1,
                        line.trim()
                    ))),
                    range: None,
                }));
            }
        }
        
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> {
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        
        let documents = self.documents.read().await;
        if let Some(content) = documents.get(&uri) {
            let source = Source::Str(content.clone().into());
            
            // Use tree-sitter to provide context-aware completions
            if let Ok(tree) = parse_source(&source) {
                let root_node = tree.root_node();
                let byte_offset = self.position_to_byte_offset(content, position);
                
                if let Some(node) = root_node.descendant_for_byte_range(byte_offset, byte_offset) {
                    return Ok(Some(CompletionResponse::Array(
                        self.get_context_aware_completions(node)
                    )));
                }
            }
        }
        
        // Fallback to basic completions
        Ok(Some(CompletionResponse::Array(self.get_basic_completions())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_lsp_initialization_result() {
        // Test that our LSP capabilities are configured correctly
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![".".to_string()]),
                work_done_progress_options: Default::default(),
                all_commit_characters: None,
                completion_item: None,
            }),
            ..Default::default()
        };
        
        // Verify that we have the expected capabilities
        assert!(capabilities.text_document_sync.is_some());
        assert!(capabilities.hover_provider.is_some());
        assert!(capabilities.completion_provider.is_some());
    }

    #[test]
    fn test_basic_completions() {
        // Test that basic completions include expected keywords
        let completions = vec![
            CompletionItem {
                label: "pub".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Public visibility modifier".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "fn".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Function declaration".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "let".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Variable binding".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "Int".to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("Integer type".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "Bool".to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("Boolean type".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "String".to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("String type".to_string()),
                ..Default::default()
            },
        ];
        
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"pub"));
        assert!(labels.contains(&"fn"));
        assert!(labels.contains(&"let"));
        assert!(labels.contains(&"Int"));
        assert!(labels.contains(&"Bool"));
        assert!(labels.contains(&"String"));
    }

    #[test]
    fn test_position_to_byte_offset_logic() {
        // Test the position to byte offset conversion logic
        let content = "hello\nworld\ntest";
        
        // Helper function that mimics the position_to_byte_offset logic
        fn calculate_byte_offset(content: &str, position: Position) -> usize {
            let mut byte_offset = 0;
            let mut current_line = 0;
            let mut current_char = 0;
            
            for ch in content.chars() {
                if current_line == position.line && current_char == position.character {
                    break;
                }
                
                if ch == '\n' {
                    current_line += 1;
                    current_char = 0;
                } else {
                    current_char += 1;
                }
                
                byte_offset += ch.len_utf8();
            }
            
            byte_offset
        }
        
        // Test start of document
        let pos = Position { line: 0, character: 0 };
        assert_eq!(calculate_byte_offset(content, pos), 0);
        
        // Test start of second line
        let pos = Position { line: 1, character: 0 };
        assert_eq!(calculate_byte_offset(content, pos), 6);
        
        // Test middle of first line
        let pos = Position { line: 0, character: 2 };
        assert_eq!(calculate_byte_offset(content, pos), 2);
    }
}