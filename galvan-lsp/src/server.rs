//! The [`tower_lsp`] server: connects protocol requests to the feature modules
//! and owns the set of open documents.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::request::{GotoTypeDefinitionParams, GotoTypeDefinitionResponse};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::Document;
use crate::features::{
    code_actions, completion, diagnostics, document_highlight, folding_range, formatting,
    goto_definition, hover, inlay_hints, references, rename, selection_range, semantic_tokens,
    signature_help, symbols, type_definition,
};
use crate::workspace::{crate_root, Crate};

pub struct Backend {
    client: Client,
    shared: Arc<Shared>,
    /// Receiver for interop-build completions, taken by the watcher task
    /// spawned on `initialized` (see [`watch_interop_ready`]).
    interop_events: Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<PathBuf>>>,
}

/// The state the watcher task shares with the request handlers.
struct Shared {
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

/// Everything a per-document request needs: the open document (an owned
/// snapshot — holding a map reference across the potentially long analysis
/// would serialize every request behind concurrent edits), the (cached)
/// crate it belongs to, and its on-disk path when it has one.
struct RequestContext {
    document: Document,
    krate: Arc<Crate>,
    file: Option<PathBuf>,
}

impl RequestContext {
    fn document(&self) -> &Document {
        &self.document
    }

    fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            shared: Arc::new(Shared {
                documents: DashMap::new(),
                crates: Mutex::new(HashMap::new()),
            }),
            interop_events: Mutex::new(crate::workspace::interop_ready_events()),
        }
    }

    /// The context for a request against the document at `uri`; `None` when
    /// the document is not open. The document is snapshotted and the map
    /// reference released immediately: `crate_for` iterates the document map,
    /// and holding an entry reference across map iteration (or across the
    /// analysis) can deadlock with concurrent `didChange` writes.
    fn request_context(&self, uri: &Url) -> Option<RequestContext> {
        let document = self.shared.documents.get(uri)?.clone();
        Some(RequestContext {
            krate: self.shared.crate_for(uri),
            file: uri.to_file_path().ok(),
            document,
        })
    }
}

impl Shared {
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

}

/// Re-analyse the crate containing `uri` and publish diagnostics for
/// *every* open document of that crate, so cross-file diagnostics never
/// go stale. Shared between the request handlers and the interop watcher.
async fn refresh(client: &Client, shared: &Shared, uri: Url) {
    let krate = shared.crate_for(&uri);

    let root = uri.to_file_path().ok().as_deref().and_then(crate_root);
    let crate_docs: Vec<Url> = shared
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
        // Snapshot the document so no map reference is held while the
        // (potentially long) analysis runs.
        let Some(document) = shared.documents.get(&doc_uri).map(|entry| entry.clone()) else {
            continue; // Closed concurrently.
        };
        let diags = diagnostics::diagnostics(&document, &krate, file.as_deref());
        client
            .publish_diagnostics(doc_uri, diags, Some(document.version))
            .await;
    }
}

