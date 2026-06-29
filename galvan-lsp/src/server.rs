//! The [`tower_lsp`] server: connects protocol requests to the feature modules
//! and owns the set of open documents.

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::Document;
use crate::features::{completion, diagnostics, goto_definition, hover};

pub struct Backend {
    client: Client,
    documents: DashMap<Url, Document>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Re-analyse a document and publish its diagnostics.
    async fn refresh(&self, uri: Url, text: String, version: i32) {
        let document = Document::new(text);
        let diags = diagnostics::diagnostics(&document);
        self.documents.insert(uri.clone(), document);
        self.client
            .publish_diagnostics(uri, diags, Some(version))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "galvan-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "galvan-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.refresh(doc.uri, doc.text, doc.version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Full-sync mode: the last change contains the entire document text.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.refresh(
                params.text_document.uri,
                change.text,
                params.text_document.version,
            )
            .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params;
        let Some(document) = self.documents.get(&position.text_document.uri) else {
            return Ok(None);
        };
        Ok(hover::hover(&document, position.position))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri.clone();
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        Ok(goto_definition::goto_definition(&document, uri, position.position)
            .map(GotoDefinitionResponse::Scalar))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        Ok(Some(CompletionResponse::Array(completion::completion(
            &document,
        ))))
    }
}
