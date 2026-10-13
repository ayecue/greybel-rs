//! Metadata-container consistency checks and snapshot splitting.

use std::collections::BTreeSet;

use meta_utils::{
    Argument, Container, Definition, Signature, TypeLookup, TypeMeta, Variation, VariationRegistry,
};
use serde_json::{Map, Value, json};

const LANGUAGE: &str = "en";
/// Definition-metadata receiver type name `self`.
const SELF: &str = "self";
const OTHER_LANGUAGE: &str = "xx";
/// Fallback `unknown` description from TypeScript `Signature.parseDescriptions` when `$meta` is absent.
const UNKNOWN_TYPE_DESCRIPTION: &str = "unknown";

/// Asserts that no consistency errors exist.
///
/// # Panics
/// Panics with all errors when the list is nonempty.
pub fn assert_no_mistakes(mistakes: &[String]) {
    assert!(
        mistakes.is_empty(),
        "{} mistake(s) in the data:\n{}",
        mistakes.len(),
        mistakes.join("\n")
    );
}

/// Finds relationship errors outside payload-schema validation: unknown types, dangling inheritance/descriptions,
/// duplicate arguments/types, invalid mutations/variations, and variation lists not ordered by `Variation`.
#[must_use]
pub fn consistency_mistakes(container: &Container) -> Vec<String> {
    let mut mistakes = Vec::new();
    let check_type =
        |signature: &Signature, place: &str, meta: &TypeMeta, mistakes: &mut Vec<String>| {
            let mut pending_metas = vec![meta];

            while let Some(meta) = pending_metas.pop() {
                let name = meta.name();
                let local =
                    name == SELF || signature.type_parameters().iter().any(|own| own == name);

                if !local && matches!(container.lookup_type(name), TypeLookup::Unknown) {
                    mistakes.push(format!("{place}: unknown type `{name}`"));
                }

                pending_metas.extend(meta.key_type());
                pending_metas.extend(meta.value_type());
            }
        };

    for signature in container.signatures() {
        let type_name = signature.type_name();

        if let Some(parent) = signature
            .extends()
            .filter(|parent| container.signature(parent).is_none())
        {
            mistakes.push(format!("{type_name}: extends unknown type `{parent}`"));
        }

        for language in signature.languages() {
            let descriptions = signature.descriptions(language).into_iter();

            for name in descriptions.flat_map(|own| own.members().keys()) {
                if !signature.definitions().contains_key(name) {
                    mistakes.push(format!(
                        "{type_name}.{name}: described in `{language}` but not defined"
                    ));
                }
            }
        }

        for (name, definition) in signature.definitions() {
            let place = format!("{type_name}.{name}");

            check_type(signature, &place, definition.type_meta(), &mut mistakes);

            let mut variations = vec![definition.variations()];

            if let Some(function) = definition.as_function() {
                variations.push(function.return_variations());

                let labels = function.arguments().iter().map(Argument::label);

                for label in duplicates(labels) {
                    mistakes.push(format!(
                        "{place}: argument `{label}` appears more than once"
                    ));
                }

                for argument in function.arguments() {
                    let names = argument.types().iter().map(TypeMeta::name);

                    for type_name in duplicates(names) {
                        mistakes.push(format!(
                            "{place}({}): type `{type_name}` appears more than once",
                            argument.label()
                        ));
                    }

                    for meta in argument.types() {
                        check_type(signature, &place, meta, &mut mistakes);
                    }
                }

                for type_name in duplicates(function.returns().iter().map(TypeMeta::name)) {
                    mistakes.push(format!(
                        "{place}: return type `{type_name}` appears more than once"
                    ));
                }

                for meta in function.returns() {
                    check_type(signature, &place, meta, &mut mistakes);
                }

                if let Some(mutation) = function.mutation() {
                    if signature.type_parameters().is_empty() {
                        mistakes.push(format!("{place}: mutates a type without type parameters"));
                    }

                    for (parameter, meta) in mutation.fixed_types() {
                        if !signature.type_parameters().contains(parameter) {
                            mistakes
                                .push(format!("{place}: mutates `{parameter}`, no type parameter"));
                        }

                        check_type(signature, &place, meta, &mut mistakes);
                    }
                }
            }

            for list in variations {
                if let Err(error) = container.variations().resolve(list) {
                    mistakes.push(format!("{place}: {error}"));
                }

                if !list.is_sorted() {
                    mistakes.push(format!("{place}: variations aren't in order"));
                }
            }
        }
    }

    for (name, group) in container.variations().groups() {
        if !group.is_sorted() {
            mistakes.push(format!(
                "variation group `{name}`: variations aren't in order"
            ));
        }
    }

    mistakes
}

fn duplicates<'a>(names: impl Iterator<Item = &'a str>) -> BTreeSet<&'a str> {
    let mut names_so_far = BTreeSet::new();

    names.filter(|name| !names_so_far.insert(*name)).collect()
}

/// Splits an observable container dump into `types`, per-signature `signature_<type>`, `queries`, and `by_id`
/// snapshots.
#[must_use]
pub fn snapshot_parts(container: &Container) -> Vec<(String, Value)> {
    let Value::Object(mut dump) = dump(container) else {
        unreachable!("a dump is an object")
    };
    let mut take = |key: &str| dump.remove(key).unwrap_or_default();
    let Value::Object(signatures) = take("signatures") else {
        unreachable!("signatures are an object")
    };
    let (queries, by_id) = (take("queries"), take("byId"));

    std::iter::once(("types".to_owned(), Value::Object(dump)))
        .chain(
            signatures
                .into_iter()
                .map(|(type_name, signature)| (format!("signature_{type_name}"), signature)),
        )
        .chain([("queries".to_owned(), queries), ("by_id".to_owned(), by_id)])
        .collect()
}

