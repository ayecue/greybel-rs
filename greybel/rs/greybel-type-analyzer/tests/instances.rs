//! Tests `self` in the methods of maps with instances: the entries those methods set on instances.

use crate::support::instances::hovers;

#[test]
fn should_read_entries_methods_set_on_instances() {
    let source = "Token = {}
Token.init = function(text)
  result = new Token
  result.size = 1
  return result
end function
Token.grow = function
  self.extra = \"x\"
end function
Token.show = function
  return [self.|size, self.|extra]
end function
t = Token.init(\"a\")";

    let texts = hovers(source);

    assert_eq!(
        texts,
        [
            Some("size: number".to_owned()),
            Some("extra: string".to_owned())
        ]
    );
}

#[test]
fn should_skip_entries_set_on_instances_outside_methods() {
    let source = "Token = {}
Token.show = function
  return self.|outside
end function
t = new Token
t.outside = 1";

    let texts = hovers(source);

    assert_eq!(texts, [Some("outside: unknown".to_owned())]);
}

#[test]
fn should_read_entries_methods_of_a_parent_set_on_instances() {
    let source = "Animal = {}
Animal.init = function
  result = new Animal
  result.legs = 4
  return result
end function
Dog = new Animal
Dog.count = function
  return self.|legs
end function
a = new Animal
d = new Dog";

    let texts = hovers(source);

    assert_eq!(texts, [Some("legs: number".to_owned())]);
}

#[test]
fn should_keep_self_of_a_map_without_instances() {
    let source = "Store = {}
Store.add = function
  self.items = []
end function
Store.read = function
  return |self
end function";

    let texts = hovers(source);

    assert_eq!(
        texts,
        [Some(
            "self: map {\n  \"add\": function(): null,\n  \"read\": function(): map,\n  \"items\": list<unknown>\n}"
                .to_owned()
        )]
    );
}