/// React to finished background interop builds: drop the crate caches of the
/// affected project (their analyses ran without interop) and re-publish
/// diagnostics for its open documents, so the editor picks up the interop
/// without waiting for the next edit.
async fn watch_interop_ready(
    client: Client,
    shared: Arc<Shared>,
    mut events: tokio::sync::mpsc::UnboundedReceiver<PathBuf>,
) {
    while let Some(manifest_dir) = events.recv().await {
        shared
            .crates
            .lock()
            .unwrap()
            .retain(|root, _| !root.starts_with(&manifest_dir));

        // One refresh per affected crate root republishes every open
        // document of that crate.
        let mut seen_roots = std::collections::HashSet::new();
        let open: Vec<Url> = shared
            .documents
            .iter()
            .map(|entry| entry.key().clone())
            .collect();
        for uri in open {
            let Ok(path) = uri.to_file_path() else {
                continue;
            };
            if !path.starts_with(&manifest_dir) {
                continue;
            }
            let Some(root) = crate_root(&path) else {
                continue;
            };
            if seen_roots.insert(root) {
                refresh(&client, &shared, uri).await;
            }
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
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(true)),
                references_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
                    first_trigger_character: "}".to_string(),
                    more_trigger_character: None,
                }),
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
        if let Some(events) = self.interop_events.lock().unwrap().take() {
            tokio::spawn(watch_interop_ready(
                self.client.clone(),
                Arc::clone(&self.shared),
                events,
            ));
        }
        self.client
            .log_message(MessageType::INFO, "galvan-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.shared
            .documents
            .insert(doc.uri.clone(), Document::with_version(doc.text, doc.version));
        refresh(&self.client, &self.shared, doc.uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // Incremental sync: apply each change in order, each against the
        // text produced by the previous one. A change without a range
        // replaces the whole document (clients may still send those).
        let mut text = self
            .shared
            .documents
            .get(&uri)
            .map(|document| document.text.clone())
            .unwrap_or_default();
        for change in params.content_changes {
            match change.range {
                Some(range) => {
                    let index = crate::position::LineIndex::new(&text);
                    let (Some(start), Some(end)) = (
                        index.offset(&text, range.start),
                        index.offset(&text, range.end),
                    ) else {
                        continue;
                    };
                    text.replace_range(start..end, &change.text);
                }
                None => text = change.text,
            }
        }
        self.shared.documents.insert(
            uri.clone(),
            Document::with_version(text, params.text_document.version),
        );
        refresh(&self.client, &self.shared, uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        // Sibling files may have changed on disk alongside the save (e.g. a
        // formatter run); re-read the crate.
        let uri = params.text_document.uri;
        self.shared.evict_crate(&uri);
        refresh(&self.client, &self.shared, uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.shared.documents.remove(&uri);
        // Clear this document's diagnostics in the client; they would
        // otherwise linger with no owner to update them.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(hover::hover(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(goto_definition::goto_definition(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
        )
        .map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let position = params.text_document_position;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        let locations = references::references(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
            params.context.include_declaration,
        );
        Ok(Some(locations))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let Some(document) = self.shared.documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(formatting::formatting(&document, &params.options))
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let Some(document) = self.shared.documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(formatting::range_formatting(
            &document,
            &params.options,
            params.range,
        ))
    }

    async fn on_type_formatting(
        &self,
        params: DocumentOnTypeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        let position = params.text_document_position;
        let Some(document) = self.shared.documents.get(&position.text_document.uri) else {
            return Ok(None);
        };
        Ok(formatting::on_type_formatting(
            &document,
            &params.options,
            position.position,
        ))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(Some(document_highlight::document_highlight(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
        )))
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let Some(document) = self.shared.documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(folding_range::folding_ranges(&document)))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        let Some(document) = self.shared.documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        // One result per requested position, in order (the protocol requires
        // the arrays to correspond); positions that resolve to nothing get an
        // empty range at the position itself.
        Ok(Some(
            params
                .positions
                .into_iter()
                .map(|position| {
                    selection_range::selection_range(&document, position).unwrap_or(
                        SelectionRange {
                            range: Range {
                                start: position,
                                end: position,
                            },
                            parent: None,
                        },
                    )
                })
                .collect(),
        ))
    }

    async fn goto_type_definition(
        &self,
        params: GotoTypeDefinitionParams,
    ) -> Result<Option<GotoTypeDefinitionResponse>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(type_definition::type_definition(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
        )
        .map(GotoTypeDefinitionResponse::Scalar))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(Some(code_actions::code_actions(
            context.document(),
            &context.krate,
            context.file(),
            params.range,
            &params.context,
        )))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(Some(SemanticTokensResult::Tokens(
            semantic_tokens::semantic_tokens(context.document(), &context.krate, context.file()),
        )))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let position = params.text_document_position_params;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(signature_help::signature_help(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
        ))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(rename::prepare_rename(
            context.document(),
            &context.krate,
            context.file(),
            params.position,
        ))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let position = params.text_document_position;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(rename::rename(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
            &params.new_name,
        ))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Nested(
            symbols::document_symbols(context.document(), &context.krate, context.file()),
        )))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        // Search every crate that has an open document.
        let uris: Vec<Url> = self
            .shared
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
            let krate = self.shared.crate_for(&uri);
            results.extend(symbols::workspace_symbols(&krate, &params.query));
        }
        Ok(Some(results))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(Some(inlay_hints::inlay_hints(
            context.document(),
            &context.krate,
            context.file(),
            params.range,
        )))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position;
        let uri = position.text_document.uri;
        let Some(context) = self.request_context(&uri) else {
            return Ok(None);
        };
        Ok(Some(CompletionResponse::Array(completion::completion(
            context.document(),
            &context.krate,
            context.file(),
            position.position,
        ))))
    }
}
