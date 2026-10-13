//! Shared build-test project with one file importing another.

pub const MAIN: &str = "import_code(\"lib/util.src\")\nprint(greet(\"Ada\"))\nprint(#envar NAME)\n";
pub const UTIL: &str = "greet = function(name)\n  return \"hello \" + name\nend function\n";
