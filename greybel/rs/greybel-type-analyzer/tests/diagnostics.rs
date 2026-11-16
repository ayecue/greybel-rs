//! Diagnostics beyond syntax errors, filtered by kind.

use greybel_documents_memory_provider::MemoryProvider;
use greybel_type_analyzer::{Analyzer, DiagnosticKind, DiagnosticOptions, Dialect, Severity};

use crate::support::diagnostics::{
    TEMPLATE_BOUNDS, UNUSED, diagnostics, messages_of_kind, missing_files, owned, severities,
};
use crate::support::workspace::load;
use crate::support::{PATH, globals, uri, with_document};

#[test]
fn should_report_calls_with_more_arguments_than_parameters() {
    let source = r#"f = function(a, b = 1)
        end function
        f
        f(1, 2)
        f(1, 2, 3)
        m = {}
        m.g = function(self, a)
        end function
        m.g
        m.g(1, 2)
        g = @m.g
        g(m, 1)
        print("a", "b", "c")
        unknown_value = rnd
        unknown_value(1, 2, 3)
        either = @f
        either = function(a, b, c)
        end function
        either(1, 2, 3)"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::CallArityMismatch);

    assert_eq!(
        reported,
        owned(&[
            (5, "Expected at most 2 arguments but got 3."),
            (10, "Expected at most 1 arguments but got 2."),
            (13, "Expected at most 2 arguments but got 3."),
        ])
    );
}

#[test]
fn should_warn_about_arity_mismatches_by_default() {
    let kind = DiagnosticKind::CallArityMismatch;

    let severity = kind.default_severity();

    assert_eq!(severity, Severity::Warning);
}

#[test]
fn should_report_missing_files_where_greyscript_names_them() {
    let workspace = load(Dialect::GreyScript, &missing_files(), &[PATH]);

    let reported = messages_of_kind(&workspace, DiagnosticKind::MissingImport);

    assert_eq!(
        reported,
        owned(&[
            (1, "Cannot find the imported file \"lib.src\"."),
            (2, "Cannot find the imported file \"/other.src\"."),
        ])
    );
}

#[test]
fn should_report_missing_files_where_greybel_names_them() {
    let workspace = load(Dialect::Greybel, &missing_files(), &[PATH]);

    let reported = messages_of_kind(&workspace, DiagnosticKind::MissingImport);

    assert_eq!(
        reported,
        owned(&[(1, "Cannot find the imported file \"lib.src\".")])
    );
}

#[test]
fn should_report_statements_after_a_terminator_as_dead() {
    let source = "f = function\n  return 1\n  x = 2\n  y = 3\nend function\nwhile true\n  if true then\n    break\n  end if\n  z = 1\n  continue\n  print 1\nend while";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DeadCodeAfterTerminator);

    assert_eq!(
        reported,
        owned(&[
            (3, "Unreachable code after return."),
            (12, "Unreachable code after continue."),
        ])
    );
}

#[test]
fn should_report_statements_after_every_path_ended_as_dead() {
    let source = "f = function(x)\n  if x then\n    return 1\n  else\n    return 2\n  end if\n  print 3\n  print 4\nend function\nwhile true\nend while\nprint 5";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DeadCodeAfterTerminator);

    assert_eq!(
        reported,
        owned(&[
            (7, "Unreachable code after if."),
            (12, "Unreachable code after while.")
        ])
    );
}

#[test]
fn should_report_statements_after_a_call_that_does_not_return_as_dead() {
    let source = "ready = 1 or exit(\"never\")\nprint ready\nexit(\"done\")\nprint 1";
    let workspace = with_document(Dialect::GreyScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DeadCodeAfterTerminator);

    assert_eq!(reported, owned(&[(4, "Unreachable code after exit.")]));
}

#[test]
fn should_report_map_literals_spelling_a_key_twice() {
    let source = "m = {\"a\": 1, \"b\": 2, \"a\": 3, 1: 0, 1.0: 1}";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DuplicateKey);

    assert_eq!(
        reported,
        owned(&[
            (1, "Duplicate key \"a\" in the map literal."),
            (1, "Duplicate key 1.0 in the map literal."),
        ])
    );
}

