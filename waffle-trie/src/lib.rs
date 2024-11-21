use std::mem::take;

use anyhow::Context;
use trie_rs::map::{Trie, TrieBuilder};
use waffle::{
    util::new_sig, Block, BlockTarget, ExportKind, Func, FuncDecl, FunctionBody, ImportKind,
    MemoryArg, Module, Operator, Signature, SignatureData, Type, Value,
};
use waffle_ast::{add_op, Builder};

pub fn internal_trie(m: &Module) -> Trie<u8, Func> {
    let mut b = TrieBuilder::new();
    for f in m.funcs.iter() {
        b.push(m.funcs[f].name().as_bytes(), f);
    }
    return b.build();
}
pub fn export_trie(m: &Module) -> Trie<u8, Func> {
    let mut b = TrieBuilder::new();
    for f in m.exports.iter() {
        let ExportKind::Func(func) = &f.kind else {
            continue;
        };
        b.push(f.name.as_bytes(), *func);
    }
    return b.build();
}
pub fn tx<T>(
    m: &mut Module,
    t: &Trie<u8, T>,
    x: &[u8],
    f: &mut FunctionBody,
    k: Block,
    v: Value,
    s: &mut impl FnMut(&T, &mut Module) -> bool,
    go: &mut impl FnMut(&T, &mut FunctionBody, &mut Module, Block) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    if let Some(0) = x.last() {
        if let Some(x) = t.exact_match(&x[..(x.len() - 2)]) {
            if s(x, m) {
                return go(x, f, m, k);
            }
            return Ok(());
        }
    }
    let p = t
        .postfix_search::<Vec<_>, _>(x)
        .filter(|a| s(a.1, m))
        .map(|_| ())
        .collect::<Vec<_>>()
        .len();
    if p == 0 {
        return Ok(());
    }
    let ty = f.values[v]
        .ty(&f.type_pool)
        .context("in getting the type")?;
    let mem = m
        .exports
        .iter()
        .find_map(|x| {
            if x.name == "memory" {
                match &x.kind {
                    waffle::ExportKind::Memory(m) => Some(*m),
                    _ => None,
                }
            } else {
                None
            }
        })
        .context("in getting the main memory")?;
    let a = add_op(
        f,
        &[v],
        &[ty.clone()],
        Operator::I32Load8U {
            memory: MemoryArg {
                memory: mem,
                align: 0,
                offset: x.len() as u64,
            },
        },
    );
    f.append_to_block(k, a);
    let xs = 0..=255;
    let xs = xs.map(|i| {
        let mut x = x.to_vec();
        x.push(i);
        let b = f.add_block();
        tx(m, t, &x, f, b, v, s, go)?;
        anyhow::Ok(BlockTarget {
            block: b,
            args: vec![],
        })
    });
    let xs = xs.collect::<anyhow::Result<Vec<_>>>()?;
    let xa = xs[0].clone();
    f.set_terminator(
        k,
        waffle::Terminator::Select {
            value: a,
            targets: xs,
            default: xa,
        },
    );
    Ok(())
}
pub fn emit(m: &mut Module) -> anyhow::Result<()> {
    let t_internal = internal_trie(&m);
    let t_export = export_trie(&m);
    for i in take(&mut m.imports) {
        if let Some(a) = i.module.strip_prefix("dyn_call") {
            let t = match a {
                "func" => &t_internal,
                "export" => &t_export,
                _ => {
                    m.imports.push(i);
                    continue;
                }
            };
            if let ImportKind::Func(f) = i.kind {
                let s = m.signatures[m.funcs[f].sig()].clone();
                let SignatureData::Func { params, returns } = s else{
                    anyhow::bail!("not a func")
                };
                let s = new_sig(
                    m,
                    SignatureData::Func {
                        returns: returns,
                        params: params[1..].iter().cloned().collect(),
                    },
                );
                let os = m.funcs[f].sig();
                let n = m.funcs[f].name().to_owned();
                let mut body = FunctionBody::new(&m, os);
                let e = body.entry;
                let mut p = body.blocks[e].params.iter();
                let pv = p.next().context("in getting the first param")?.1;
                let ps = p.map(|a| a.1).collect::<Vec<_>>();
                tx(
                    m,
                    &*t,
                    &[],
                    &mut body,
                    e,
                    pv,
                    &mut |a,m| m.funcs[*a].sig() == s,
                    &mut |f, b, m, k| {
                        b.set_terminator(
                            k,
                            waffle::Terminator::ReturnCall {
                                func: *f,
                                args: ps.clone(),
                            },
                        );
                        Ok(())
                    },
                )?;
                m.funcs[f] = FuncDecl::Body(os, n, body);
                continue;
            }
        }
        m.imports.push(i)
    }
    Ok(())
}
