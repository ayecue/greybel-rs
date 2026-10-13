//! Tests that synchronous and asynchronous fixture loading produce the same graph without duplicate reads.

use greybel_documents_fs_provider::FsProvider;
use greybel_test_support::documents::{CallRecorder, graph_outline};
use greybel_test_support::future::block_on;
use greybel_transpiler::TranspileOptions;
use greybel_transpiler::documents::ModuleGraph;

use crate::support::greybel::fixtures;

#[test]
fn should_load_the_same_graph_sync_and_async() {
    let options = TranspileOptions::default().load_options();
    let fixtures = fixtures();
    let paths: Vec<_> = ["scripts", "special"]
        .into_iter()
        .flat_map(|directory| std::fs::read_dir(fixtures.path().join(directory)).expect("fixtures"))
        .map(|entry| FsProvider::uri(&entry.expect("fixture").path()))
        .collect();

    let outcomes: Vec<_> = paths
        .iter()
        .map(|path| {
            let sync_provider = CallRecorder::new(FsProvider::default());
            let sync = ModuleGraph::load(path, &options, &sync_provider)
                .map(|graph| graph_outline(&graph))
                .map_err(|error| error.to_string());
            let async_provider = CallRecorder::new(FsProvider::default());
            let streamed = block_on(ModuleGraph::load_async(path, &options, &async_provider))
                .map(|graph| graph_outline(&graph))
                .map_err(|error| error.to_string());
            let repeated = [
                sync_provider.repeated_reads(),
                async_provider.repeated_reads(),
            ];

            (path, sync, streamed, repeated)
        })
        .collect();

    let checked = outcomes.len();

    for (path, sync, streamed, repeated) in outcomes {
        assert_eq!(sync, streamed, "{path}");
        assert_eq!(
            repeated,
            [Vec::<String>::new(), Vec::new()],
            "{path} read twice"
        );
    }

    assert!(checked > 0);
}
