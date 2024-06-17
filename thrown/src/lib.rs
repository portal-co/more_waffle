use std::{iter::once, mem::take};

use waffle::{
    util::new_sig, BlockTarget, ExportKind, Func, FuncDecl, FunctionBody, ImportKind, Module,
    Operator, SignatureData, Terminator, Type,
};
use waffle_ast::{
    add_op,
    fcopy::{obf_mod, DontObf, Obfuscate},
    results_ref_2,
};
pub fn bundle_fn(m: &mut Module, tys: &[Type]) -> Func {
    let s = new_sig(
        m,
        SignatureData {
            params: tys.to_owned(),
            returns: tys.to_owned(),
        },
    );
    let mut f = FunctionBody::new(&m, s);
    f.set_terminator(
        f.entry,
        Terminator::Return {
            values: f.blocks[f.entry].params.iter().map(|a| a.1).collect(),
        },
    );
    return m
        .funcs
        .push(waffle::FuncDecl::Body(s, format!("bundle${:?}", tys), f));
}
pub struct Thrown {}
impl Obfuscate for Thrown {
    fn obf(
        &mut self,
        o: waffle::Operator,
        f: &mut waffle::FunctionBody,
        b: waffle::Block,
        args: &[waffle::Value],
        types: &[waffle::Type],
        module: &mut waffle::Module,
    ) -> anyhow::Result<(waffle::Value, waffle::Block)> {
        let mut types = types.to_owned();
        for r in types.iter_mut() {
            patch_ty(r)
        }
        let t2 = types.clone();
        if let Operator::Call { .. } | Operator::CallIndirect { .. } | Operator::CallRef { .. } = &o
        {
            types.push(Type::I32);
        }
        let (v, b) = DontObf {}.obf(o.clone(), f, b, args, &types, module)?;
        if let Operator::Call { .. } | Operator::CallIndirect { .. } | Operator::CallRef { .. } = o
        {
            let mut r = results_ref_2(f, v);
            let p = r.pop().unwrap();
            let nb = f.add_block();
            let o = f.add_block();
            f.set_terminator(
                b,
                Terminator::CondBr {
                    cond: p,
                    if_true: BlockTarget {
                        block: o,
                        args: vec![],
                    },
                    if_false: BlockTarget {
                        block: nb,
                        args: vec![],
                    },
                },
            );
            let mut d: Vec<_> = t2
                .iter()
                .map(|x| {
                    let t = vec![x.clone()];
                    let ov = add_op(
                        f,
                        args,
                        &t,
                        match x {
                            Type::I32 => Operator::I32Const { value: 0 },
                            Type::I64 => Operator::I64Const { value: 0 },
                            Type::F32 => Operator::F32Const { value: 0 },
                            Type::F64 => Operator::F64Const { value: 0 },
                            Type::V128 => Operator::V128Const { value: 0 },
                            Type::FuncRef => Operator::RefNull { ty: x.clone() },
                            Type::ExternRef => Operator::RefNull { ty: x.clone() },
                            Type::TypedFuncRef {
                                nullable,
                                sig_index,
                            } => Operator::RefNull { ty: x.clone() },
                        },
                    );
                    f.append_to_block(o, ov);
                    ov
                })
                .collect();
            d.push(p);
            f.set_terminator(o, Terminator::Return { values: d });
            let r = bundle_fn(module, &t2);
            let (r, nb) = DontObf {}.obf(
                Operator::Call { function_index: r },
                f,
                nb,
                args,
                &t2,
                module,
            )?;
            return Ok((r, nb));
        }
        return Ok((v, b));
    }
    fn obf_term(
        &mut self,
        mut t: waffle::Terminator,
        b: waffle::Block,
        f: &mut waffle::FunctionBody,
    ) -> anyhow::Result<()> {
        if let Terminator::Return { values } = &mut t {
            let v = add_op(f, &[], &[Type::I32], Operator::I32Const { value: 0 });
            values.push(v);
        }
        return DontObf {}.obf_term(t, b, f);
    }
}
fn patch_ty(ty: &mut Type) {
    if let Type::TypedFuncRef {
        nullable,
        sig_index,
    } = ty
    {
        *nullable = true;
    }
}
pub fn run(m: &mut Module) -> anyhow::Result<()> {
    for s in m.signatures.values_mut() {
        s.returns.push(waffle::Type::I32);
        for r in s.returns.iter_mut() {
            patch_ty(r)
        }
    }
    for f in m.funcs.values_mut() {
        if let Some(b) = f.body_mut() {
            b.rets.push(waffle::Type::I32);
            for r in b.rets.iter_mut() {
                patch_ty(r)
            }
        }
    }
    obf_mod(m, &mut Thrown {})?;
    for i in take(&mut m.imports) {
        if i.module == "env" && i.name.starts_with("invoke_") {
            if let ImportKind::Func(f) = i.kind {
                let mut x = m.exports.iter();
                let x = loop {
                    let Some(x) = x.next() else {
                        anyhow::bail!("table not found")
                    };
                    if x.name != "__indirect_function_table" {
                        continue;
                    }
                    let ExportKind::Table(t) = &x.kind else {
                        continue;
                    };
                    break *t;
                };
                let mut sp = m.exports.iter();
                let sp = loop {
                    let Some(x) = sp.next() else {
                        anyhow::bail!("table not found")
                    };
                    if x.name != "__stack_pointer" {
                        continue;
                    }
                    let ExportKind::Global(t) = &x.kind else {
                        continue;
                    };
                    break *t;
                };
                let mut set_threw = m.exports.iter();
                let set_threw =loop {
                    let Some(x) = set_threw.next() else {
                        anyhow::bail!("table not found")
                    };
                    if x.name != "_set_threw" {
                        continue;
                    }
                    let ExportKind::Func(t) = &x.kind else {
                        continue;
                    };
                    break *t;
                };
                let mut b = FunctionBody::new(&m, m.funcs[f].sig());
                let sp_save = add_op(&mut b, &[], &[m.globals[sp].ty], Operator::GlobalGet { global_index: sp });
                b.append_to_block(b.entry, sp_save);
                let index = b.blocks[b.entry].params[0].1;
                let others: Vec<_> = b.blocks[b.entry].params[1..].iter().map(|a| a.1).collect();
                let tys = m.funcs[f].sig();
                let tys = m.signatures[tys].returns.clone();
                let r = b.blocks[b.entry]
                    .params
                    .iter()
                    .map(|a| a.1)
                    .collect::<Vec<_>>();
                let v = add_op(
                    &mut b,
                    &r,
                    &tys,
                    Operator::CallIndirect {
                        sig_index: m.funcs[f].sig(),
                        table_index: x,
                    },
                );
                b.append_to_block(b.entry, v);
                let mut v = results_ref_2(&mut b, v);
                let w = v.last().unwrap().clone();
                let fl = b.add_block();
                let tr = b.add_block();
                b.set_terminator(
                    b.entry,
                    Terminator::CondBr {
                        cond: w,
                        if_true: BlockTarget {
                            block: tr,
                            args: vec![],
                        },
                        if_false: BlockTarget {
                            block: fl,
                            args: vec![],
                        },
                    },
                );
                b.set_terminator(fl, Terminator::Return { values: v.clone() });
                v.pop();
                let v2 = add_op(&mut b, &[], &[Type::I32], Operator::I32Const { value: 0 });
                b.append_to_block(tr, v2);
                let sp_rest = add_op(&mut b, &[sp_save], &[], Operator::GlobalSet { global_index: sp });
                b.append_to_block(tr, sp_rest);
                let v3 = add_op(&mut b, &[], &[Type::I32], Operator::I32Const { value: 1 });
                b.append_to_block(tr, v3);
                let v3 = add_op(&mut b, &[v3,v2], &[], Operator::Call { function_index: set_threw });
                b.append_to_block(tr, v3);
                b.set_terminator(
                    tr,
                    Terminator::Return {
                        values: v.iter().cloned().chain(once(v2)).collect(),
                    },
                );
                m.funcs[f] = FuncDecl::Body(m.funcs[f].sig(), m.funcs[f].name().to_owned(), b);
            }
        }
        m.imports.push(i)
    }
    return Ok(());
}