#[test]
fn should_report_calling_or_indexing_what_cannot_be() {
    let source = r#"n = 1
        n(2)
        n
        x = n[0]
        f = function
        end function
        y = @f
        z = y[0]
        mixed = 1
        mixed = "a"
        w = mixed[0]
        w(1)
        nothing = null
        nothing.value"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = diagnostics(
        &workspace,
        &[
            DiagnosticKind::CallOfNonCallable,
            DiagnosticKind::IndexOfNonIndexable,
        ],
    );

    assert_eq!(
        reported,
        [
            (
                DiagnosticKind::CallOfNonCallable,
                2,
                "Cannot call a value of type number.".to_owned()
            ),
            (
                DiagnosticKind::IndexOfNonIndexable,
                4,
                "Cannot index a value of type number.".to_owned()
            ),
            (
                DiagnosticKind::IndexOfNonIndexable,
                8,
                "Cannot index a value of type null.".to_owned()
            ),
            (
                DiagnosticKind::CallOfNonCallable,
                12,
                "Cannot call a value of type string.".to_owned()
            ),
        ]
    );
}

#[test]
fn should_report_values_annotations_do_not_accept() {
    let source = r#"// @param {string} name
        // @param {number} count
        // @returns {string}
        greet = function(name, count = "many")
          if count then return 1
          return name
        end function
        greet("a", 2)
        greet(1, 2)
        greet(null, [])
        either = "a"
        if rnd then either = 1
        greet(either)
        // @vtype Box
        // @function put
        //   @param {list} items
        // @define {Box}
        box = {}
        box.put({})
        box.put([])"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = diagnostics(
        &workspace,
        &[
            DiagnosticKind::AnnotationSignatureMismatch,
            DiagnosticKind::CallSignatureMismatch,
        ],
    );

    assert_eq!(
        reported,
        [
            (
                DiagnosticKind::AnnotationSignatureMismatch,
                4,
                "@param \"count\" expects number but is inferred as string.".to_owned()
            ),
            (
                DiagnosticKind::AnnotationSignatureMismatch,
                5,
                "@returns \"greet\" expects string but is inferred as number.".to_owned()
            ),
            (
                DiagnosticKind::CallSignatureMismatch,
                9,
                "Argument 1 expects string but got number.".to_owned()
            ),
            (
                DiagnosticKind::CallSignatureMismatch,
                10,
                "Argument 2 expects number but got list<unknown>.".to_owned()
            ),
            (
                DiagnosticKind::CallSignatureMismatch,
                19,
                "Argument 1 expects list<any> but got map.".to_owned()
            ),
        ]
    );
}

#[test]
fn should_make_annotation_mismatches_errors_by_default() {
    let kind = DiagnosticKind::AnnotationSignatureMismatch;

    let severity = kind.default_severity();

    assert_eq!(severity, Severity::Error);
}

#[test]
fn should_report_unused_and_shadowing_locals() {
    let source = r#"total = 0
        helper = function(used, unused, _ignored, self)
          kept = used
          dropped = 1
          dropped = 2
          total = kept
          _scratch = 3
          callback = function
          end function
          map = {}
          map.key = 1
          return kept
        end function"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = diagnostics(
        &workspace,
        &[
            DiagnosticKind::UnusedLocal,
            DiagnosticKind::UnusedParameter,
            DiagnosticKind::Shadow,
        ],
    );

    assert_eq!(
        reported,
        [
            (
                DiagnosticKind::UnusedParameter,
                2,
                "Unused parameter \"unused\".".to_owned()
            ),
            (
                DiagnosticKind::UnusedLocal,
                4,
                "Unused local \"dropped\".".to_owned()
            ),
            (
                DiagnosticKind::UnusedLocal,
                6,
                "Unused local \"total\".".to_owned()
            ),
            (
                DiagnosticKind::Shadow,
                6,
                "\"total\" shadows an outer variable.".to_owned()
            ),
        ]
    );
}

