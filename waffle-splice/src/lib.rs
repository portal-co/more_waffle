use std::collections::{BTreeMap, HashMap};
use waffle::{
    op_traits::{op_inputs, op_outputs},
    Func, FunctionBody, Module, Operator, SignatureData, ValueDef,
};
use waffle::{util::new_sig, Block, Value};
use waffle_ast::{fcopy::Obfuscate, results_ref_2};
pub mod mapper;

pub fn splice_op(
    m: &mut Module,
    mut x: Operator,
    // h: &mut impl Handler,
    o: &mut impl Obfuscate,
) -> anyhow::Result<Func> {
    let ins = op_inputs(&m, None, &x)?;
    let outs = op_outputs(&m, None, &x)?;
    let sig = SignatureData::Func {
        params: ins.to_vec(),
        returns: outs.to_vec(),
    };
    let sig = new_sig(m, sig);
    let mut body = FunctionBody::new(&m, sig);
    match x {
        Operator::Call { function_index } => body.set_terminator(
            body.entry,
            waffle::Terminator::ReturnCall {
                func: function_index,
                args: body.blocks[body.entry].params.iter().map(|a| a.1).collect(),
            },
        ),
        Operator::CallIndirect {
            sig_index,
            table_index,
        } => body.set_terminator(
            body.entry,
            waffle::Terminator::ReturnCallIndirect {
                sig: sig_index,
                table: table_index,
                args: body.blocks[body.entry].params.iter().map(|a| a.1).collect(),
            },
        ),
        Operator::CallRef { sig_index } => body.set_terminator(
            body.entry,
            waffle::Terminator::ReturnCallRef {
                sig: sig_index,
                args: body.blocks[body.entry].params.iter().map(|a| a.1).collect(),
            },
        ),
        _ => {
            let mut k = body.entry;
            let mut args: Vec<_> = body.blocks[body.entry].params.iter().map(|a| a.1).collect();
            // h.modify(m, &mut x, &mut body, &mut args, &mut k)?;
            // let vs = body.arg_pool.from_iter(args.into_iter());
            // let ts = body.type_pool.from_iter(outs.iter().map(|a| *a));
            // let a = body.add_value(crate::ValueDef::Operator(x, vs, ts));
            // body.append_to_block(k, a);
            let (a, k) = o.obf(x, &mut body, k, &args, &*&outs, m)?;
            // let mut b = vec![a];
            let b = results_ref_2(&mut body, a);
            body.set_terminator(k, waffle::Terminator::Return { values: b });
        }
    }
    return Ok(m
        .funcs
        .push(waffle::FuncDecl::Body(sig, x.to_string(), body)));
}
pub type SpliceCache = HashMap<Operator, Func>;
pub struct Splicer<O: Obfuscate,S: Obfuscate,F> {
    pub wrapped: O,
    pub splop: S,
    pub cache: SpliceCache,
    pub condition: F,
}
impl<O: Obfuscate,S: Obfuscate,F: FnMut(&Operator) -> bool> Obfuscate for Splicer<O,S,F> {
    fn obf(
        &mut self,
        mut o: Operator,
        f: &mut FunctionBody,
        b: Block,
        args: &[Value],
        types: &[waffle::Type],
        module: &mut Module,
    ) -> anyhow::Result<(Value, Block)> {
        if let Operator::Select = o {
            return self.wrapped.obf(o, f, b, args, types, module);
        }
        if !(self.condition)(&o){
            return self.wrapped.obf(o, f, b, args, types, module);
        }
        // if waffle::op_traits::op_rematerialize(&o) {
        //     return self.wrapped.obf(o, f, b, args, types, module);
        // }
        // if let Operator::Call { function_index } = o {
        //     return self.wrapped.obf(o, f, b, args, types, module);
        // }
        let fun = self.cache.get(&o);
        let fun = match fun {
            Some(f) => *f,
            None => {
                let s = splice_op(module, o.clone(), &mut self.splop)?;
                self.cache.insert(o.clone(), s);
                s
            }
        };
        return self.wrapped.obf(
            Operator::Call {
                function_index: fun,
            },
            f,
            b,
            args,
            types,
            module,
        );
    }
}
// pub fn splice_func(
//     m: &mut Module,
//     f: &mut FunctionBody,
//     k: &mut SpliceCache,
//     // h: &mut impl Handler,
//     ob: &mut impl Obfuscate,
// ) -> anyhow::Result<()> {
//     for v in f.values.values_mut() {
//         let ValueDef::Operator(o, _, _) = v else {
//             continue;
//         };
//         if let Operator::Select = o {
//             continue;
//         }
//         if waffle::op_traits::op_rematerialize(o) {
//             continue;
//         }
//         if let Operator::Call { function_index } = o {
//             continue;
//         }
//         let f = k.get(&*o);
//         let f = match f {
//             Some(f) => *f,
//             None => {
//                 let s = splice_op(m, o.clone(), &mut *ob)?;
//                 k.insert(o.clone(), s);
//                 s
//             }
//         };
//         *o = Operator::Call { function_index: f };
//     }
//     return Ok(());
// }
// pub fn splice_module(
//     m: &mut Module,
//     // h: &mut impl Handler,
//     o: &mut impl Obfuscate,
// ) -> anyhow::Result<()> {
//     let mut b = BTreeMap::new();
//     let mut cache = SpliceCache::new();
//     for (f, d) in m.funcs.entries() {
//         if let Some(d) = d.body() {
//             let d = d.clone();
//             b.insert(f, d);
//         }
//     }
//     //let c = b.clone();
//     for (k, v) in b.iter_mut() {
//         splice_func(m, v, &mut cache, &mut *o)?;
//     }
//     for (k, v) in b {
//         *m.funcs[k].body_mut().unwrap() = v;
//     }
//     return Ok(());
// }
