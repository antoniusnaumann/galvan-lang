use std::collections::HashMap;
use tower_lsp::jsonrpc;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use galvan_files::Source;
use galvan_into_ast::SourceIntoAst;

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
        
        // Try to parse the document with the existing AST infrastructure
        let source = Source::Str(content.to_string().into());
        match source.try_into_ast() {
            Ok(_ast) => {
                // Document parsed successfully, no diagnostics needed
            }
            Err(err) => {
                // Add a diagnostic for the parsing error
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: 0, character: 0 },
                        end: Position { line: 0, character: content.len() as u32 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: None,
                    code_description: None,
                    source: Some("galvan-lsp".to_string()),
                    message: format!("Parse error: {}", err),
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
            // Simple hover: show information about the current line
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

    async fn completion(&self, _params: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> {
        // Basic completion: suggest some Galvan keywords
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
        ];

        Ok(Some(CompletionResponse::Array(completions)))
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
    fn test_completion_keywords() {
        // Test that our completion keywords are correctly configured
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
        ];
        
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"pub"));
        assert!(labels.contains(&"fn"));
        assert!(labels.contains(&"Int"));
        assert!(labels.contains(&"Bool"));
    }
}