#[test]
fn should_report_conditions_of_literals_as_constant() {
    let source = r#"if true then
        end if
        if 1 == 1 then
        else if "a" == "b" then
        end if
        if 1 < 2 < 3 then
        end if
        if not 0 then
        end if
        x = 1
        if x == 1 then
        end if
        if (1 == 1) and x then
        end if
        while true
          break
        end while
        if 1 == "1" then
        end if
        if 0.5 and 1 then
        end if"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::ConstantCondition);

    assert_eq!(
        reported,
        owned(&[
            (1, "Constant condition is always true."),
            (3, "Constant condition is always true."),
            (4, "Constant condition is always false."),
            (6, "Constant condition is always true."),
            (8, "Constant condition is always true."),
            (18, "Constant condition is always false."),
        ])
    );
}

#[test]
fn should_warn_about_constant_conditions_by_default() {
    let kind = DiagnosticKind::ConstantCondition;

    let severity = kind.default_severity();

    assert_eq!(severity, Severity::Warning);
}

#[test]
fn should_report_assigning_a_variable_to_itself() {
    let source = r#"x = 1
        x = x
        f = function
          x = x
          z = 1
          z = z
        end function
        globals.x = x"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::SelfAssignment);

    assert_eq!(
        reported,
        owned(&[
            (2, "\"x\" is assigned to itself."),
            (6, "\"z\" is assigned to itself.")
        ])
    );
}

#[test]
fn should_report_globals_read_before_assignment() {
    let source = r#"print a
        f = function
          print b
        end function
        a = 1
        b = 2
        while rnd
          if c then print c
          c = 1
        end while
        print d
        g = function
          globals.d = 1
        end function
        for i in [1]
        end for
        print i
        print e
        e = 1"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::UseBeforeDefinition);

    assert_eq!(
        reported,
        owned(&[
            (1, "\"a\" is used before it is assigned."),
            (11, "\"d\" is used before it is assigned."),
            (18, "\"e\" is used before it is assigned.")
        ])
    );
}

#[test]
fn should_count_globals_assigned_by_a_call_or_on_some_path_as_defined() {
    let source =
        "set = function\n  globals.d = 1\nend function\nset\nprint d\nif rnd then e = 1\nprint e";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::UseBeforeDefinition);

    assert_eq!(reported, owned(&[]));
}

#[test]
fn should_warn_about_assignments_replacing_native_functions() {
    let source = "print = function(x)\nend function\nname = \"x\"\nrun = function\n  globals.typeof = 1\nend function\nmine = 1";
    let workspace = with_document(Dialect::GreyScript, source);

    let reported = diagnostics(&workspace, &[DiagnosticKind::NativeOverride]);

    assert_eq!(
        reported,
        [
            (
                DiagnosticKind::NativeOverride,
                1,
                "Assignment to \"print\" replaces the native function of that name.".to_owned()
            ),
            (
                DiagnosticKind::NativeOverride,
                5,
                "Assignment to \"typeof\" replaces the native function of that name.".to_owned()
            ),
        ]
    );
    assert_eq!(
        severities(&workspace)
            .into_iter()
            .find(|&(kind, _)| kind == DiagnosticKind::NativeOverride)
            .map(|(_, severity)| severity),
        Some(Severity::Warning)
    );
}

#[test]
fn should_read_the_native_of_a_name_assigned_later() {
    let source = "print 1\nprint = 2";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = diagnostics(
        &workspace,
        &[
            DiagnosticKind::UseBeforeDefinition,
            DiagnosticKind::CallOfNonCallable,
        ],
    );

    assert_eq!(reported, []);
}

#[test]
fn should_report_isa_chains_leading_back_to_themselves() {
    let source = r#"Foo = {}
        Foo.__isa = Foo
        A = {}
        B = {}
        A.__isa = B
        B.__isa = A
        Bar = {}
        Bar = new Bar
        value = Foo.missing

        // @vtype First
        // @extends Second

        // @vtype Second
        // @extends First"#;
    let workspace = with_document(Dialect::MiniScript, source);

    let inheritance = messages_of_kind(&workspace, DiagnosticKind::CyclicInheritance);
    let definitions = messages_of_kind(&workspace, DiagnosticKind::CyclicTypeDefinition);

    assert_eq!(
        inheritance,
        owned(&[
            (2, "Cyclic inheritance: Foo -> Foo."),
            (5, "Cyclic inheritance: A -> B -> A."),
        ])
    );
    assert_eq!(
        definitions,
        owned(&[(11, "Cyclic type definition: First -> Second -> First.")])
    );
}

