use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::Contact;

use crate::support::layout::layout;

#[tokio::test]
async fn should_preserve_a_layouts_whois_contact() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut layout = layout();
    let contact = Contact {
        name: "Rental Admin".to_owned(),
        mail: "admin@example.test".to_owned(),
        phone: "123456789".to_owned(),
    };

    layout.contact = Some(contact.clone());

    world
        .modules
        .creation
        .network_generation
        .add_network_layout(&layout)
        .await
        .unwrap();

    let whois = world
        .modules
        .core
        .networks
        .dns
        .whois(&layout.public_ip)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(whois, (String::new(), Some(contact)));
}
