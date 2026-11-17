//! A project folder for the code navigation tools.

use std::path::{Path, PathBuf};

pub const MAIN: &str = r#"import_code("lib.src")
greet = function(name)
	return App.utils.format("hi " + name)
end function
first = function()
	x = 1
	return x
end function
second = function()
	x = 2
	return x
end function
print greet("ada")
"#;

pub const LIB: &str = r#"App = {}
App.utils = {}
// Wraps a value in brackets.
// @param {string} value
// @return {string}
App.utils.format = function(value)
	return "[" + value + "]"
end function
"#;

/// A project folder with `main.src` importing `lib.src`.
pub fn project(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("greybel-mcp-code-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);

    std::fs::create_dir_all(&folder).expect("creates the folder");
    std::fs::write(folder.join("main.src"), MAIN).expect("writes");
    std::fs::write(folder.join("lib.src"), LIB).expect("writes");
    std::fs::write(folder.join("greybel.json"), "{}").expect("writes");

    folder
}

pub fn path(folder: &Path, file: &str) -> String {
    folder.join(file).display().to_string()
}