#[test]
fn should_report_imports_leading_back_to_the_document() {
    let provider = MemoryProvider::from_iter([
        (PATH, "#include \"lib.src\"\n#include \"other.src\""),
        ("/lib.src", "#include \"util.src\""),
        ("/util.src", "#include \"main.src\""),
        ("/other.src", "x = 1"),
    ]);
    let workspace = load(Dialect::Greybel, &provider, &[PATH]);

    let reported = messages_of_kind(&workspace, DiagnosticKind::CyclicImport);

    assert_eq!(
        reported,
        owned(&[(
            1,
            "Cyclic import: /main.src -> /lib.src -> /util.src -> /main.src."
        )])
    );
}

#[test]
fn should_make_cyclic_imports_errors_by_default() {
    let kind = DiagnosticKind::CyclicImport;

    let severity = kind.default_severity();

    assert_eq!(severity, Severity::Error);
}

#[test]
fn should_check_template_bounds_at_calls() {
    let workspace = with_document(Dialect::MiniScript, TEMPLATE_BOUNDS);

    let reported = messages_of_kind(&workspace, DiagnosticKind::TypeMismatch);

    assert_eq!(
        reported,
        owned(&[
            (
                8,
                "Type argument number does not satisfy constraint string for \"T\"."
            ),
            (
                15,
                "Type argument map does not satisfy constraint list<any> for \"T\"."
            ),
        ])
    );
}

#[test]
fn should_type_a_call_satisfying_a_template_bound() {
    let source = TEMPLATE_BOUNDS;

    let types = globals(Dialect::MiniScript, source, &["a"]);

    assert_eq!(types, ["string"]);
}

#[test]
fn should_warn_about_unused_parameters_and_locals() {
    let workspace = with_document(Dialect::MiniScript, UNUSED);

    let reported = severities(&workspace);

    assert_eq!(
        reported,
        [
            (DiagnosticKind::UnusedParameter, Severity::Warning),
            (DiagnosticKind::UnusedLocal, Severity::Warning),
        ]
    );
}

#[test]
fn should_override_severities_and_disable_kinds_by_options() {
    let workspace = with_document(Dialect::MiniScript, UNUSED);

    let mut options = DiagnosticOptions::default();

    options
        .severity_overrides
        .insert(DiagnosticKind::UnusedLocal, Severity::Hint);
    options
        .disabled_kinds
        .insert(DiagnosticKind::UnusedParameter);

    let reported: Vec<_> = options
        .apply(Analyzer::new(&workspace).diagnostics(&uri(PATH)))
        .into_iter()
        .map(|diagnostic| (diagnostic.kind, diagnostic.severity))
        .collect();

    assert_eq!(reported, [(DiagnosticKind::UnusedLocal, Severity::Hint)]);
}

#[test]
fn should_warn_about_import_code_that_grey_hack_reads_in_comments_and_strings() {
    let source = "import_code(\"/lib/a.src\")\n// import_code(\"/lib/b.src\")\n//import_code(\"/lib/c.src\")\ns = \"import_code(\"\"/x\"\")\"\nprint s // import_code(\"/lib/d.src\")";
    let workspace = with_document(Dialect::GreyScript, source);

    let reported: Vec<u32> = messages_of_kind(&workspace, DiagnosticKind::IgnoredImportCode)
        .into_iter()
        .map(|(line, _)| line)
        .collect();

    assert_eq!(reported, [2, 4, 5]);
}

#[test]
fn should_not_look_for_import_code_in_miniscript() {
    let source = "// import_code(\"/lib/b.src\")";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::IgnoredImportCode);

    assert!(reported.is_empty());
}

#[test]
fn should_give_every_diagnostic_kind_its_own_code() {
    let kinds = DiagnosticKind::ALL;

    let read: Vec<Option<DiagnosticKind>> = kinds
        .iter()
        .map(|kind| DiagnosticKind::from_code(kind.code()))
        .collect();

    let expected: Vec<_> = kinds.iter().copied().map(Some).collect();

    assert_eq!(read, expected);
}

