use std::fmt;

use value::Value;

use crate::{
    expression::{Block, Predicate, Resolved},
    state::{ExternalEnv, LocalEnv},
    value::VrlValueConvert,
    BatchContext, Context, Expression, TypeDef,
};

#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub predicate: Predicate,
    pub consequent: Block,
    pub alternative: Option<Block>,
}

impl Expression for IfStatement {
    fn resolve(&self, ctx: &mut Context) -> Resolved {
        let predicate = self
            .predicate
            .resolve(ctx)
            .expect("predicate must not be fallible")
            .try_boolean()
            .expect("predicate must be boolean");

        match predicate {
            true => self.consequent.resolve(ctx),
            false => self
                .alternative
                .as_ref()
                .map(|block| block.resolve(ctx))
                .unwrap_or(Ok(Value::Null)),
        }
    }

    fn resolve_batch(&self, ctx: &mut BatchContext) {
        self.predicate.resolve_batch(ctx);
        let mut predicates = {
            let mut moved = vec![Ok(Value::Null); ctx.len()];
            std::mem::swap(ctx.resolved_values_mut(), &mut moved);
            moved
        };

        let mut i = 0;
        let mut ctx_false = ctx.drain_filter(|_| {
            let predicate = {
                let mut moved = Ok(Value::Null);
                std::mem::swap(&mut predicates[i], &mut moved);
                moved
            };
            let predicate = predicate
                .expect("predicate must not be fallible")
                .try_boolean()
                .expect("predicate must be boolean");

            i += 1;

            !predicate
        });

        self.consequent.resolve_batch(ctx);
        if let Some(alternative) = &self.alternative {
            alternative.resolve_batch(&mut ctx_false);
        } else {
            for resolved in ctx_false.resolved_values_mut() {
                *resolved = Ok(Value::Null);
            }
        }

        ctx.extend(ctx_false);
    }

    fn type_def(&self, state: (&LocalEnv, &ExternalEnv)) -> TypeDef {
        let type_def = self.consequent.type_def(state);

        match &self.alternative {
            None => type_def.add_null(),
            Some(alternative) => type_def.merge_deep(alternative.type_def(state)),
        }
    }
}

impl fmt::Display for IfStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("if ")?;
        self.predicate.fmt(f)?;
        f.write_str(" ")?;
        self.consequent.fmt(f)?;

        if let Some(alt) = &self.alternative {
            f.write_str(" else")?;
            alt.fmt(f)?;
        }

        Ok(())
    }
}
