use reman::language::Scope;
use reman::repository::Repository;

/// `from -> to` of every edge of `scope`.
pub fn edges(repository: &Repository, scope: Scope) -> Vec<String> {
    repository
        .edges
        .iter()
        .filter(|edge| edge.scope == scope)
        .map(|edge| {
            format!(
                "{} -> {}",
                repository.nodes[edge.from].id, repository.nodes[edge.to].id
            )
        })
        .collect()
}