#[test]
fn should_report_members_a_map_of_the_script_never_has() {
    let source = "M = {\"a\": 1}
        M.f = function
          return self.b
        end function
        x = M.c
        y = M.a + M.len
        T = new {\"e\": 1}
        z = T.d + T.e";
    let workspace = with_document(Dialect::MiniScript, source);

    let messages = messages_of_kind(&workspace, DiagnosticKind::UnknownMember);

    assert_eq!(
        messages,
        owned(&[
            (3, "Cannot find \"b\" in this map."),
            (5, "Cannot find \"c\" in this map."),
            (8, "Cannot find \"d\" in this map.")
        ])
    );
}

#[test]
fn should_report_api_types_used_as_values() {
    let source = "a = new shell
        shell.extra = 1
        b = missing";
    let workspace = with_document(Dialect::GreyScript, source);

    let messages = messages_of_kind(&workspace, DiagnosticKind::UnresolvedReference);

    assert_eq!(
        messages,
        owned(&[
            (
                1,
                "\"shell\" is a type, not a value: it can't be extended or created with \"new\"."
            ),
            (
                2,
                "\"shell\" is a type, not a value: it can't be extended or created with \"new\"."
            ),
            (3, "Cannot find \"missing\".")
        ])
    );
}

#[test]
fn should_not_report_members_of_maps_with_unknown_keys() {
    let source = "computed = {}
        computed[str(rnd)] = 1
        a = computed.x
        map.extra = 1
        b = {}.extra
        opaque = {}
        callback = globals[\"handler\"]
        callback(opaque)
        c = opaque.x
        parent = globals[\"Parent\"]
        child = new parent
        d = child.x
        Token = {}
        Token.show = function
          return self.text
        end function
        token = new Token
        token.text = \"t\"";
    let workspace = with_document(Dialect::MiniScript, source);

    let messages = messages_of_kind(&workspace, DiagnosticKind::UnknownMember);

    assert_eq!(messages, owned(&[]));
}

#[test]
fn should_not_report_members_of_the_custom_object() {
    let source = "get_custom_object.a = 1\nx = get_custom_object.setElsewhere";
    let workspace = with_document(Dialect::GreyScript, source);

    let messages = messages_of_kind(&workspace, DiagnosticKind::UnknownMember);

    assert_eq!(messages, owned(&[]));
}

#[test]
fn should_report_operators_whose_operand_types_never_work() {
    let source = "n = 1
        s = \"a\"
        l = [1]
        a = n - s
        b = l + n
        c = s * s
        d = -s
        e = null + n
        f = n < s
        g = n | s
        h = l * null
        i = (null * 2) + 1
        o = null
        if rnd then o = \"x\"
        fine = [n + null, s + n, n + s, l + l, l * 2, s - s, s - null, s < s, n == s, o + n, l + s]";
    let workspace = with_document(Dialect::Greybel, source);

    let messages = messages_of_kind(&workspace, DiagnosticKind::InvalidOperation);

    assert_eq!(
        messages,
        owned(&[
            (4, "\"-\" on number and string always gives null."),
            (
                5,
                "\"+\" on list<number> and number raises a type mismatch."
            ),
            (6, "\"*\" on string and string raises a type mismatch."),
            (7, "\"-\" on string always gives null."),
            (8, "\"+\" on null and number always gives null."),
            (9, "\"<\" on number and string always gives null."),
            (10, "\"|\" on number and string always gives null."),
            (11, "\"*\" on list<number> and null raises a type mismatch."),
            (12, "\"*\" on null and number always gives null.")
        ])
    );
}

#[test]
fn should_not_report_operators_on_values_of_unknown_type() {
    let source = "f = function(x)\n  return x - 1\nend function\nu = nope - \"a\"\nm = {}\nv = m.missing * \"a\"";
    let workspace = with_document(Dialect::MiniScript, source);

    let messages = messages_of_kind(&workspace, DiagnosticKind::InvalidOperation);

    assert_eq!(messages, owned(&[]));
}

#[test]
fn should_report_uses_of_deprecated_functions() {
    let source = r#"// Adds two numbers.
        // @deprecated Use add instead.
        //   It is removed in 2.0.
        sum = function(a, b)
          return a + b
        end function
        x = sum(1, 2)
        y = @sum
        M = {}
        // @deprecated
        M.old = function
        end function
        M.old
        M.old = 1"#;
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::Deprecated);

    assert_eq!(
        reported,
        owned(&[
            (
                7,
                "\"sum\" is deprecated. Use add instead.\nIt is removed in 2.0."
            ),
            (
                8,
                "\"sum\" is deprecated. Use add instead.\nIt is removed in 2.0."
            ),
            (13, "\"old\" is deprecated."),
        ])
    );
}

#[test]
fn should_report_assignments_to_constants_and_second_declarations() {
    let source = r#"let count = 0
        const LIMIT = 10
        LIMIT = 11
        LIMIT += 1
        let count = 2
        globals.LIMIT = 3
        f = function(a)
          let LIMIT = a
          return LIMIT
        end function"#;
    let workspace = with_document(Dialect::Greybel, source);

    let reported = diagnostics(
        &workspace,
        &[
            DiagnosticKind::ConstAssignment,
            DiagnosticKind::DuplicateDeclaration,
        ],
    );

    assert_eq!(
        reported,
        [
            (
                DiagnosticKind::ConstAssignment,
                3,
                "\"LIMIT\" is a constant; it can't be assigned again.".to_owned()
            ),
            (
                DiagnosticKind::ConstAssignment,
                4,
                "\"LIMIT\" is a constant; it can't be assigned again.".to_owned()
            ),
            (
                DiagnosticKind::DuplicateDeclaration,
                5,
                "\"count\" is already declared; remove \"let\" here.".to_owned()
            ),
            (
                DiagnosticKind::ConstAssignment,
                6,
                "\"LIMIT\" is a constant; it can't be assigned again.".to_owned()
            ),
        ]
    );
}

#[test]
fn should_report_assignments_that_create_variables_without_let_when_enabled() {
    let source = r#"x = 1
        x = 2
        let y = 1
        y = 2
        f = function(a)
          b = a
          a = 2
          for i in [1]
            i = 3
          end for
        end function"#;
    let workspace = with_document(Dialect::Greybel, source);
    let all = Analyzer::new(&workspace).all_diagnostics(&uri(PATH));
    let enabled = DiagnosticOptions {
        severity_overrides: [(DiagnosticKind::MissingLet, Severity::Warning)]
            .into_iter()
            .collect(),
        ..DiagnosticOptions::default()
    };

    let reported: Vec<(u32, String)> = all
        .iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::MissingLet)
        .map(|diagnostic| (diagnostic.range.start.line, diagnostic.message.clone()))
        .collect();
    let by_default = DiagnosticOptions::default()
        .apply(all.clone())
        .into_iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::MissingLet)
        .count();
    let when_enabled = enabled
        .apply(all)
        .into_iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::MissingLet)
        .count();

    assert_eq!(
        reported,
        owned(&[
            (1, "\"x\" is assigned without \"let\" or \"const\"."),
            (5, "\"f\" is assigned without \"let\" or \"const\"."),
            (6, "\"b\" is assigned without \"let\" or \"const\"."),
        ])
    );
    assert_eq!((by_default, when_enabled), (0, 3));
}

