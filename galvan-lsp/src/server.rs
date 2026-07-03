//! The [`tower_lsp`] server: connects protocol requests to the feature modules
//! and owns the set of open documents.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::Document;
use crate::features::{
    code_actions, completion, diagnostics, formatting, goto_definition, hover, inlay_hints,
    references, rename, semantic_tokens, signature_help, symbols,
};
use crate::workspace::{crate_root, Crate};

pub struct Backend {
    client: Client,
    documents: DashMap<Url, Document>,
    /// Loaded (and lazily analyzed) crates, keyed by crate root. An entry is
    /// reused as long as the same documents are open at the same versions;
    /// any edit, open, close or save rebuilds it. Known limitation: edits
    /// made *on disk* while no open document changes are picked up only on
    /// the next open/change/save.
    crates: Mutex<HashMap<PathBuf, CachedCrate>>,
}

struct CachedCrate {
    /// Sorted `(uri, version)` of the open documents under the crate root at
    /// load time.
    key: Vec<(Url, i32)>,
    krate: Arc<Crate>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            crates: Mutex::new(HashMap::new()),
        }
    }

    /// The (cached) crate the document at `uri` belongs to.
    fn crate_for(&self, uri: &Url) -> Arc<Crate> {
        let Some(root) = uri.to_file_path().ok().as_deref().and_then(crate_root) else {
            // Untitled buffers and other non-file documents resolve in
            // isolation and are cheap; skip the cache.
            return Arc::new(Crate::load(uri, &self.documents));
        };

        let key = self.cache_key(&root);
        let mut crates = self.crates.lock().unwrap();
        if let Some(cached) = crates.get(&root) {
            if cached.key == key {
                return Arc::clone(&cached.krate);
            }
        }
        let krate = Arc::new(Crate::load(uri, &self.documents));
        crates.insert(
            root,
            CachedCrate {
                key,
                krate: Arc::clone(&krate),
            },
        );
        krate
    }

    /// Cache key for a crate root: the open documents under it with their
    /// versions.
    fn cache_key(&self, root: &Path) -> Vec<(Url, i32)> {
        let mut key: Vec<(Url, i32)> = self
            .documents
            .iter()
            .filter(|entry| {
                entry
                    .key()
                    .to_file_path()
                    .is_ok_and(|path| path.starts_with(root))
            })
            .map(|entry| (entry.key().clone(), entry.value().version))
            .collect();
        key.sort();
        key
    }

    /// Drop the cached crate containing `uri`, forcing the next request to
    /// re-read the crate from disk (used on save, when sibling files may
    /// have changed on disk, e.g. through a formatter or branch switch).
    fn evict_crate(&self, uri: &Url) {
        if let Some(root) = uri.to_file_path().ok().as_deref().and_then(crate_root) {
            self.crates.lock().unwrap().remove(&root);
        }
    }

    /// Re-analyse the crate containing `uri` and publish diagnostics for
    /// *every* open document of that crate, so cross-file diagnostics never
    /// go stale.
    async fn refresh(&self, uri: Url) {
        let krate = self.crate_for(&uri);

        let root = uri.to_file_path().ok().as_deref().and_then(crate_root);
        let crate_docs: Vec<Url> = self
            .documents
            .iter()
            .map(|entry| entry.key().clone())
            .filter(|doc_uri| {
                doc_uri == &uri
                    || match (&root, doc_uri.to_file_path()) {
                        (Some(root), Ok(path)) => path.starts_with(root),
                        _ => false,
                    }
            })
            .collect();

        for doc_uri in crate_docs {
            let file = doc_uri.to_file_path().ok();
            // Scope the document borrow so it is released before the await.
            let published = {
                let Some(document) = self.documents.get(&doc_uri) else {
                    continue; // Closed concurrently.
                };
                let diags = diagnostics::diagnostics(&document, &krate, file.as_deref());
                (diags, document.version)
            };
            self.client
                .publish_diagnostics(doc_uri, published.0, Some(published.1))
                .await;
        }
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
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            work_done_progress_options: Default::default(),
                        },
                    ),
                ),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    work_done_progress_options: Default::default(),
                }),
                completion_provider: Some(CompletionOptions {
                    // `:` is kept so clients auto-trigger after `::` (each
                    // colon fires individually; `context_at` gates results).
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
        self.documents
            .insert(doc.uri.clone(), Document::with_version(doc.text, doc.version));
        self.refresh(doc.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Full-sync mode: the last change contains the entire document text.
        if let Some(change) = params.content_changes.into_iter().last() {
            let uri = params.text_document.uri;
            self.documents.insert(
                uri.clone(),
                Document::with_version(change.text, params.text_document.version),
            );
            self.refresh(uri).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Sibling files may have changed on disk alongside the save (e.g. a
        // formatter run); re-read the crate.
        let uri = params.text_document.uri;
        self.evict_crate(&uri);
        self.refresh(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        // Clear this document's diagnostics in the client; they would
        // otherwise linger with no owner to update them.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(hover::hover(
            &document,
            &krate,
            file.as_deref(),
            position.position,
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(goto_definition::goto_definition(
            &document,
            &krate,
            file.as_deref(),
            position.position,
        )
        .map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let position = params.text_document_position;
        let uri = position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        let locations = references::references(
            &document,
            &krate,
            file.as_deref(),
            position.position,
            params.context.include_declaration,
        );
        Ok(Some(locations))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let Some(document) = self.documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(formatting::formatting(&document, &params.options))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(Some(code_actions::code_actions(
            &document,
            &krate,
            file.as_deref(),
            params.range,
            &params.context,
        )))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(Some(SemanticTokensResult::Tokens(
            semantic_tokens::semantic_tokens(&document, &krate, file.as_deref()),
        )))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(signature_help::signature_help(
            &document,
            &krate,
            file.as_deref(),
            position.position,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(rename::prepare_rename(
            &document,
            &krate,
            file.as_deref(),
            params.position,
        ))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let position = params.text_document_position;
        let uri = position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(rename::rename(
            &document,
            &krate,
            file.as_deref(),
            position.position,
            &params.new_name,
        ))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(Some(DocumentSymbolResponse::Nested(
            symbols::document_symbols(&document, &krate, file.as_deref()),
        )))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        // Search every crate that has an open document.
        let uris: Vec<Url> = self
            .documents
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        let mut seen_roots = Vec::new();
        let mut results = Vec::new();
        for uri in uris {
            let root = uri.to_file_path().ok().as_deref().and_then(crate_root);
            if let Some(root) = &root {
                if seen_roots.contains(root) {
                    continue;
                }
                seen_roots.push(root.clone());
            }
            let krate = self.crate_for(&uri);
            results.extend(symbols::workspace_symbols(&krate, &params.query));
        }
        Ok(Some(results))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(Some(inlay_hints::inlay_hints(
            &document,
            &krate,
            file.as_deref(),
            params.range,
        )))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position;
        let uri = position.text_document.uri;
        let Some(document) = self.documents.get(&uri) else {
            return Ok(None);
        };
        let krate = self.crate_for(&uri);
        let file = uri.to_file_path().ok();
        Ok(Some(CompletionResponse::Array(completion::completion(
            &document,
            &krate,
            file.as_deref(),
            position.position,
        ))))
    }
}
