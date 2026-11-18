//! Tests the server's advertised tool list.

use greybel_mcp::ServerOptions;

use crate::support::{Client, options, run};

#[test]
fn should_list_the_game_tools_only_when_allowed() {
    run(async {
        let mut client = Client::connect(options()).await;
        let mut game_client = Client::connect(ServerOptions {
            allow_game: true,
            ..options()
        })
        .await;
        let names = client.tool_names().await;
        let game_names = game_client.tool_names().await;

        for name in [
            "api_reference",
            "api_search",
            "build",
            "call_hierarchy",
            "check",
            "describe",
            "document_symbols",
            "find_references",
            "find_symbol",
            "rename",
            "run",
            "transform",
        ] {
            assert!(names.iter().any(|candidate| candidate == name), "{names:?}");
        }

        assert!(
            !names.iter().any(|name| name == "import_into_game"),
            "{names:?}"
        );
        assert!(
            game_names.iter().any(|name| name == "import_into_game"),
            "{game_names:?}"
        );
    });
}
