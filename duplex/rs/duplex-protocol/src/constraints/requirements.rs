use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{
    Definition, Length, Message, Protocol, ResponseBody, Role, Span, TypeKind, TypeRef,
};
use crate::pattern::PatternTable;

impl ConstraintTable<'_> {
    pub(super) fn verify_requirements(ty: &TypeRef, span: Span) -> Result<(), DefinitionError> {
        match ty {
            TypeRef::String(requirements) => {
                Self::verify_length(requirements.length, span)?;

                match &requirements.pattern {
                    Some(pattern) => PatternTable::compile(pattern).map(drop).map_err(|error| {
                        DefinitionError::new(span, format!("the pattern `{pattern}` {error}"))
                    }),
                    None => Ok(()),
                }
            }
            TypeRef::Bytes(length) | TypeRef::List(_, length) | TypeRef::Record(_, length) => {
                Self::verify_length(*length, span)
            }
            TypeRef::Number(kind, bounds) => Definition::bounds_of(*kind, bounds)
                .map(drop)
                .map_err(|error| {
                    DefinitionError::new(span, format!("the bounds of {} {error}", kind.keyword()))
                }),
            TypeRef::Bool | TypeRef::Char | TypeRef::Nullable(_) | TypeRef::Declared(_) => Ok(()),
        }
    }

    pub(super) fn verify_length(length: Length, span: Span) -> Result<(), DefinitionError> {
        if length.max.is_some_and(|max| max < length.min) {
            return Err(DefinitionError::new(
                span,
                format!("the length `{length}` has a least above its most"),
            ));
        }

        Ok(())
    }

    pub(super) fn verify_limit(
        &self,
        protocol: &Protocol,
        message: &Message,
    ) -> Result<(), DefinitionError> {
        let name = &message.name;

        if message.receiver == Role::Client && message.response.is_none() && message.limit.is_some()
        {
            return Err(DefinitionError::new(
                message.span,
                format!(
                    "`{name}` is a notification the provider sends: the client sends nothing of it to limit"
                ),
            ));
        }

        if let Some(stream) = &message.stream {
            Self::verify_length(stream.stream.items, stream.span)?;
        }

        let limit = u64::from(protocol.limit_of(message));

        for (what, size) in self.client_frames(message) {
            if size > limit {
                return Err(DefinitionError::new(
                    message.span,
                    format!(
                        "{what} of `{name}` takes up to {size} bytes, past its limit of {limit}: give `{name}` a larger `limit`"
                    ),
                ));
            }
        }

        Ok(())
    }

    fn client_frames(&self, message: &Message) -> Vec<(&'static str, u64)> {
        let mut frames = Vec::new();

        if message.receiver == Role::Provider {
            frames.push(("the message", self.definition.bounded_size(&message.fields)));
            if let Some(field) = &message.stream {
                frames.push((
                    "an item of the stream",
                    self.upper_bound(&field.stream.item),
                ));
                frames.extend(
                    field
                        .stream
                        .end
                        .iter()
                        .map(|end| ("the end of the stream", self.upper_bound(end))),
                );
            }

            return frames;
        }

        let Some(response) = &message.response else {
            return frames;
        };

        match &response.body {
            ResponseBody::Value(ty) => frames.push(("the response", self.upper_bound(ty))),
            ResponseBody::Stream(stream) => {
                frames.push(("an item of the response", self.upper_bound(&stream.item)));
                frames.extend(
                    stream
                        .start
                        .iter()
                        .map(|start| ("the start of the response", self.upper_bound(start))),
                );
                frames.extend(
                    stream
                        .end
                        .iter()
                        .map(|end| ("the end of the response", self.upper_bound(end))),
                );
            }
        }

        frames.extend(
            response
                .failure
                .iter()
                .map(|failure| ("the failure", self.upper_bound(failure))),
        );

        frames
    }

    fn upper_bound(&self, ty: &TypeRef) -> u64 {
        if let Some(size) = self.definition.max_size(ty) {
            return size;
        }

        match ty {
            TypeRef::Declared(name) => match self.types.get(name.as_str()).map(|it| &it.kind) {
                Some(TypeKind::Struct(fields)) => self.definition.bounded_size(fields),
                _ => 0,
            },
            _ => 0,
        }
    }
}
