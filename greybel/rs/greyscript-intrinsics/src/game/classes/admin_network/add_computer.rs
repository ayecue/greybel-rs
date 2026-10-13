use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::COMPUTER_SERVICES;

use super::public_ip;
use crate::error::IntrinsicError;
use crate::game::client::failure::failure_value;
use crate::game::client::object;
use crate::game::client::object::Access;
use crate::game::client::options::{option, options, string_option};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{
    AdminAddComputer, ComputerCreation, ComputerOptions, ComputerUser, MiningHardware,
};

/// Creates a computer in this network and returns root access.
#[native(method)]
pub fn add_computer(
    vm: &mut Vm,
    this: &Value,
    #[name = "options"] options_value: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "add_computer";

    let Some(public_ip) = public_ip(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let map = options(options_value, NAME)?;
    let heap = vm.heap();
    let mut computer = ComputerOptions {
        name: string_option(heap, map, "name", NAME)?,
        lan_ip: string_option(heap, map, "lan_ip", NAME)?,
        root_password: string_option(heap, map, "root_password", NAME)?,
        users: None,
        services: None,
        is_switch: option(heap, map, "switch").is_some_and(|value| heap.is_truthy(value)),
        mining_hardware: MiningHardware {
            cpu_speed: option(heap, map, "cpuSpeed")
                .and_then(Value::as_number)
                .unwrap_or(1.0) as f32,
            cpu_cores: option(heap, map, "cpuCores")
                .and_then(Value::as_int)
                .unwrap_or(1) as u32,
            ram_model: option(heap, map, "ramModel")
                .and_then(Value::as_int)
                .unwrap_or(1) as u32,
            gpu_hashrate: option(heap, map, "gpuHashrate")
                .and_then(Value::as_number)
                .unwrap_or(0.0) as f32,
        },
    };

    if let Some(users) = option(heap, map, "users") {
        let Value::Map(users) = users else {
            return Err(IntrinsicError::InvalidArguments(NAME));
        };
        let users = users
            .entries(heap)
            .iter()
            .map(
                |(name, password)| match (name.as_str(), password.as_str()) {
                    (Some(name), Some(password)) => Ok(ComputerUser {
                        name: name.to_owned(),
                        password: password.to_owned(),
                    }),
                    _ => Err(IntrinsicError::InvalidArguments(NAME)),
                },
            )
            .collect::<Result<Vec<_>, _>>()?;

        computer.users = Some(users);
    }

    if let Some(services) = option(heap, map, "services") {
        let Value::List(services) = services else {
            return Err(IntrinsicError::InvalidArguments(NAME));
        };
        let mut names = Vec::new();

        for service in services.items(heap) {
            match service.as_str() {
                Some(name)
                    if COMPUTER_SERVICES
                        .iter()
                        .any(|service| service.name() == name) =>
                {
                    names.push(name.to_owned());
                }
                Some(name) => {
                    return Ok(NativeOutput::ready(format!(
                        "Error: unknown service {name}"
                    )));
                }
                None => return Err(IntrinsicError::InvalidArguments(NAME)),
            }
        }

        computer.services = Some(names);
    }

    let message = AdminAddComputer {
        program: program(vm)?,
        public_ip: public_ip.into(),
        computer,
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_add_computer(&message),
        |vm, creation| match creation {
            ComputerCreation::Computer(id) => {
                let computer = Access::new(object::handle(vm, id)?);

                Ok::<_, IntrinsicError>(object::create_computer(vm, computer))
            }
            ComputerCreation::Failure(failure) => {
                failure_value("add_computer", &failure)
            }
        },
    ))
}
