use waffle::{op_traits::op_outputs, Block, FunctionBody, Module, Operator, Type, Value, ValueDef};

pub mod bulk_memory_lowering;
pub mod fcopy;

pub fn add_op(f: &mut FunctionBody, args: &[Value], rets: &[Type], op: Operator) -> Value {
    let args = f.arg_pool.from_iter(args.iter().map(|a| *a));
    let rets = f.type_pool.from_iter(rets.iter().map(|a| *a));
    return f.add_value(ValueDef::Operator(op, args, rets));
}
pub trait Builder {
    type Result;
    fn build(
        &mut self,
        mo: &mut Module,
        func: &mut FunctionBody,
        k: Block,
    ) -> anyhow::Result<(Self::Result, Block)>;
}


pub fn results_ref_2(f: &mut FunctionBody, c: Value) -> Vec<Value> {
    let c = f.resolve_and_update_alias(c);
    let b = f.value_blocks[c];
    let mut v = vec![];
    let s = match f.values[c] {
        ValueDef::Operator(_, _1, _2) => f.type_pool[_2].to_owned(),
        _ => return vec![c],
    };
    if s.len() == 1 {
        return vec![c];
    }
    for (s, i) in s.iter().map(|a| *a).enumerate() {
        let w = f.add_value(ValueDef::PickOutput(c, s as u32, i));
        f.append_to_block(b, w);
        v.push(w);
    }

    return v;
}
pub enum Expr {
    Leaf(Value),
    Bind(Operator, Vec<Expr>),
    Mount(Box<dyn Builder<Result = waffle::Value>>)
}
impl Builder for Expr {
    type Result = waffle::Value;

    fn build(
        &mut self,
        mo: &mut waffle::Module,
        func: &mut waffle::FunctionBody,
        mut k: waffle::Block,
    ) -> anyhow::Result<(Self::Result, waffle::Block)> {
        match self {
            Expr::Leaf(a) => Ok((*a, k)),
            Expr::Bind(a, c) => {
                let mut r = vec![];
                for d in c.iter_mut() {
                    let (e, f) = d.build(mo, func, k)?;
                    k = f;
                    r.push(e);
                }
                let o = add_op(func, &r, &op_outputs(&mo, None, &a)?, a.clone());
                func.append_to_block(k, o);
                return Ok((o, k));
            }
            Expr::Mount(m) => m.build(mo, func, k),
        }
    }
}