#[test]
fn should_take_let_before_destructuring_and_skip_its_temporaries() {
    let source = "let [a, [b]] = [1, [2]]\nprint a + b";
    let workspace = with_document(Dialect::Greybel, source);

    let missing: Vec<String> = Analyzer::new(&workspace)
        .all_diagnostics(&uri(PATH))
        .into_iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::MissingLet)
        .map(|diagnostic| diagnostic.message)
        .collect();

    assert_eq!(missing, Vec::<String>::new());
}

#[test]
fn should_report_nothing_for_links_after_a_null_optional_base() {
    let source = "none = null\na = none?.x.y\nb = none?.[0].z\nc = none?.run(1).w\nprint [a, b, c]";
    let workspace = with_document(Dialect::Greybel, source);

    let messages = crate::support::workspace::message_texts(&workspace, PATH);

    assert_eq!(messages, Vec::<String>::new());
}

#[test]
fn should_report_private_members_outside_the_methods_of_their_class() {
    let source = "class Dog\n  private tricks = []\n  learn(trick)\n    self.tricks.push(trick)\n    inner = function\n      return self.tricks\n    end function\n  end function\nend class\nclass Puppy extends Dog\n  show()\n    return self.tricks\n  end function\nend class\nrex = Dog.create\nprint rex.tricks\nrex.tricks = []\nprint Dog.tricks";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::PrivateMember);

    assert_eq!(
        reported,
        owned(&[
            (16, "\"tricks\" is private to \"Dog\"."),
            (17, "\"tricks\" is private to \"Dog\"."),
            (18, "\"tricks\" is private to \"Dog\"."),
        ])
    );
}

