//! File-object intrinsics (`FileIntrinsics`), one file per method.
//! Every call re-resolves the file by path.

use greybel_interpreter::{Native, NativeFuture, NativeOutput, Value, Vm};
use greyhack_behaviour::FileOwnership;

use crate::error::IntrinsicError;
use crate::game::client::object::FileObject;
use crate::game::client::receiver::receiver_for;
use crate::game::client::server::{program, refuse, respond, server};
use crate::game::client::{object, receiver};
use crate::protocol::{FileEntries, FileSetGroup, FileSetOwner};

mod allow_import;
mod chmod;
mod copy;
mod date;
mod delete;
mod get_content;
mod get_files;
mod get_folders;
mod group;
mod has_permission;
mod is_binary;
mod is_folder;
mod is_symlink;
mod r#move;
mod name;
mod owner;
mod parent;
mod path;
mod permissions;
mod rename;
mod set_content;
mod set_group;
mod set_owner;
mod size;
mod symlink;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("copy", &copy::copy::NATIVE),
    ("move", &r#move::r#move::NATIVE),
    ("rename", &rename::rename::NATIVE),
    ("path", &path::path::NATIVE),
    ("parent", &parent::parent::NATIVE),
    ("name", &name::name::NATIVE),
    ("is_folder", &is_folder::is_folder::NATIVE),
    ("get_content", &get_content::get_content::NATIVE),
    ("set_content", &set_content::set_content::NATIVE),
    ("is_binary", &is_binary::is_binary::NATIVE),
    ("is_symlink", &is_symlink::is_symlink::NATIVE),
    ("has_permission", &has_permission::has_permission::NATIVE),
    ("delete", &delete::delete::NATIVE),
    ("get_folders", &get_folders::get_folders::NATIVE),
    ("get_files", &get_files::get_files::NATIVE),
    ("chmod", &chmod::chmod::NATIVE),
    ("permissions", &permissions::permissions::NATIVE),
    ("owner", &owner::owner::NATIVE),
    ("set_owner", &set_owner::set_owner::NATIVE),
    ("group", &group::group::NATIVE),
    ("set_group", &set_group::set_group::NATIVE),
    ("size", &size::size::NATIVE),
    ("date", &date::date::NATIVE),
    ("allow_import", &allow_import::allow_import::NATIVE),
    ("symlink", &symlink::symlink::NATIVE),
];

/// Methods available on FTP file objects (`FtpFileType`).
pub(crate) const FTP_METHODS: &[(&str, &Native)] = &[
    ("copy", &copy::copy::NATIVE),
    ("move", &r#move::r#move::NATIVE),
    ("rename", &rename::rename::NATIVE),
    ("path", &path::path::NATIVE),
    ("parent", &parent::parent::NATIVE),
    ("name", &name::name::NATIVE),
    ("is_folder", &is_folder::is_folder::NATIVE),
    ("is_binary", &is_binary::is_binary::NATIVE),
    ("is_symlink", &is_symlink::is_symlink::NATIVE),
    ("has_permission", &has_permission::has_permission::NATIVE),
    ("delete", &delete::delete::NATIVE),
    ("get_folders", &get_folders::get_folders::NATIVE),
    ("get_files", &get_files::get_files::NATIVE),
    ("permissions", &permissions::permissions::NATIVE),
    ("owner", &owner::owner::NATIVE),
    ("group", &group::group::NATIVE),
    ("size", &size::size::NATIVE),
    ("date", &date::date::NATIVE),
];

/// Returns file object `this` when its type provides `method`.
fn file<'v>(vm: &'v Vm, this: &Value, method: &str) -> Option<&'v FileObject> {
    receiver_for::<receiver::File>(vm, this, method)
}

/// Converts `files` into file objects with the same access mode as `file`.
fn file_objects(
    vm: &mut Vm,
    file: &FileObject,
    files: FileEntries,
) -> Result<Value, IntrinsicError> {
    let files = match files {
        FileEntries::Entries(files) => files,
        FileEntries::Null => return Ok(Value::Null),
        FileEntries::Failure(failure) => {
            return Ok(Value::from(crate::game::client::failure::failure_text(
                "", &failure,
            )?));
        }
    };
    let mut objects = Vec::with_capacity(files.len());

    for id in files {
        let child = file.with_object(object::handle(vm, id)?);

        objects.push(object::create_file(vm, child));
    }

    Ok(Value::List(vm.heap_mut().add_list(objects)))
}

/// Implements `set_owner` and `set_group` through `UpdateOwnerAbsolute` / `UpdateGroupAbsolute`.
fn change_ownership(
    vm: &Vm,
    this: &Value,
    owner: FileOwnership,
    name: &Value,
    recursive: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let method = match owner {
        FileOwnership::Owner => "set_owner",
        FileOwnership::Group => "set_group",
    };
    let (Some(name), Some(file)) = (name.as_str(), file(vm, this, method)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if let Err(refusal) = owner.check(name) {
        return refuse(method, &refusal);
    }

    let Some(recursive) = recursive.as_int() else {
        return Err(IntrinsicError::InvalidArguments(method));
    };
    let (file, recursive, server) = (file.reference(), recursive == 1, server(vm)?);
    let request = match owner {
        FileOwnership::Owner => server.send_file_set_owner(&FileSetOwner {
            program: program(vm)?,
            file,
            owner: name.into(),
            recursive,
        }),
        FileOwnership::Group => server.send_file_set_group(&FileSetGroup {
            program: program(vm)?,
            file,
            group: name.into(),
            recursive,
        }),
    };

    Ok(respond(vm, request, move |_, change| {
        change.response(method)
    }))
}
