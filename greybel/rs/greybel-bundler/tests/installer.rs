use std::fmt::Write as _;

use greybel_bundler::{BuildError, Installer};
use greybel_documents_memory_provider::MemoryProvider;

use crate::support::installer::{char_count, scripts};
use crate::support::{auto_compiled, build, build_with, options, parse_errors};

#[test]
fn should_write_one_installer_for_a_small_project() {
    let output = build(options());

    let scripts = scripts(&output);

    let script = &scripts[0];
    let header = format!(
        "BUILD_DESTINATION=\"/root\";BUILD_RESOURCE_DESTINATION=\"{}\";BUILD_AUTO_COMPILE=0;s = get_shell;",
        output.destination.resources
    );

    assert_eq!(
        (
            scripts.len(),
            script.name.as_str(),
            script.code.starts_with(&header),
            script.code.ends_with(";d;")
        ),
        (1, "installer0.src", true, true)
    );
}

#[test]
fn should_escape_quotes_and_import_code_in_contents() {
    let output = build(options());

    let scripts = scripts(&output);

    let code = &scripts[0].code;

    assert!(
        code.contains("import\"+\"_\"+\"code(\"\"/root/lib/tool.src\"\")")
            && code.contains("\"\"\"\"\"\"quoted"),
        "{code}"
    );
}

#[test]
fn should_parse_the_installer_as_greyscript() {
    let output = build(auto_compiled(true));

    let scripts = scripts(&output);

    assert_eq!(parse_errors(&scripts[0].code), None);
}

#[test]
fn should_split_files_over_installers_within_max_chars() {
    let code = (0..400).fold(String::new(), |mut code, line| {
        let _ = writeln!(code, "print \"line {line}\"");

        code
    }) + "import_code(\"tool.src\")";
    let provider: MemoryProvider = [("/p/main.src", code.as_str()), ("/p/tool.src", "print 1")]
        .into_iter()
        .collect();
    let output = build_with(options(), &provider, &["/p/main.src"]).expect("builds");
    let results: Vec<_> = (2500..2600)
        .map(|max_chars| Installer::new(&output, max_chars).scripts().expect("fits"))
        .collect();

    for (offset, scripts) in results.iter().enumerate() {
        let max_chars = 2500 + offset;

        assert!(scripts.len() > 2, "max_chars {max_chars}");
        for script in scripts {
            assert!(
                char_count(&script.code) <= max_chars,
                "{} over {max_chars}",
                script.name
            );
            assert_eq!(
                parse_errors(&script.code),
                None,
                "{} at {max_chars}:\n{}",
                script.name,
                script.code
            );
        }
    }
}

#[test]
fn should_delete_every_installer_after_it_ran() {
    let code = "print \"x\"\n".repeat(400);
    let provider: MemoryProvider = [("/p/main.src", code.as_str())].into_iter().collect();
    let output = build_with(options(), &provider, &["/p/main.src"]).expect("builds");
    let scripts = Installer::new(&output, 2600).scripts().expect("fits");

    assert!(
        scripts
            .iter()
            .all(|script| script.code.ends_with(";d") || script.code.ends_with(";d;"))
    );
}

#[test]
fn should_append_auto_compile_to_the_last_installer() {
    let output = build(auto_compiled(false));

    let scripts = scripts(&output);

    let code = &scripts.last().expect("one").code;
    let file = format!("m(\"{}/main.src\",", output.destination.resources);

    assert!(
        code.contains("BUILD_AUTO_COMPILE=1")
            && code.contains(&file)
            && code.contains("rootFilePaths = [\"/main.src\"]"),
        "{code}"
    );
}

#[test]
fn should_refuse_max_chars_that_cannot_fit_the_header() {
    let output = build(options());
    let result = Installer::new(&output, 100).scripts();

    assert!(matches!(
        result,
        Err(BuildError::MaxCharsTooSmall { max_chars: 100, .. })
    ));
}

#[test]
fn should_give_an_auto_compile_that_does_not_fit_its_own_installer_with_the_header() {
    let code = "print \"x\"\n".repeat(300);
    let provider: MemoryProvider = [("/p/main.src", code.as_str())].into_iter().collect();
    let output = build_with(auto_compiled(false), &provider, &["/p/main.src"]).expect("builds");

    let separate: Vec<String> = (3500..6000)
        .map(|max_chars| Installer::new(&output, max_chars).scripts().expect("fits"))
        .filter_map(|scripts| scripts.last().map(|script| script.code.clone()))
        .filter(|code| !code.contains("m(\""))
        .collect();

    assert!(!separate.is_empty());
    assert!(
        separate
            .iter()
            .all(|code| code.starts_with("BUILD_DESTINATION="))
    );
}
