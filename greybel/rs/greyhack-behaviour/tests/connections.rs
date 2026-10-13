use greyhack_behaviour::forbidden_param_token;

#[test]
fn should_find_the_first_forbidden_param_token() {
    assert_eq!(forbidden_param_token("plain"), None);
    assert_eq!(forbidden_param_token("abc[};"), Some("["));
    assert_eq!(forbidden_param_token("abc//def"), Some("//"));
}
