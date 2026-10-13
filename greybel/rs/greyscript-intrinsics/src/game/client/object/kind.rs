use greybel_interpreter::Native;

/// Declares each object class, `classID`, and method table; `classes!` derives the remaining metadata.
macro_rules! classes {
    ($($class:ident => $id:literal, $methods:expr;)*) => {
        /// Object types with their type maps and assigned `classID` values.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub(crate) enum Kind {
            $($class,)*
        }

        impl Kind {
            pub(crate) const COUNT: usize = [$(Self::$class,)*].len();
            pub(crate) const ALL: [Self; Self::COUNT] = [$(Self::$class,)*];

            pub(crate) const fn class_id(self) -> &'static str {
                match self {
                    $(Self::$class => $id,)*
                }
            }

            /// Type-map methods registered by `GreyInterpreter`, keyed by name.
            pub(crate) fn methods(self) -> &'static [(&'static str, &'static Native)] {
                match self {
                    $(Self::$class => $methods,)*
                }
            }
        }
    };
}

classes! {
    Shell         => "shell",         crate::game::classes::shell::METHODS;
    Computer      => "computer",      crate::game::classes::computer::METHODS;
    File          => "file",          crate::game::classes::file::METHODS;
    Port          => "port",          crate::game::classes::port::METHODS;
    Router        => "router",        crate::game::classes::router::METHODS;
    Admin         => "admin",         crate::game::classes::admin::METHODS;
    AdminNetwork  => "adminNetwork",  crate::game::classes::admin_network::METHODS;
    AdminComputer => "adminComputer", crate::game::classes::admin_computer::METHODS;
    FtpShell      => "ftpshell",      crate::game::classes::shell::FTP_METHODS;
    FtpComputer   => "ftpComputer",   crate::game::classes::computer::FTP_METHODS;
    FtpFile       => "ftpFile",       crate::game::classes::file::FTP_METHODS;
    Metaxploit    => "MetaxploitLib", crate::game::classes::metaxploit::METHODS;
    MetaLib       => "MetaLib",       crate::game::classes::meta_lib::METHODS;
    NetSession    => "NetSession",    crate::game::classes::net_session::METHODS;
    Crypto        => "cryptoLib",     crate::game::classes::crypto::METHODS;
    AptClient     => "aptclientLib",  crate::game::classes::apt_client::METHODS;
    Blockchain    => "blockchainLib", crate::game::classes::blockchain::METHODS;
    Wallet        => "wallet",        crate::game::classes::wallet::METHODS;
    Coin          => "coin",          crate::game::classes::coin::METHODS;
    SubWallet     => "subwallet",     crate::game::classes::sub_wallet::METHODS;
    MetaMail      => "MetaMail",      crate::game::classes::meta_mail::METHODS;
    Service       => "service",       crate::game::classes::service::METHODS;
    Ctf           => "ctfEvent",      crate::game::classes::ctf::METHODS;
    SmartAppliance => "SmartAppliance", crate::game::classes::smart_appliance::METHODS;
    TrafficNet     => "TrafficNet",     crate::game::classes::traffic::METHODS;
    DebugLibrary   => "debugLibrary",   crate::game::classes::debug_library::METHODS;
}