#[test]
fn should_report_members_an_implemented_type_has_but_the_class_has_not() {
    let source = "// @type Pet\n// @property {string} name\n// @property {number} age\n// @property {string} sound\nPet = {}\nclass Dog implements Pet\n  name = \"rex\"\n  age = \"old\"\nend class\nclass Cat implements Pet, Missing\n  name = \"tom\"\n  age = 2\n  sound = \"meow\"\nend class";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::MissingImplementation);

    assert_eq!(
        reported,
        owned(&[
            (6, "\"age\" is string in \"Dog\" but number in \"Pet\"."),
            (6, "\"Dog\" implements \"Pet\" but has no \"sound\"."),
            (10, "Cannot find the type \"Missing\"."),
        ])
    );
}

#[test]
fn should_report_implicit_any_only_when_enabled() {
    let source = "// @param {number} count\nf = function(count, name, size = 1, typed: string)\n  return unknownValue\nend function\nx = unknownValue\nlet y: number = unknownValue\ng = function(): any\n  return 1\nend function";
    let workspace = with_document(Dialect::Greybel, source);
    let all = Analyzer::new(&workspace).all_diagnostics(&uri(PATH));

    let reported: Vec<(u32, String)> = all
        .iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::ImplicitAny)
        .map(|diagnostic| (diagnostic.range.start.line, diagnostic.message.clone()))
        .collect();
    let by_default = DiagnosticOptions::default()
        .apply(all)
        .into_iter()
        .filter(|diagnostic| diagnostic.kind == DiagnosticKind::ImplicitAny)
        .count();

    assert_eq!(
        reported,
        [
            (2, "The return type is implicitly any.".to_owned()),
            (2, "The type of \"name\" is implicitly any.".to_owned()),
            (5, "The type of \"x\" is implicitly any.".to_owned()),
        ]
    );
    assert_eq!(by_default, 0);
}

#[test]
fn should_report_assignments_of_const_enum_entries() {
    let source = "const enum Port\n  Ssh = 22\nend enum\nenum Mode\n  Read\nend enum\nPort.Ssh = 2\nMode.Read = 1\nx = Port.Ssh";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::ConstAssignment);

    assert_eq!(
        reported,
        owned(&[(7, "Cannot assign to const enum entry \"Port.Ssh\".")])
    );
}

#[test]
fn should_report_optional_members_read_without_has_index() {
    let source = "type Options extends map\n  name: string\n  timeout?: number\nend type\nf = function(o: Options)\n  a = o.name\n  b = o.timeout\n  if o.hasIndex(\"timeout\") then c = o.timeout\n  d = o.hasIndex(\"timeout\") and o.timeout > 1\n  o.timeout = 3\n  e = o.timeout\nend function";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::OptionalMember);

    assert_eq!(
        reported,
        owned(&[(
            7,
            "\"timeout\" is optional; check hasIndex(\"timeout\") before reading it."
        )])
    );
}

#[test]
fn should_not_report_the_return_of_a_constructor_that_ends_with_exit() {
    let source = "class Dog\n  constructor(name)\n    if name == \"\" then return\n    exit(\"no\")\n  end function\nend class\nd = Dog.create(\"a\")";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DeadCodeAfterTerminator);

    assert_eq!(reported, owned(&[]));
}

#[test]
fn should_report_case_values_the_subject_never_equals() {
    let source = "f = function(p: \"tcp\"|\"udp\", n: number)\n  switch p\n    case \"tcp\", \"tpc\"\n      print 1\n    else\n      print 2\n  end switch\n  switch n\n    case 1, \"one\"\n      print 3\n  end switch\nend function";
    let workspace = with_document(Dialect::Greybel, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::ImpossibleCase);

    assert_eq!(
        reported,
        owned(&[
            (
                3,
                "The switch subject is \"tcp\" or \"udp\", so this case never matches."
            ),
            (
                9,
                "The switch subject is number, so this case never matches."
            ),
        ])
    );
}
