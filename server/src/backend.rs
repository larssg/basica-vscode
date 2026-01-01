use std::collections::HashMap;
use std::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::completion;
use crate::definition;
use crate::diagnostics;
use crate::hover;
use crate::symbols;

pub struct BasicaBackend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl BasicaBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: RwLock::new(HashMap::new()),
        }
    }

    async fn validate(&self, uri: &Url, text: &str) {
        let diagnostics = diagnostics::check(text);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BasicaBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![" ".to_string()]),
                    ..Default::default()
                }),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "basica LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents
            .write()
            .unwrap()
            .insert(uri.clone(), text.clone());
        self.validate(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents
                .write()
                .unwrap()
                .insert(uri.clone(), change.text.clone());
            self.validate(&uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .unwrap()
            .remove(&params.text_document.uri);
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.documents.read().unwrap();
        if let Some(text) = docs.get(uri) {
            return Ok(definition::find_definition(text, pos, uri.clone()));
        }
        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let docs = self.documents.read().unwrap();
        if let Some(text) = docs.get(uri) {
            return Ok(hover::get_hover(text, pos));
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let docs = self.documents.read().unwrap();
        if let Some(text) = docs.get(uri) {
            let items = completion::get_completions(text, pos);
            return Ok(Some(CompletionResponse::Array(items)));
        }
        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;
        let docs = self.documents.read().unwrap();
        if let Some(text) = docs.get(uri) {
            let syms = symbols::get_document_symbols(text);
            return Ok(Some(DocumentSymbolResponse::Nested(syms)));
        }
        Ok(None)
    }
}
