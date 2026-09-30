use crate::graph::{EffectKey, Graph, RawEffectKey, RawSignalKey, ScopeKey, SignalKey, graph};
use slotmap::Key;
use std::fmt::{self, Result, Write};
use std::mem;

pub struct Snapshot;

impl fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        graph().debug(f)
    }
}

#[derive(Clone, Copy)]
pub struct Indent {
    lvl: usize,
}

impl Indent {
    pub fn inc(self) -> Self {
        Self { lvl: self.lvl + 1 }
    }
}

impl fmt::Display for Indent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result {
        for _ in 0..self.lvl * 4 {
            write!(f, " ")?;
        }
        Ok(())
    }
}

impl Graph {
    fn debug_effect_body<W: Write>(
        &self,
        f: &mut W,
        effect_key: EffectKey,
        indent: Indent,
    ) -> Result {
        writeln!(f, "{indent}label=\"\"")?;
        writeln!(f, "{indent}style=\"rounded, filled\"")?;
        writeln!(f, "{indent}fillcolor=\"lightgray\"")?;
        write!(f, "{indent}")?;
        if self.state.effect.key == Some(effect_key) {
            writeln!(f, "color=red")?;
        } else {
            writeln!(f, "color=black")?;
        }
        let effect = &self[effect_key];
        #[cfg(debug_assertions)]
        let tooltip = format_args!(
            " tooltip=\"{}:{}:{}\"",
            effect.location.file(),
            effect.location.line(),
            effect.location.column()
        );
        #[cfg(not(debug_assertions))]
        let tooltip = "";
        writeln!(
            f,
            "{indent}{effect_key:?}o [label=e{}{tooltip} color=lightgray]",
            effect_key.1.0
        )?;
        Ok(())
    }

    fn debug_effect<W: Write>(&self, f: &mut W, effect_key: EffectKey, indent: Indent) -> Result {
        writeln!(f, "{indent}subgraph cluster_{effect_key:?} {{")?;
        self.debug_effect_body(f, effect_key, indent.inc())?;
        writeln!(f, "{indent}}}")?;
        Ok(())
    }

    fn debug_connections<W: Write>(
        &self,
        f: &mut W,
        scope_key: ScopeKey,
        indent: Indent,
    ) -> Result {
        for (i, _) in self[scope_key].effects.iter().enumerate() {
            let effect_key = EffectKey(scope_key, RawEffectKey(i));
            let effect = &self[effect_key];
            for signal_key in effect.reads.iter() {
                writeln!(
                    f,
                    "{indent}{signal_key:?} -> {effect_key:?}o \
                     [lhead=\"cluster_{effect_key:?}\"]"
                )?;
            }
            for signal_key in effect.writes.iter() {
                writeln!(
                    f,
                    "{indent}{effect_key:?}o -> {signal_key:?} \
                     [ltail=\"cluster_{effect_key:?}\"]"
                )?;
            }
        }
        Ok(())
    }

    fn debug_scope_body<W: Write>(&self, f: &mut W, scope_key: ScopeKey, indent: Indent) -> Result {
        write!(f, "{indent}")?;
        if self.state.effect.key.is_none() && self.state.scope.key == Some(scope_key) {
            writeln!(f, "color=red")?;
        } else {
            writeln!(f, "color=black")?;
        }
        writeln!(f, "{indent}label=r{:?}", scope_key.data())?;
        let scope = &self[scope_key];
        for (i, signal) in scope.signals.iter().enumerate() {
            let signal_key = SignalKey(scope_key, RawSignalKey(i));
            let dirty = self.batching.signal_cache.contains(&signal_key);
            let dirty_marker = if dirty { " color=red" } else { "" };
            #[cfg(debug_assertions)]
            let tooltip = format_args!(
                " tooltip=\"{}:{}:{}\"",
                signal.location.file(),
                signal.location.line(),
                signal.location.column()
            );
            #[cfg(not(debug_assertions))]
            let tooltip = "";
            writeln!(
                f,
                "{indent}{signal_key:?} [label=s{}{tooltip} style=filled fillcolor=black fontcolor=white{dirty_marker}]",
                signal_key.1.0
            )?;
        }
        writeln!(
            f,
            "{indent}r{:?}o [shape=point style=invis]",
            scope_key.data()
        )?;

        for (i, _) in scope.effects.iter().enumerate() {
            self.debug_effect(f, EffectKey(scope_key, RawEffectKey(i)), indent)?;
        }

        Ok(())
    }

    fn debug_scopes<W: Write>(&mut self, f: &mut W, indent: Indent) -> Result {
        for (root, scope) in &self.scopes {
            if scope.parent.is_some() {
                continue;
            }

            self.debugger.scope_stack.push((root, indent, false));

            while let Some((scope_key, indent, closing)) = self.debugger.scope_stack.pop() {
                if self.scopes.get(scope_key).is_none() {
                    continue;
                }

                if closing {
                    writeln!(f, "{indent}}}")?;
                    continue;
                }

                writeln!(f, "{indent}subgraph cluster_r{:?} {{", scope_key.data())?;

                self.debug_scope_body(f, scope_key, indent.inc())?;

                self.debugger.scope_stack.push((scope_key, indent, true));

                let mut debugger = mem::take(&mut self.debugger);
                for &child in self[scope_key].children.iter() {
                    if self.scopes.get(child).is_some() {
                        debugger.scope_stack.push((child, indent.inc(), false));
                    }
                }
                self.debugger = debugger;
            }
        }

        Ok(())
    }

    fn debug_body<W: Write>(&mut self, f: &mut W, indent: Indent) -> Result {
        writeln!(f, "{indent}compound=true")?;
        writeln!(f, "{indent}layout=dot")?;
        writeln!(f, "{indent}splines=ortho")?;
        self.debug_scopes(f, indent)?;
        for scope_key in self.scopes.keys() {
            self.debug_connections(f, scope_key, indent)?;
        }
        Ok(())
    }

    fn debug<W: Write>(&mut self, f: &mut W) -> Result {
        let indent = Indent { lvl: 0 };
        writeln!(f, "{indent}digraph {{")?;
        self.debug_body(f, indent.inc())?;
        writeln!(f, "{indent}}}")?;
        Ok(())
    }
}
