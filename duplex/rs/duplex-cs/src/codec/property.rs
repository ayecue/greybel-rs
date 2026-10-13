use duplex_protocol::{Encoding, FieldPlan, Presence};

use crate::types;

/// Maps presence and nullability flags to field property representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// Required and non-null.
    Always,
    /// Optional and non-null when present; null means absent.
    Present,
    /// Required and nullable.
    NonNull,
    /// Optional and nullable, using `Optional<T?>`.
    Both,
}

impl Shape {
    pub(crate) fn of(presence: Presence) -> Self {
        match (presence.present, presence.non_null) {
            (None, None) => Self::Always,
            (Some(_), None) => Self::Present,
            (None, Some(_)) => Self::NonNull,
            (Some(_), Some(_)) => Self::Both,
        }
    }
}

/// Generated field property and constructor parameter metadata.
pub(crate) struct Property<'f, 'd> {
    pub(crate) plan: &'f FieldPlan<'d>,
    pub(crate) name: String,
    pub(crate) parameter: String,
}

impl<'f, 'd> Property<'f, 'd> {
    pub(crate) fn of(plan: &'f FieldPlan<'d>, owner: &str) -> Self {
        Self {
            plan,
            name: types::property(plan.field, owner),
            parameter: types::parameter(plan.field),
        }
    }

    pub(crate) fn shape(&self) -> Shape {
        Shape::of(self.plan.presence)
    }

    /// Returns value encoding for a present, non-null property.
    pub(crate) fn inner(&self) -> &Encoding<'d> {
        self.plan.ty.non_null()
    }

    /// Returns the property C# type.
    pub(crate) fn ty(&self) -> String {
        let inner = types::of(self.inner());

        match self.shape() {
            Shape::Always => inner,
            Shape::Present | Shape::NonNull => format!("{inner}?"),
            Shape::Both => format!("global::Duplex.Optional<{inner}?>"),
        }
    }

    /// Returns the C# value for an absent field.
    pub(crate) fn absent(&self) -> String {
        match (&self.plan.default, self.shape()) {
            (Some(value), Shape::Both) => format!(
                "new global::Duplex.Optional<{}?>({})",
                types::of(self.inner()),
                types::literal(value, &self.plan.ty)
            ),
            (Some(value), _) => types::literal(value, &self.plan.ty),
            (None, Shape::Both) => "default".to_owned(),
            (None, _) => "null".to_owned(),
        }
    }
}
