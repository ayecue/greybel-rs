use greybel_interpreter::value::{CLASS_ID, Key};
use greybel_interpreter::{Value, Vm};
use greybel_symbols::names::ISA;

use super::prototype::prototype;
use super::{Access, FileObject, GameObject, Kind};
use crate::error::IntrinsicError;
use crate::game::game;
use crate::protocol::ObjectId;
use crate::runtime::Handle;

/// Returns the handle for server object `id` issued to the VM program.
pub(crate) fn handle(vm: &Vm, id: ObjectId) -> Result<Handle, IntrinsicError> {
    Ok(game(vm)?.server.object(id))
}

pub(crate) fn create(vm: &mut Vm, kind: Kind, object: GameObject) -> Value {
    prototype(vm, kind).instantiate(vm.heap_mut(), object)
}

/// Constructs the `kind` object for server ID `id`, storing `object` in its handle.
pub(crate) fn create_of(
    vm: &mut Vm,
    kind: Kind,
    object: fn(Handle) -> GameObject,
    id: ObjectId,
) -> Result<Value, IntrinsicError> {
    let object = object(handle(vm, id)?);

    Ok(create(vm, kind, object))
}

/// Constructs a shell object, selecting FTP access when applicable.
pub(crate) fn create_shell(vm: &mut Vm, access: Access) -> Value {
    let kind = access.kind(Kind::Shell, Kind::FtpShell);

    create(vm, kind, GameObject::Shell(access))
}

/// Constructs a computer object, selecting the FTP method subset when applicable.
pub(crate) fn create_computer(vm: &mut Vm, access: Access) -> Value {
    let kind = access.kind(Kind::Computer, Kind::FtpComputer);

    create(vm, kind, GameObject::Computer(access))
}

/// Constructs a file object, selecting the FTP method subset when applicable.
pub(crate) fn create_file(vm: &mut Vm, file: FileObject) -> Value {
    let kind = if file.ftp { Kind::FtpFile } else { Kind::File };

    create(vm, kind, GameObject::File(file))
}

/// Constructs an object exposing only methods granted by a zero-day payload.
pub(crate) fn create_partial(
    vm: &mut Vm,
    kind: Kind,
    object: GameObject,
    methods: &[String],
) -> Value {
    // Library payloads always link the full MetaLibType because `CreateMetaLib` ignores `partialObj`.
    // MetaLib therefore retains all methods; other object kinds expose only granted methods.
    let entries = kind
        .methods()
        .iter()
        .filter(|(name, _)| kind == Kind::MetaLib || methods.iter().any(|method| method == name))
        .map(|&(name, native)| (Value::from(name), vm.native_function(native)))
        .collect::<Vec<_>>();
    let isa = vm.heap_mut().add_sealed_map(entries);
    let class_id = match kind {
        Kind::Computer => "pcomputer",
        Kind::File => "pfile",
        Kind::Shell => "pshell",
        Kind::Router => "prouter",
        _ => kind.class_id(),
    };
    let isa_key = Key::new(ISA);
    let class_key = Key::new(CLASS_ID);
    let fields = [
        (&isa_key, Value::Map(isa)),
        (&class_key, Value::from(class_id)),
    ];

    Value::Map(vm.heap_mut().add_host_object(fields, object))
}
