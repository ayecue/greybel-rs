use greyhack_behaviour::{
    HACK_SHOP_LIBRARIES, LibraryKind, NetworkKind, detail_key, shop_programs,
};

#[test]
fn should_list_programs_for_shop_networks_only() {
    assert_eq!(
        shop_programs(NetworkKind::HackShop).map(<[_]>::len),
        Some(7)
    );
    assert_eq!(
        shop_programs(NetworkKind::ComputerShop).map(<[_]>::len),
        Some(12)
    );
    assert_eq!(
        shop_programs(NetworkKind::NetServices),
        Some(&[("ConfigLan.exe", 0)][..])
    );
    assert_eq!(shop_programs(NetworkKind::Police), None);
}

#[test]
fn should_sell_the_libraries_of_the_hack_shop() {
    assert_eq!(
        HACK_SHOP_LIBRARIES,
        [
            LibraryKind::Metaxploit,
            LibraryKind::Crypto,
            LibraryKind::Rshell
        ]
    );
}

#[test]
fn should_key_the_detail_text_by_the_name_without_its_extension() {
    assert_eq!(detail_key("AdminMonitor.exe"), "ADMINMONITOR_DETAIL");
    assert_eq!(detail_key("librshell.so"), "LIBRSHELL_DETAIL");
    assert_eq!(detail_key("scanlib"), "SCANLIB_DETAIL");
}
