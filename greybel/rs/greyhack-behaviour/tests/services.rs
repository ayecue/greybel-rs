use greyhack_behaviour::{COMPUTER_SERVICES, LibraryKind, ServiceId};

#[test]
fn should_open_the_games_ports_for_computer_and_device_services() {
    let ports = [
        ServiceId::Ssh,
        ServiceId::Ftp,
        ServiceId::Http,
        ServiceId::Smtp,
        ServiceId::Repository,
        ServiceId::Rshell,
        ServiceId::Cam,
        ServiceId::SmartAppliance,
    ]
    .map(ServiceId::default_port);

    assert_eq!(ports, [22, 21, 80, 25, 1542, 1222, 37_777, 1883]);
    assert!(
        COMPUTER_SERVICES
            .iter()
            .all(|service| ports.contains(&service.default_port()))
    );
}

#[test]
fn should_have_a_definition_for_every_service() {
    for service in ServiceId::ALL {
        assert!(service.default_port() > 0, "{} has no port", service.name());
        assert_eq!(
            ServiceId::from_name(service.name()),
            Some(*service),
            "{} does not parse back",
            service.name()
        );

        let sql = matches!(
            service,
            ServiceId::Criminals
                | ServiceId::Employees
                | ServiceId::Students
                | ServiceId::BankAccount
        );
        let expected = if sql { None } else { Some(*service) };

        assert_eq!(
            ServiceId::from_name(&service.library()),
            expected,
            "library of {} does not parse back",
            service.name()
        );
    }
}

#[test]
fn should_map_the_sql_backed_services_to_libsql() {
    for service in [
        ServiceId::Criminals,
        ServiceId::Employees,
        ServiceId::Students,
        ServiceId::BankAccount,
    ] {
        assert_eq!(service.library(), "libsql.so");
        assert_eq!(service.library_kind(), LibraryKind::Sql);
    }
}

#[test]
fn should_use_the_games_service_ids() {
    let ids = [
        (ServiceId::Ssh, 0),
        (ServiceId::Ftp, 1),
        (ServiceId::Http, 2),
        (ServiceId::Smtp, 3),
        (ServiceId::Criminals, 4),
        (ServiceId::Employees, 5),
        (ServiceId::Students, 6),
        (ServiceId::BankAccount, 7),
        (ServiceId::Cam, 9),
        (ServiceId::Rshell, 10),
        (ServiceId::Repository, 11),
        (ServiceId::SmartAppliance, 12),
        (ServiceId::Adb, 13),
    ];

    for (service, id) in ids {
        assert_eq!(service.code(), id);
        assert_eq!(ServiceId::from_code(id), Some(service));
    }
}

#[test]
fn should_keep_the_sql_services_databases_in_the_server_config_folder() {
    let paths = [
        ServiceId::Employees,
        ServiceId::Students,
        ServiceId::Criminals,
    ]
    .map(ServiceId::config_path);

    assert_eq!(
        paths,
        [
            "/server/conf/employees.db",
            "/server/conf/students.db",
            "/server/conf/criminals.db"
        ]
    );
}
