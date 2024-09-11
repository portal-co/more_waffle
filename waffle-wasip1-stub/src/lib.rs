use std::mem::take;

use waffle::{
    passes::mem_fusing::get_exports, ExportKind, FuncDecl, FunctionBody, ImportKind, Type,
};
use waffle_ast::add_op;

pub fn icify<'a>(m: &mut waffle::Module) -> anyhow::Result<()> {
    static mods: &[&str] = &["wasi_snapshot_preview1", "wasi_unstable"];
    let ex = get_exports(m);

    // let mut s = State::new(importmap, BTreeSet::new());
    for i in take(&mut m.imports) {
        if mods.contains(&i.module.as_str()) {
            let ImportKind::Func(f) = i.kind else {
                continue;
            };
            let sig = m.funcs[f].sig();
            let mut new = FunctionBody::new(&m, sig);

            let o = add_op(
                &mut new,
                &[],
                &[Type::I32],
                waffle::Operator::I32Const { value: 8 },
            );
            new.append_to_block(new.entry, o);
            let mut r = vec![o];
            if m.signatures[sig].returns.len() == 0 {
                r = vec![];
            }
            new.set_terminator(new.entry, waffle::Terminator::Return { values: r });

            m.funcs[f] = FuncDecl::Body(sig, m.funcs[f].name().to_owned(), new);
        } else {
            m.imports.push(i);
        }
    }
    for ex in take(&mut m.exports) {
        if ex.name == "_initialize" || ex.name == "_start" {
            if let ExportKind::Func(s) = ex.kind {
                waffle::util::add_start(m, s);
                continue;
            }
        }
        m.exports.push(ex);
    }
    return Ok(());
}
#[cfg(test)]
mod tests {
    use super::*;
}
