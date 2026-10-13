use greyhack_behaviour::{GUEST_USER, LibraryKind};

use crate::{Library, Rng};

#[test]
fn should_generate_libraries_deterministically() {
    let (mut first, mut second) = (Rng::new(5), Rng::new(5));
    let a = Library::generate(&mut first, LibraryKind::Ssh, "1.0.0");
    let b = Library::generate(&mut second, LibraryKind::Ssh, "1.0.0");

    assert_eq!(a, b);
    assert!(!a.zones.is_empty());
    assert!(a.zones.iter().all(|zone| !zone.weaknesses.is_empty()));
}

#[test]
fn should_give_a_router_library_a_free_guest_weakness() {
    let mut rng = Rng::new(9);
    let library = Library::generate(&mut rng, LibraryKind::KernelRouter, "1.0.0");
    let has_free_guest = library.zones.iter().any(|zone| {
        !zone.hide
            && zone
                .weaknesses
                .iter()
                .any(|weakness| weakness.user == GUEST_USER && weakness.requirements.is_empty())
    });

    assert!(has_free_guest);
}
