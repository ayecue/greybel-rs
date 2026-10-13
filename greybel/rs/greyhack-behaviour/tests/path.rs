use greyhack_behaviour::path::{absolute, binary_name, file_name, parent, split, steps};

#[test]
fn should_split_like_grey_hack() {
    let paths = ["/a/b/", "a/b", "/", "a", ""];
    let [trailing, relative, root, single, empty] = paths.map(split);

    assert_eq!(trailing, Some(vec!["a", "b"]));
    assert_eq!(relative, Some(vec!["a", "b"]));
    assert_eq!(root, Some(vec!["/"]));
    assert_eq!(single, Some(vec!["a"]));
    assert_eq!(empty, None);
}

#[test]
fn should_name_the_file_of_a_path() {
    let path = "/home/user";
    let name = file_name(path);

    assert_eq!(name, "user");
}

#[test]
fn should_find_the_parent_like_grey_hack() {
    let paths = ["/home/user", "/home", "/", "home"];
    let [nested, top, root, relative] = paths.map(parent);

    assert_eq!(nested, "/home");
    assert_eq!(top, "/");
    assert_eq!(root, "/");
    assert_eq!(relative, "home");
}

#[test]
fn should_make_paths_absolute() {
    let cases = [
        ("~", "/", "bob"),
        ("~/x", "/", "root"),
        (".", "/tmp", "bob"),
        ("./a", "/tmp", "bob"),
        ("a/b/", "/tmp", "bob"),
        ("/a/../b", "/tmp", "bob"),
        ("..", "/tmp", "bob"),
        ("/", "/tmp", "bob"),
    ];
    let [
        home,
        root_home,
        current,
        in_current,
        relative,
        parent_step,
        up,
        root,
    ] = cases.map(|(path, current, user)| absolute(path, current, user));

    assert_eq!(home, "/home/bob");
    assert_eq!(root_home, "/root/x");
    assert_eq!(current, "/tmp");
    assert_eq!(in_current, "/tmp/a");
    assert_eq!(relative, "/tmp/a/b");
    assert_eq!(parent_step, "/b");
    assert_eq!(up, "/");
    assert_eq!(root, "/");
}

#[test]
fn should_resolve_a_relative_path_in_the_current_folder() {
    let paths = [("/etc", "/home"), ("docs", "/home/test"), ("docs", "/")]
        .map(|(path, current)| greyhack_behaviour::path::resolve(path, current));

    assert_eq!(paths, ["/etc", "/home/test/docs", "/docs"]);
}

#[test]
fn should_give_each_segment_with_the_path_through_it() {
    assert_eq!(
        steps("/a/b/"),
        vec![("a", "/a".to_owned()), ("b", "/a/b".to_owned())]
    );
    assert_eq!(steps(""), Vec::new());
}

#[test]
fn should_name_a_binary_by_removing_the_last_extension() {
    assert_eq!(binary_name("tool.src"), Some("tool"));
    assert_eq!(binary_name("a.b.src"), Some("a.b"));
    assert_eq!(binary_name("tool"), None);
    assert_eq!(binary_name(".src"), None);
}
