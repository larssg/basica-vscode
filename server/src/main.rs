mod backend;
mod completion;
mod definition;
mod diagnostics;
mod hover;
mod symbols;

use backend::BasicaBackend;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(BasicaBackend::new).finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
