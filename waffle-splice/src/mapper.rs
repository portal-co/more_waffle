use std::iter::{empty, once};

use anyhow::Context;
use waffle::{BlockTarget, ExportKind, Operator, Terminator};
use waffle_ast::Handler;

// use crate::Handler;

pub struct Mapper {}
impl Handler for Mapper {
    fn modify(
        &mut self,
        m: &mut waffle::Module,
        x: &mut waffle::Operator,
        f: &mut waffle::FunctionBody,
        args: &mut [waffle::Value],
        k: &mut waffle::Block,
    ) -> anyhow::Result<()> {
        return waffle::op_traits::rewrite_mem(&mut x.clone(), &mut args.to_owned(), |mem, v| {
            let Some(v) = v else {
                return Ok(());
            };
            if !m.memories[*mem].memory64 {
                return Ok(());
            };
            let mut exp = format!("~{}", *mem);
            for x in m.exports.iter() {
                if let ExportKind::Memory(m) = &x.kind {
                    if *m == *mem {
                        exp = format!("{}", x.name)
                    }
                }
            }
            let name = format!("mapper_{}_{exp}", x.to_string().split_once("<").unwrap().0);
            let mut func = None;
            for x in m.exports.iter() {
                if x.name == name {
                    if let ExportKind::Func(f) = &x.kind {
                        func = Some(*f)
                    }
                }
            }
            let Some(func) = func else { return Ok(()) };
            let a1 = f.arg_pool.from_iter(empty());
            let t0 = f.type_pool.from_iter(once(waffle::Type::I64));
            let w = f.add_value(waffle::ValueDef::Operator(
                Operator::I64Const { value: 0 },
                a1,
                t0,
            ));
            f.append_to_block(*k, w);
            let a1 = f.arg_pool.from_iter(once(w).chain(once(*v)));
            let t1 = f.type_pool.from_iter(once(waffle::Type::I32));
            let w = f.add_value(waffle::ValueDef::Operator(Operator::I64GeS, a1, t1));
            f.append_to_block(*k, w);
            let n = f.add_block();
            let o = f.add_block();
            f.set_terminator(
                *k,
                Terminator::CondBr {
                    cond: w,
                    if_true: BlockTarget {
                        block: o,
                        args: vec![],
                    },
                    if_false: BlockTarget {
                        block: n,
                        args: vec![],
                    },
                },
            );
            *k = n;
            let n = f.add_value(waffle::ValueDef::Operator(Operator::I64Sub, a1, t0));
            f.append_to_block(o, n);
            let mut a2 = args.to_owned();
            a2[0] = n;
            f.set_terminator(o, Terminator::ReturnCall { func, args: a2 });
            return Ok(());
        });
    }
}