fn dump(container: &Container) -> Value {
    let all_types: Vec<&str> = container.signatures().map(Signature::type_name).collect();
    let signatures: Map<String, Value> = container
        .signatures()
        .map(|signature| {
            (
                signature.type_name().to_owned(),
                dump_signature(container, signature),
            )
        })
        .collect();

    let properties: BTreeSet<&str> = container
        .signatures()
        .flat_map(|signature| signature.definitions().keys().map(String::as_str))
        .collect();
    let queries: Map<String, Value> = properties
        .into_iter()
        .map(|property| {
            let query = |types: &[&str]| {
                let search: Vec<&str> = container
                    .search_definitions(types, property, LANGUAGE)
                    .into_keys()
                    .collect();
                let origin = container
                    .definition(types, property, LANGUAGE)
                    .map(|definition| definition.origin());

                (search, origin)
            };
            let each: Map<String, Value> = all_types
                .iter()
                .filter_map(|&type_name| {
                    let (search, origin) = query(&[type_name]);

                    (!search.is_empty() || origin.is_some()).then(|| {
                        (
                            type_name.to_owned(),
                            json!({ "search": search, "origin": origin }),
                        )
                    })
                })
                .collect();
            let (any_search, any_origin) = query(&["any"]);
            let (all_search, all_origin) = query(&all_types);

            (
                property.to_owned(),
                json!({
                    "any": { "search": any_search, "origin": any_origin },
                    "all": { "search": all_search, "origin": all_origin },
                    "each": each,
                }),
            )
        })
        .collect();

    let ids: BTreeSet<&str> = container
        .signatures()
        .flat_map(|signature| signature.definitions().values())
        .filter_map(|definition| definition.as_function().map(|function| function.id()))
        .collect();
    let by_id: Map<String, Value> = ids
        .into_iter()
        .map(|id| {
            let origins: Vec<&str> = container
                .definitions_by_id(id, LANGUAGE)
                .map(|definition| definition.origin())
                .collect();

            (id.to_owned(), json!(origins))
        })
        .collect();

    json!({
        "primitives": container.primitives().map(Signature::type_name).collect::<Vec<_>>(),
        "types": container.types().map(Signature::type_name).collect::<Vec<_>>(),
        "availableTags": container.available_tags(),
        "visibleTypes": container.visible_types().collect::<Vec<_>>(),
        "signatures": signatures,
        "queries": queries,
        "byId": by_id,
    })
}

fn dump_signature(container: &Container, signature: &Signature) -> Value {
    let mut description_order = Vec::new();
    let mut descriptions = Map::new();

    if let Some(own) = signature.descriptions(LANGUAGE) {
        let type_description = own.type_description().map_or_else(
            || json!({ "description": UNKNOWN_TYPE_DESCRIPTION, "example": null }),
            |description| json!(description),
        );
        let members = own
            .members()
            .iter()
            .map(|(name, description)| (name.clone(), json!(description)));

        for (name, description) in
            std::iter::once(("$meta".to_owned(), type_description)).chain(members)
        {
            description_order.push(json!(name));
            descriptions.insert(name, description);
        }
    }

    let definitions: Map<String, Value> = signature
        .definitions()
        .iter()
        .map(|(name, definition)| {
            (
                name.clone(),
                dump_definition(container.variations(), signature, name, definition),
            )
        })
        .collect();

    json!({
        "extends": signature.extends(),
        "hidden": signature.is_hidden(),
        "descriptionOrder": description_order,
        "descriptions": descriptions,
        "definitionOrder": signature.definitions().keys().collect::<Vec<_>>(),
        "definitions": definitions,
    })
}

fn dump_definition(
    variations: &VariationRegistry,
    signature: &Signature,
    name: &str,
    definition: &Definition,
) -> Value {
    let resolve = |list: &[Variation]| match variations.resolve(list) {
        Ok(resolution) => json!({ "ok": resolution }),
        Err(error) => json!({ "error": error.to_string() }),
    };
    let localization = signature
        .definition(name, LANGUAGE)
        .expect("the signature has the definition");
    let mut dump = json!({
        "origin": localization.origin(),
        "json": localization,
        "tags": definition.tags(),
        "typeString": definition.type_meta().to_string(),
        "variations": resolve(definition.variations()),
        "otherLanguageDescription": signature
            .definition(name, OTHER_LANGUAGE)
            .and_then(|other| other.description()),
    });

    if let Some(function) = definition.as_function() {
        let Value::Object(object) = &mut dump else {
            unreachable!("a definition dumps as an object")
        };

        object.insert("id".to_owned(), json!(function.id()));
        object.insert(
            "returnStrings".to_owned(),
            function.returns().iter().map(ToString::to_string).collect(),
        );
        object.insert(
            "returnVariations".to_owned(),
            resolve(function.return_variations()),
        );
        object.insert(
            "argStrings".to_owned(),
            function
                .arguments()
                .iter()
                .map(|argument| {
                    argument
                        .types()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Value>()
                })
                .collect(),
        );
    }

    dump
}
