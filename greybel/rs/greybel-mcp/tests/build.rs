//! Tests `build` with imported files.

use serde_json::json;

use crate::support::{Client, OPTIONS, run};

#[test]
fn should_build_a_file_with_what_it_imports_as_its_greybel_json_says() {
    let folder = std::env::temp_dir().join(format!("greybel-mcp-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);

    std::fs::create_dir_all(folder.join("lib")).expect("creates the folder");
    std::fs::write(
        folder.join("main.src"),
        "import_code(\"lib/util.src\")\nprint(1)",
    )
    .expect("writes");
    std::fs::write(folder.join("lib/util.src"), "x = 1").expect("writes");
    std::fs::write(
        folder.join("greybel.json"),
        r#"{ "build": { "autoCompile": false, "installer": { "active": false } } }"#,
    )
    .expect("writes");

    let entry = folder.join("main.src").display().to_string();
    let output = folder.join("out");

    run(async {
        let mut client = Client::connect(OPTIONS).await;

        let (code, _) = client.call("build", json!({ "path": entry })).await;
        let (written, failed) = client
            .call(
                "build",
                json!({ "path": entry, "output": output.display().to_string() }),
            )
            .await;

        assert!(
            code.contains("--- main.src\nimport_code(\"/root/lib/util.src\")"),
            "{code}"
        );
        assert!(code.contains("--- lib/util.src\nx=1"), "{code}");
        assert!(!failed, "{written}");
        assert_eq!(
            std::fs::read_to_string(output.join("lib/util.src")).expect("built"),
            "x=1"
        );
    });

    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn should_write_installers_that_compile_the_build_by_default() {
    let folder = std::env::temp_dir().join(format!("greybel-mcp-installer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);

    std::fs::create_dir_all(&folder).expect("creates the folder");
    std::fs::write(folder.join("main.src"), "print(1)").expect("writes");

    let entry = folder.join("main.src").display().to_string();

    run(async {
        let mut client = Client::connect(OPTIONS).await;

        let (code, failed) = client.call("build", json!({ "path": entry })).await;

        assert!(!failed, "{code}");
        assert!(code.contains("--- installer0.src\n"), "{code}");
    });

    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn should_build_with_the_settings_of_a_profile() {
    let folder = std::env::temp_dir().join(format!("greybel-mcp-profile-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);

    std::fs::create_dir_all(folder.join("lib")).expect("creates the folder");
    std::fs::write(folder.join("main.src"), "import_code(\"lib/util.src\")").expect("writes");
    std::fs::write(folder.join("lib/util.src"), "x = 1").expect("writes");
    std::fs::write(
        folder.join("greybel.json"),
        r#"{
            "build": { "autoCompile": false, "installer": { "active": false } },
            "profiles": { "home": { "build": { "ingameDirectory": "/home/user/" } } }
        }"#,
    )
    .expect("writes");

    let entry = folder.join("main.src").display().to_string();

    run(async {
        let mut client = Client::connect(OPTIONS).await;

        let (code, _) = client
            .call("build", json!({ "path": entry, "profile": "home" }))
            .await;
        let (missing, failed) = client
            .call("build", json!({ "path": entry, "profile": "staging" }))
            .await;

        assert!(
            code.contains("import_code(\"/home/user/lib/util.src\")"),
            "{code}"
        );
        assert!(failed, "{missing}");
        assert!(
            missing.contains("the project has no profile `staging`"),
            "{missing}"
        );
    });

    let _ = std::fs::remove_dir_all(&folder);
}
