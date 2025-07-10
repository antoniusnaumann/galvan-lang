use tower_lsp::{LspService, Server};
use galvan_lsp::GalvanLanguageServer;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| GalvanLanguageServer::new(client));
    
    Server::new(stdin, stdout, socket).serve(service).await;
}