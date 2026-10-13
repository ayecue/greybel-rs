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
            (5, "Call expected at most 2 arguments but got 3."),
            (10, "Call expected at most 1 arguments but got 2."),
            (13, "Call expected at most 2 arguments but got 3."),
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
            (1, "Missing import: lib.src"),
            (2, "Missing import: /other.src"),
        ])
    );
}

#[test]
fn should_report_missing_files_where_greybel_names_them() {
    let workspace = load(Dialect::Greybel, &missing_files(), &[PATH]);

    let reported = messages_of_kind(&workspace, DiagnosticKind::MissingImport);

    assert_eq!(reported, owned(&[(1, "Missing import: lib.src")]));
}

#[test]
fn should_report_statements_after_a_terminator_as_dead() {
    let source = "f = function\n  return 1\n  x = 2\n  y = 3\nend function\nwhile true\n  if true then\n    break\n  end if\n  z = 1\n  continue\n  print 1\nend while";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DeadCodeAfterTerminator);

    assert_eq!(
        reported,
        owned(&[
            (3, "Dead code after return."),
            (12, "Dead code after continue."),
        ])
    );
}

#[test]
fn should_report_map_literals_spelling_a_key_twice() {
    let source = "m = {\"a\": 1, \"b\": 2, \"a\": 3, 1: 0, 1.0: 1}";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = messages_of_kind(&workspace, DiagnosticKind::DuplicateKey);

    assert_eq!(
        reported,
        owned(&[
            (1, "Duplicate map literal key: \"a\""),
            (1, "Duplicate map literal key: 1.0"),
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
                "Cannot call non-callable type number.".to_owned()
            ),
            (
                DiagnosticKind::IndexOfNonIndexable,
                4,
                "Cannot index non-indexable type number.".to_owned()
            ),
            (
                DiagnosticKind::IndexOfNonIndexable,
                8,
                "Cannot index non-indexable type null.".to_owned()
            ),
            (
                DiagnosticKind::CallOfNonCallable,
                12,
                "Cannot call non-callable type string.".to_owned()
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
        either = 1
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
                "@param count expects number but inferred string.".to_owned()
            ),
            (
                DiagnosticKind::AnnotationSignatureMismatch,
                5,
                "@returns greet expects string but inferred number.".to_owned()
            ),
            (
                DiagnosticKind::CallSignatureMismatch,
                9,
                "Argument 1 expected string but got number".to_owned()
            ),
            (
                DiagnosticKind::CallSignatureMismatch,
                10,
                "Argument 2 expected number but got list<unknown>".to_owned()
            ),
            (
                DiagnosticKind::CallSignatureMismatch,
                19,
                "Argument 1 expected list<any> but got map".to_owned()
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
                "Unused parameter: unused".to_owned()
            ),
            (
                DiagnosticKind::UnusedLocal,
                4,
                "Unused local: dropped".to_owned()
            ),
            (
                DiagnosticKind::UnusedLocal,
                6,
                "Unused local: total".to_owned()
            ),
            (
                DiagnosticKind::Shadow,
                6,
                "Shadowing outer binding: total".to_owned()
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
        owned(&[(2, "Self-assignment: x"), (6, "Self-assignment: z")])
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
        while true
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
            (1, "Used before defined: a"),
            (18, "Used before defined: e")
        ])
    );
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
            (2, "Cyclic inheritance: Foo -> Foo"),
            (5, "Cyclic inheritance: A -> B -> A"),
        ])
    );
    assert_eq!(
        definitions,
        owned(&[(11, "Cyclic type definition: First -> Second -> First")])
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
            "Cyclic import: /main.src -> /lib.src -> /util.src -> /main.src"
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
                "Type argument number does not satisfy constraint string for T."
            ),
            (
                15,
                "Type argument map does not satisfy constraint list<any> for T."
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
