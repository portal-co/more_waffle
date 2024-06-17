use std::borrow::Cow;

use waffle::{op_traits::rewrite_mem, Module, Operator, Type};
use waffle_ast::{
    add_op,
    fcopy::{obf_mod, DontObf, Obfuscate},
};

pub struct Grunk {}
impl Obfuscate for Grunk {
    fn obf(
        &mut self,
        mut o: waffle::Operator,
        f: &mut waffle::FunctionBody,
        b: waffle::Block,
        args: &[waffle::Value],
        types: &[waffle::Type],
        module: &mut waffle::Module,
    ) -> anyhow::Result<(waffle::Value, waffle::Block)> {
        let mut args = args.to_owned();
        rewrite_mem(&mut o, &mut args, |m, v| {
            if module.memories[*m].memory64 {
                if let Some(w) = v {
                    let o = add_op(f, &[*w], &[Type::I32], Operator::I32WrapI64);
                    f.append_to_block(b, o);
                    *w = o;
                }
            };
            Ok::<_, anyhow::Error>(())
        })?;
        match &o {
            Operator::MemoryGrow { mem } => {
                if module.memories[*mem].memory64 {
                    let o = add_op(f, &[args[0]], &[Type::I32], Operator::I32WrapI64);
                    f.append_to_block(b, o);
                    args[0] = o;
                }
            }
            _ => {}
        }
        let types = match o {
            Operator::MemorySize { mem } | Operator::MemoryGrow { mem } => {
                &[Type::I32]
            },
            _ => types
        };
        let (v, b) = DontObf {}.obf(o.clone(), f, b, &args, types, module)?;
        match o {
            Operator::MemorySize { mem } | Operator::MemoryGrow { mem } => {
                if module.memories[mem].memory64{
                    let v = add_op(f, &[v], &[Type::I64], Operator::I64ExtendI32U);
                    f.append_to_block(b, v);
                    return Ok((v,b));
                }
            }
            _ => {}
        }   
        Ok((v, b))
    }
}
pub fn grunk(m: &mut Module) -> anyhow::Result<()>{
    obf_mod(m, &mut Grunk{})?;
    for m in m.memories.values_mut(){
        m.memory64 = false;
    }
    Ok(())
}