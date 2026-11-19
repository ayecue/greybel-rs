//! Tests that nesting past `MAX_NODE_DEPTH` produces an error instead of a stack overflow in strict and unsafe
//! parsing.

use greybel_core::highlight::Highlighter;
use greybel_core::{Dialect, ErrorCode, MAX_NODE_DEPTH, Parser};
use greybel_test_support::node_depth::{
    CHAIN_LENGTH, assert_no_failures, nested_programs, operator_chains, programs_at_max_depth,
};

use crate::support::greybel_options;

#[test]
fn should_parse_code_at_the_nesting_limit() {
    let cases = programs_at_max_depth();

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(shape, source)| (shape, Parser::new(source, greybel_options()).parse()))
        .collect();

    assert_no_failures(outcomes);
}

#[test]
fn should_reject_code_nested_past_the_depth_limit() {
    let cases: Vec<_> = [MAX_NODE_DEPTH + 8, 100_000]
        .into_iter()
        .flat_map(nested_programs)
        .collect();

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(shape, source)| (*shape, Parser::new(source, greybel_options()).parse()))
        .collect();

    for (shape, result) in outcomes {
        let error = result.expect_err(shape);

        assert_eq!(error.code(), ErrorCode::NodeDepthLimit, "{shape}: {error}");
    }
}

#[test]
fn should_skip_a_statement_past_the_depth_limit_in_an_unsafe_parse() {
    let cases: Vec<_> = nested_programs(100_000)
        .into_iter()
        .map(|(shape, source)| (shape, format!("{source}\ny = 2\n")))
        .collect();

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(shape, source)| (shape, Parser::new(source, greybel_options()).parse_unsafe()))
        .collect();

    for (shape, output) in outcomes {
        assert!(
            output
                .errors
                .iter()
                .any(|error| error.code() == ErrorCode::NodeDepthLimit),
            "{shape}"
        );

        let last = output.ast.body().last().copied().expect("statements");

        assert_eq!(output.ast.text(last), Some("y = 2"), "{shape}");
    }
}

#[test]
fn should_parse_operator_chains_of_any_length() {
    let cases = operator_chains(CHAIN_LENGTH);

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(shape, source)| {
            let parsed = Parser::new(source, greybel_options()).parse();

            if let Ok(ast) = &parsed {
                assert!(
                    !Highlighter::new(ast).tokens(Dialect::Greybel).is_empty(),
                    "{shape}"
                );
            }

            (shape, parsed)
        })
        .collect();

    assert_no_failures(outcomes);
}

#[test]
fn should_walk_an_operator_chain_from_its_first_operand() {
    let ast = Parser::new("x = 1 + 2 * 3 - 4", greybel_options())
        .parse()
        .unwrap();
    let value = ast
        .nodes()
        .children(ast.body()[0])
        .last()
        .copied()
        .expect("the assignment's value");

    let chain = ast.operator_chain(value);

    assert_eq!(ast.text(chain.first), Some("1"));
    assert_eq!(
        chain
            .links
            .iter()
            .map(|&link| ast.text(link))
            .collect::<Vec<_>>(),
        [Some("1 + 2 * 3"), Some("1 + 2 * 3 - 4")]
    );
}
