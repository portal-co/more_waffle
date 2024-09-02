use std::{collections::BTreeSet, mem::take};

use anyhow::Context;
// use more_waffle::{
//     copying::module::{ImportBehavior, Imports, State},
//     passes::mem_fusing::get_exports, x2i,
// };
use waffle::{passes::mem_fusing::get_exports, util::add_start, ExportKind, FuncDecl, FunctionBody, ImportKind, Module, Type};
use waffle_ast::add_op;
pub enum Mode<'a> {
    Str(&'a str),
    // Mod(Module<'static>),
    Nope,
}
pub fn icify<'a>(m: &mut waffle::Module, prefix: Mode<'a>, mods: &[&str]) -> anyhow::Result<()> {
    let ex = get_exports(m);
    // struct I {};
    // impl Imports for I {
    //     fn get_import(
    //         &mut self,
    //         a: &mut Module<'_>,
    //         m: String,
    //         n: String,
    //     ) -> anyhow::Result<Option<ImportBehavior>> {
    //         let ex = get_exports(a);
    //         if m == "target" {
    //             let k = ex.get(&n).context("in getting target export")?;
    //             return Ok(Some(ImportBehavior::Bind(x2i(k.clone()))));
    //         }
    //         if m.starts_with("$") {
    //             return Ok(Some(ImportBehavior::Passthrough(
    //                 m.split_at(1).1.to_owned(),
    //                 n,
    //             )));
    //         }
    //         return Ok(None);
    //     }
    // }
    // let importmap = I {};
    // let mut s = State::new(importmap, BTreeSet::new());
    for i in take(&mut m.imports) {
        if mods.contains(&i.module.as_str()) {
            let ImportKind::Func(f) = i.kind else {
                continue;
            };
            let sig = m.funcs[f].sig();
            let mut new = FunctionBody::new(&m, sig);
            match prefix {
                Mode::Str(prefix) => {
                    let ic = ex
                        .get(&format!("{prefix}{}", i.name))
                        .context("in getting export")?
                        .clone();
                    let ExportKind::Func(ic) = ic else {
                        anyhow::bail!("wrong type");
                    };
                    let params = new.blocks[new.entry].params.iter().map(|a| a.1).collect();
                    new.set_terminator(
                        new.entry,
                        waffle::Terminator::ReturnCall {
                            func: ic,
                            args: params,
                        },
                    );
                }
                Mode::Nope => {
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
                    new.set_terminator(new.entry, waffle::Terminator::Return { values: r })
                }
                // Mode::Mod(m) => todo!(),
            }

            m.funcs[f] = FuncDecl::Body(sig, m.funcs[f].name().to_owned(), new);
        } else {
            m.imports.push(i);
        }
    }
    for ex in take(&mut m.exports) {
        if ex.name == "_initialize" || ex.name == "_start" {
            if let ExportKind::Func(s) = ex.kind {
                add_start(m, s);
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
