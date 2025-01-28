use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    mem::{replace, take},
};

use waffle::{
    passes::tcore::results_ref_2, BlockTarget, ExportKind, FuncDecl, FunctionBody, GlobalData,
    ImportKind, Module, Operator, SignatureData, Terminator, Type,
};

pub fn unistub(m: &mut Module, prefix: &impl Display) {
    let mut se = BTreeMap::new();
    for mut i in take(&mut m.imports) {
        if let ImportKind::Func(f) = &mut i.kind {
            let st = format!("{prefix}.{}.{}", i.module, i.name);
            if let Some(g) = se.get(&st).cloned() {
                let mut g = m.funcs[g].clone();
                g.set_name(m.funcs[*f].name());
                m.funcs[*f] = g;
            } else {
                if let Some(ExportKind::Func(g)) = m.exports.iter().find_map(|x| {
                    if x.name == st {
                        Some(x.kind.clone())
                    } else {
                        None
                    }
                }) {
                    let s = m.funcs[*f].sig();

                    if let SignatureData::Func { params, returns } = m.signatures[s].clone() {
                        let n = m.funcs[*f].name().to_owned();
                        let h = take(&mut m.funcs[*f]);
                        let h = m.funcs.push(h);
                        let f = replace(f, h);
                        let l = m.globals.push(GlobalData {
                            ty: Type::I32,
                            value: Some(0),
                            mutable: true,
                        });
                        let mut b = FunctionBody::new(&m, s);
                        let args = b.blocks[b.entry]
                            .params
                            .iter()
                            .map(|a| a.1)
                            .collect::<Vec<_>>();
                        let lx = b.add_op(
                            b.entry,
                            Operator::GlobalGet { global_index: l },
                            &[],
                            &[Type::I32],
                        );
                        let p = b.add_block();
                        let q = b.add_block();
                        b.set_terminator(
                            b.entry,
                            Terminator::CondBr {
                                cond: lx,
                                if_true: BlockTarget {
                                    block: p,
                                    args: vec![],
                                },
                                if_false: BlockTarget {
                                    block: q,
                                    args: vec![],
                                },
                            },
                        );
                        b.set_terminator(
                            p,
                            Terminator::ReturnCall {
                                func: h,
                                args: args.clone(),
                            },
                        );
                        let one = b.add_op(q, Operator::I32Const { value: 1 }, &[], &[Type::I32]);
                        b.add_op(q, Operator::GlobalSet { global_index: l }, &[one], &[]);
                        let k = b.add_op(q, Operator::Call { function_index: g }, &args, &returns);
                        let k = results_ref_2(&mut b, k);
                        b.add_op(q, Operator::GlobalSet { global_index: l }, &[lx], &[]);
                        b.set_terminator(q, Terminator::Return { values: k });
                        m.funcs[f] = FuncDecl::Body(s, n, b);
                        se.insert(st, f);
                    }
                }
            }
        }
        m.imports.push(i);
    }
}
