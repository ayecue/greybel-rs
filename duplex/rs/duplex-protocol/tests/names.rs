use duplex_protocol::{Role, Symbol};

use crate::support::{SHOP, definition, protocol, struct_fields};

#[test]
fn should_name_a_field_in_each_casing() {
    let definition = definition("struct S { new_line: bool }");
    let field = &struct_fields(&definition, "S")[0];

    let name = Symbol::Field(field).name();

    assert_eq!(
        [name.pascal(), name.camel(), name.snake()],
        ["NewLine", "newLine", "new_line"]
    );
}

#[test]
fn should_name_what_is_generated_for_a_protocol() {
    let definition = definition(SHOP);
    let protocol = protocol(&definition);
    let peer = protocol.peer(Role::Provider);
    let order = &protocol.messages()[0];

    let names = [
        Symbol::Peers(protocol).pascal(),
        Symbol::Router(peer).pascal(),
        Symbol::Channel(peer).pascal(),
        Symbol::Handler(order).name().camel(),
        Symbol::Handler(order).name().snake(),
    ];

    assert_eq!(
        names,
        [
            "ShopPeer",
            "CounterRouter",
            "CounterChannel",
            "onOrder",
            "on_order"
        ]
    );
}

#[test]
fn should_case_a_declared_name_from_its_pascal_case() {
    let declared = "ReadKey";

    let name = Symbol::Declared(declared).name();

    assert_eq!(
        [name.pascal(), name.camel(), name.snake()],
        ["ReadKey", "readKey", "read_key"]
    );
}
