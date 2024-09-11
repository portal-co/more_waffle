use anyhow::Context;
use fcopy::{obf_fn, DontObf, Obfuscate};
use waffle::{
    op_traits::op_outputs, util::new_sig, Block, Export, Func, FunctionBody, Import, ImportKind,
    Memory, Module, Operator, SignatureData, Type, Value, ValueDef,
};

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
    Mount(Box<dyn Builder<Result = waffle::Value>>),
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
pub fn find<'a>(m: &'a Module, a: usize, size: usize, mem: Memory) -> Option<&'a [u8]> {
    return m.memories[mem]
        .segments
        .iter()
        .filter(|s| a >= s.offset)
        .find_map(|s| {
            let b = a - s.offset;
            let (_, t) = s.data.split_at_checked(b)?;
            if t.len() < size {
                return None;
            }
            return Some(&t[..size]);
        });
}
pub fn find_val<'a>(
    f: &FunctionBody,
    a: Value,
    size: Value,
    m: &'a Module,
    mem: Memory,
) -> Option<&'a [u8]> {
    let a = match f.values[a] {
        ValueDef::Operator(a, _, _) => match a {
            Operator::I32Const { value } => value as usize,
            Operator::I64Const { value } => value as usize,
            _ => return None,
        },
        _ => return None,
    };
    let size = match f.values[size] {
        ValueDef::Operator(a, _, _) => match a {
            Operator::I32Const { value } => value as usize,
            Operator::I64Const { value } => value as usize,
            _ => return None,
        },
        _ => return None,
    };
    find(m, a, size, mem)
}
pub trait Handler {
    fn modify(
        &mut self,
        m: &mut Module,
        x: &mut Operator,
        f: &mut FunctionBody,
        args: &mut [Value],
        k: &mut Block,
    ) -> anyhow::Result<()>;
}
pub struct Handle<H,O>{
    pub handler: H,
    pub obf: O,
}
impl<H: Handler,O: Obfuscate> Obfuscate for Handle<H,O>{
    fn obf(
        &mut self,
        mut o: Operator,
        f: &mut FunctionBody,
        mut b: Block,
        args: &[Value],
        types: &[Type],
        module: &mut Module,
    ) -> anyhow::Result<(Value, Block)> {
        let mut args = args.to_vec();
        self.handler.modify(module, &mut o, f, &mut args, &mut b)?;
        return self.obf.obf(o, f, b, &args, types, module);
    }
}
pub struct Stamp {
    pub func: Func,
}
impl Obfuscate for Stamp {
    fn obf(
        &mut self,
        o: Operator,
        f: &mut FunctionBody,
        b: Block,
        args: &[Value],
        types: &[Type],
        module: &mut Module,
    ) -> anyhow::Result<(Value, Block)> {
        if let Operator::Call { function_index } = &o {
            if let Some((module_name, name)) = module.imports.iter().find_map(|i| {
                if i.kind == ImportKind::Func(*function_index) {
                    Some((i.module.clone(), i.name.clone()))
                } else {
                    None
                }
            }) {
                let mem = module
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
                if module_name == "stamp" && name == "mark_export" {
                    if let Some(b) = find_val(f, args[0], args[1], module, mem)
                        .and_then(|a| std::str::from_utf8(a).ok())
                    {
                        let b = b.to_owned();
                        module.exports.push(Export {
                            name: b,
                            kind: waffle::ExportKind::Func(self.func),
                        })
                    }
                }
                if module_name == "stamp/import" {
                    if let Some(bm) = find_val(f, args[0], args[1], module, mem)
                        .and_then(|a| std::str::from_utf8(a).ok())
                    {
                        let bm = bm.to_owned();
                        if let Some(bn) = find_val(f, args[2], args[3], module, mem)
                            .and_then(|a| std::str::from_utf8(a).ok())
                        {
                            let bn = bn.to_owned();
                            let args = &args[4..];
                            let itys = args
                                .iter()
                                .flat_map(|a| f.values[*a].tys(&f.type_pool).iter())
                                .cloned()
                                .collect::<Vec<_>>();
                            let rets = types.to_owned();
                            let s = new_sig(
                                module,
                                SignatureData {
                                    params: itys,
                                    returns: rets,
                                },
                            );
                            let fx = module
                                .funcs
                                .push(waffle::FuncDecl::Import(s, format!("{bm}.{bn}")));
                            module.imports.push(Import {
                                module: bm,
                                name: bn,
                                kind: ImportKind::Func(fx),
                            });
                            return self.obf(
                                Operator::Call { function_index: fx },
                                f,
                                b,
                                args,
                                types,
                                module,
                            );
                        }
                    }
                }
            }
        }
        return DontObf {}.obf(o, f, b, args, types, module);
    }
}
pub fn stamp(m: &mut Module) -> anyhow::Result<()>{
    for f in m.funcs.iter().collect::<Vec<_>>(){
        obf_fn(m, f, &mut Stamp{func: f})?;
    }
    Ok(())
}