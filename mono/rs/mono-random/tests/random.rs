use mono_random::Random;

use crate::support::dotnet::DOTNET;

#[test]
fn should_match_dotnet_next_double_sequences() {
    let cases = DOTNET;
    let outcomes: Vec<(i32, Vec<f64>)> = cases
        .iter()
        .map(|&(seed, _, _)| {
            let mut random = Random::new(seed);

            (seed, (0..4).map(|_| random.next_double()).collect())
        })
        .collect();

    for ((seed, got), &(_, doubles, _)) in outcomes.iter().zip(cases) {
        assert_eq!(*got, doubles, "NextDouble, seed {seed}");
    }
}

#[test]
fn should_match_dotnet_next_below_sequences() {
    let cases = DOTNET;
    let outcomes: Vec<(i32, Vec<usize>)> = cases
        .iter()
        .map(|&(seed, _, _)| {
            let mut random = Random::new(seed);

            (seed, (0..4).map(|_| random.next_below(100)).collect())
        })
        .collect();

    for ((seed, got), &(_, _, below)) in outcomes.iter().zip(cases) {
        assert_eq!(*got, below, "Next(100), seed {seed}");
    }
}
