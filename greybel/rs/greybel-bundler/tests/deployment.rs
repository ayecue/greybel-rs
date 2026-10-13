use greybel_bundler::Deployment;

use crate::support::{auto_compiled, build, options, parse_errors};

#[test]
fn should_list_files_with_their_ingame_paths() {
    let output = build(options());
    let deployment = Deployment::new(&output);
    let paths: Vec<&str> = deployment
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect();

    assert_eq!(
        (
            paths,
            deployment.directory.as_str(),
            deployment.post_script.is_none()
        ),
        (vec!["/main.src", "/lib/tool.src"], "/root", true)
    );
}

#[test]
fn should_create_the_folders_before_the_files() {
    let output = build(options());
    let deployment = Deployment::new(&output);

    assert!(
        deployment
            .pre_script
            .contains("filePaths = [\"/main.src\",\"/lib/tool.src\"]")
    );
}

#[test]
fn should_compile_the_entries_after_the_files_with_auto_compile() {
    let output = build(auto_compiled(true));

    let deployment = Deployment::new(&output);

    let post = deployment.post_script.expect("auto-compile");
    let temporary = format!(
        "tmpDirectory = \"{}\"",
        output.destination.temporary_folder_name
    );

    assert!(
        post.contains("rootFilePaths = [\"/main.src\"]")
            && post.contains(&temporary)
            && post.contains("tmpFolder.path, 1)"),
        "{post}"
    );
}

#[test]
fn should_parse_the_deployment_scripts_as_greyscript() {
    let output = build(auto_compiled(false));

    let deployment = Deployment::new(&output);

    assert_eq!(
        (
            parse_errors(&deployment.pre_script),
            parse_errors(deployment.post_script.as_deref().unwrap_or(""))
        ),
        (None, None)
    );
}
