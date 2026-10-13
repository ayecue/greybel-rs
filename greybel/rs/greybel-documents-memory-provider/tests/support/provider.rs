use greybel_documents::ResourceProvider;
use greybel_documents_memory_provider::*;

pub fn relative(provider: &MemoryProvider, source: &str, target: &str) -> String {
    ResourceProvider::relative_to(provider, &MemoryProvider::uri(source), target).to_string()
}

pub fn files_in_lib() -> MemoryProvider {
    [("lib/a.src", "a"), ("lib/b", "b")].into_iter().collect()
}

/// Creates a provider with `/a.src` set to `1`, then replaces it with `2` using relative path `a.src`.
pub fn provider_with_a_replaced_file() -> MemoryProvider {
    let mut provider = MemoryProvider::new();

    provider.insert("/a.src", "1");
    provider.insert("a.src", "2");

    provider
}
