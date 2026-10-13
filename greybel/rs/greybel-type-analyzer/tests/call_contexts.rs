use crate::support::workspace::message_texts;
use crate::support::{PATH, greyscript, miniscript, with_document};

#[test]
fn should_type_each_call_by_its_arguments() {
    let source = "identity = function(x)
              return x
            end function
            a = identity(1)
            b = identity(\"hi\")
            pickName = function(user)
              return user.name
            end function
            first = {\"name\": \"neo\"}
            second = {\"name\": 42}
            c = pickName(first)
            d = pickName(second)
            pickFirst = function(items)
              return items[0]
            end function
            // @define {list<number>}
            items = []
            e = pickFirst(items)";

    let types = miniscript(source, &["a", "b", "c", "d", "e"]);

    assert_eq!(types, ["number", "string", "string", "number", "number"]);
}

#[test]
fn should_type_each_call_by_its_receiver() {
    let source = "obj = {}
            obj.name = \"neo\"
            obj.getSelf = function
              return self
            end function
            obj.makeChild = function
              return new self
            end function
            result = obj.getSelf()
            name = result.name
            child = obj.makeChild()
            childName = child.name
            first = {}
            first.name = \"neo\"
            first.getName = function
              return self.name
            end function
            second = {}
            second.name = 42
            second.getName = function
              return self.name
            end function
            a = first.getName()
            b = second.getName()";

    let types = miniscript(source, &["name", "childName", "a", "b"]);

    assert_eq!(types, ["string", "string", "string", "number"]);
}

#[test]
fn should_bind_an_explicit_self_to_the_receiver_or_first_argument() {
    let source = "obj = {}
            obj.name = \"neo\"
            obj.pickValue = function(self, value, fallback)
              return value
            end function
            obj.pickName = function(self, fallback)
              return self.name
            end function
            shifted = obj.pickValue(\"hello\", 42)
            detached = @obj.pickName
            bound = detached({\"name\": 42}, 0)";

    let types = miniscript(source, &["shifted", "bound"]);

    assert_eq!(types, ["string", "number"]);
}

#[test]
fn should_show_inherited_methods_the_callers_class() {
    let source = "A = {}
            A.Create = function
              return new self
            end function
            A.chain = function
              return self
            end function
            B = new A
            B.whatever = function
              return 1
            end function
            B.extra = \"hello\"
            b = B.Create
            whatever = b.whatever
            create = @b.Create
            chained = B.chain
            extra = chained.extra
            a = A.Create
            missing = a.whatever
            Mid = new A
            Mid.midMethod = function
              return \"mid\"
            end function
            Leaf = new Mid
            Leaf.leafMethod = function
              return 2
            end function
            obj = Leaf.Create
            leaf = obj.leafMethod
            mid = obj.midMethod";

    let types = miniscript(
        source,
        &["whatever", "create", "extra", "missing", "leaf", "mid"],
    );

    assert_eq!(
        types,
        [
            "number",
            "function(): map",
            "string",
            "unknown",
            "number",
            "string"
        ]
    );
}

#[test]
fn should_combine_branches_and_helpers() {
    let source = "pick = function(left, right, first)
              if first then
                return left
              end if
              return right
            end function
            a = pick(1, \"x\", 1)
            maybeName = function(user, ok)
              if ok then
                return user.name
              end if
              return \"unknown\"
            end function
            person = {}
            person.name = \"neo\"
            b = maybeName(person, 1)
            inner = function(x)
              return x
            end function
            outer = function(y)
              return inner(y)
            end function
            c = outer(1)
            d = outer(\"hi\")
            e = inner(c)
            f = inner(e)
            obj = {}
            obj.name = \"neo\"
            obj.pickName = function
              return self.name
            end function
            obj.getName = function
              return self.pickName()
            end function
            g = obj.getName()";

    let types = miniscript(source, &["a", "b", "c", "d", "f", "g"]);

    assert_eq!(
        types,
        [
            "number or string",
            "string",
            "number",
            "string",
            "number",
            "string"
        ]
    );
}

#[test]
fn should_settle_recursive_calls() {
    let source = "rec = function(x, done)
              if done then
                return x
              end if
              return rec(x, 1)
            end function
            a = rec(1, 0)
            b = rec(\"hi\", 0)
            first = function(x, done)
              if done then
                return x
              end if
              return second(x, 1)
            end function
            second = function(x, done)
              if done then
                return x
              end if
              return third(x, 1)
            end function
            third = function(x, done)
              if done then
                return x
              end if
              return first(x, 1)
            end function
            c = first(1, 0)
            d = first(\"hi\", 0)";

    let types = miniscript(source, &["a", "b", "c", "d"]);

    assert_eq!(types, ["number", "string", "number", "string"]);
}

#[test]
fn should_give_maps_and_lists_made_in_a_call_to_it() {
    let source = "makeUser = function(user)
              return {\"name\": user.name}
            end function
            wrap = function(user)
              return makeUser(user)
            end function
            profile = function(user)
              return {\"profile\": {\"name\": user.name}}
            end function
            pickName = function(user)
              return profile(user).profile.name
            end function
            makeNames = function(user)
              return [user.name]
            end function
            pickFirst = function(user)
              names = makeNames(user)
              return names[0]
            end function
            first = {}
            first.name = \"neo\"
            second = {}
            second.name = 42
            a = makeUser(first).name
            b = makeUser(second).name
            c = wrap(first).name
            d = wrap(second).name
            e = pickName(first)
            f = pickName(second)
            g = makeNames(first)
            h = pickFirst(second)";

    let types = miniscript(source, &["a", "b", "c", "d", "e", "f", "g", "h"]);

    assert_eq!(
        types,
        [
            "string",
            "number",
            "string",
            "number",
            "string",
            "number",
            "list<string>",
            "number"
        ]
    );
}

#[test]
fn should_keep_what_a_closures_call_captured() {
    let source = "make = function()
              return function(x)
                return x
              end function
            end function
            f = make()
            a = f(1)
            b = f(\"hi\")
            capture = function(x)
              return function()
                return x
              end function
            end function
            g = capture(1)
            h = capture(\"hi\")
            c = g()
            d = h()
            member = function(user)
              return function()
                return user.name
              end function
            end function
            first = {}
            first.name = \"neo\"
            second = {}
            second.name = 42
            i = member(first)
            j = member(second)
            e = i()
            k = j()
            pass = function(fn)
              return @fn
            end function
            wrap = function(arg)
              localFn = capture(arg)
              return pass(@localFn)
            end function
            l = wrap(1)
            m = wrap(\"hi\")
            n = l()
            o = m()";

    let types = miniscript(source, &["f", "a", "b", "c", "d", "e", "k", "n", "o"]);

    assert_eq!(
        types,
        [
            "function(x: any): any",
            "number",
            "string",
            "number",
            "string",
            "string",
            "number",
            "number",
            "string"
        ]
    );
}

#[test]
fn should_pass_null_for_missing_arguments() {
    let source = "orDefault = function(value, fallback = \"none\")
              if value == null then
                return fallback
              end if
              return value
            end function
            a = orDefault(1)
            b = orDefault
            c = orDefault(null, 2)";

    let types = greyscript(source, &["a", "b", "c"]);

    assert_eq!(
        types,
        ["number or string", "null or string", "null or number"]
    );
}

#[test]
fn should_report_the_diagnostics_of_a_body_once() {
    let workspace = with_document(
        greybel_type_analyzer::Dialect::MiniScript,
        "f = function(x)
          y = missing
          return x
        end function
        a = f(1)
        b = f(\"a\")",
    );

    let messages = message_texts(&workspace, PATH);

    assert_eq!(messages, ["Unused local: y", "Cannot find \"missing\"."]);
}
