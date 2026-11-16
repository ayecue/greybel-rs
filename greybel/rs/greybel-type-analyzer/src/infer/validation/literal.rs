use greybel_core::syntax::greybel::{Enum, Switch};
use greybel_core::{LiteralValue, NodeId, NodeKind};

use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::infer::flow::KindSet;
use crate::types::{LiteralType, Type, TypeId, TypeNotation};

impl Inference<'_> {
    /// Registers the values of an `enum` as a type: the union of their literal types, used for the enum's name in
    /// annotations.
    pub(crate) fn register_enum_type(&mut self, enumeration: &Enum) {
        let ast = self.ast();
        let NodeKind::Assignment(assignment) = ast.kind(enumeration.assignment) else {
            return;
        };
        let (Some(name), NodeKind::Map(map)) =
            (ast.text(assignment.variable), ast.kind(assignment.init))
        else {
            return;
        };
        let values: Vec<NodeId> = ast
            .list(map.fields)
            .iter()
            .filter_map(|&field| match ast.kind(field) {
                NodeKind::MapEntry(entry) => Some(entry.value),
                _ => None,
            })
            .collect();
        let name = name.to_owned();

        if enumeration.constant {
            self.constant_enums.insert(name.clone());
        }

        let literals: Vec<TypeId> = values
            .into_iter()
            .filter_map(|value| self.literal_type_of(value))
            .collect();
        let ty = self.types.union_all(literals);

        self.enum_types.insert(name, ty);
    }

    /// Reports an assignment of an entry of a `const enum` (`Port.Ssh = 1`): builds write the entry's value where it
    /// is read.
    pub(crate) fn validate_constant_entry(&mut self, member: NodeId) {
        if self.constant_enums.is_empty() {
            return;
        }

        let ast = self.ast();
        let NodeKind::Member(node) = ast.kind(member) else {
            return;
        };
        let (Some(name), Some(entry)) = (ast.text(node.base), ast.text(node.identifier)) else {
            return;
        };

        if matches!(ast.kind(node.base), NodeKind::Identifier(_))
            && self.constant_enums.contains(name)
        {
            let message = format!("\"{name}.{entry}\" is an entry of a const enum.");

            self.report(DiagnosticKind::ConstAssignment, member, message);
        }
    }

    /// The literal type of a string or number literal node.
    pub(crate) fn literal_type_of(&mut self, node: NodeId) -> Option<TypeId> {
        let ast = self.ast();
        let NodeKind::Literal(literal) = ast.kind(node) else {
            return None;
        };
        let key = (self.current_document, node);

        if let Some(&ty) = self.literal_types.get(&key) {
            return ty;
        }

        let literal_type = match &literal.value {
            LiteralValue::String(text) => {
                LiteralType::String(text.read(ast.source()).into_owned().into_boxed_str())
            }
            &LiteralValue::Number(value) => {
                LiteralType::number(if literal.negated { -value } else { value })
            }
            _ => {
                self.literal_types.insert(key, None);

                return None;
            }
        };
        let ty = self.types.insert(Type::Literal(literal_type));

        self.literal_types.insert(key, Some(ty));

        Some(ty)
    }

    /// Reports `case` values the subject never equals: a literal missing from a subject that is a union of literal
    /// types, or a value of another kind than every variant of the subject.
    fn validate_case_values(&mut self, switch: &Switch, subject: &[TypeId]) {
        let is_known = |types: &crate::types::TypeTable, ty: TypeId| {
            !matches!(
                types.get(ty),
                Type::Any | Type::Unknown | Type::TypeParameter { .. }
            )
        };

        if subject.is_empty()
            || !subject
                .iter()
                .all(|&variant| is_known(&self.types, variant))
        {
            return;
        }

        let literal_subject = subject
            .iter()
            .all(|&variant| matches!(self.types.get(variant), Type::Literal(_)));
        let subject_kinds = subject.iter().fold(KindSet::NONE, |kinds, &variant| {
            kinds | KindSet::of(self.types.get(variant))
        });
        let values: Vec<NodeId> = switch
            .cases
            .iter()
            .filter(|case| !case.type_case)
            .flat_map(|case| case.values.clone())
            .collect();

        for value in values {
            let literal = self.literal_type_of(value);
            let impossible = match literal {
                Some(literal) if literal_subject => !subject.contains(&literal),
                Some(literal) => {
                    KindSet::of(self.types.get(literal)) & subject_kinds == KindSet::NONE
                }
                None => {
                    let Some(ty) = self.node_types[self.current_document as usize][value.index()]
                    else {
                        continue;
                    };
                    let variants: Vec<TypeId> = self.types.variants(ty).collect();

                    !variants.is_empty()
                        && variants
                            .iter()
                            .all(|&variant| is_known(&self.types, variant))
                        && variants.iter().all(|&variant| {
                            KindSet::of(self.types.get(variant)) & subject_kinds == KindSet::NONE
                        })
                }
            };

            if impossible {
                let subject = self.types.union_all(subject.iter().copied());
                let notation = TypeNotation::new(&self.model, &self.types);
                let message = format!(
                    "The switch subject is {}, so this case never matches.",
                    notation.display(subject)
                );

                self.report(DiagnosticKind::ImpossibleCase, value, message);
            }
        }
    }

    /// Reports a `switch` without `else` over a union of literal types that has no case for some of them.
    pub(crate) fn validate_switch_cases(&mut self, node: NodeId, switch: &Switch) {
        if self.context.current.is_some() {
            return;
        }

        // A variable subject is read through copies in the conditions, so its own node is inferred here.
        let subject = match self.node_types[self.current_document as usize][switch.subject.index()]
        {
            Some(subject) => subject,
            None => self.infer_value(switch.subject),
        };
        let variants: Vec<TypeId> = self.types.variants(subject).collect();

        self.validate_case_values(switch, &variants);

        if switch.fallback.is_some()
            || variants.is_empty()
            || !variants
                .iter()
                .all(|&variant| matches!(self.types.get(variant), Type::Literal(_)))
        {
            return;
        }

        let cases: Vec<TypeId> = switch
            .cases
            .iter()
            .filter(|case| !case.type_case)
            .flat_map(|case| case.values.clone())
            .filter_map(|value| self.literal_type_of(value))
            .collect();
        let notation = TypeNotation::new(&self.model, &self.types);
        let values_without_case: Vec<String> = variants
            .iter()
            .filter(|variant| !cases.contains(variant))
            .map(|&variant| notation.display(variant).to_string())
            .collect();

        if values_without_case.is_empty() {
            return;
        }

        self.report(
            DiagnosticKind::MissingCase,
            node,
            format!(
                "The switch has no case for {}.",
                values_without_case.join(", ")
            ),
        );
    }
}
