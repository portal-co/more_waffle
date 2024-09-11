use std::collections::BTreeMap;

use anyhow::Context;
use waffle::{
    entity::{EntityRef, PerEntity}, Func, FunctionBody, ImportKind, Module, Operator, Table, Terminator, ValueDef
};

pub struct MonomorphDecl {
    pub map: BTreeMap<(String, String), Func>,

}
pub struct Monomorph<'a> {
    pub decl: &'a MonomorphDecl,
    pub map: PerEntity<Func, Func>,
}
impl<'a> Monomorph<'a> {

    pub fn get(&mut self, f: Func, m: &mut Module) -> anyhow::Result<Func> {
        loop {
            if self.map[f].is_valid() {
                return Ok(self.map[f]);
            }
            let g = m.funcs.push(waffle::FuncDecl::None);
            self.map[f] = g;
            match &m.funcs[f] {
                waffle::FuncDecl::Import(a, b) => {
                    let i = m
                        .imports
                        .iter()
                        .find(|a| a.kind == ImportKind::Func(f))
                        .context("in getting the import")?
                        .clone();
                    let f = match self.decl.map.get(&(i.module, i.name)) {
                        Some(f) => *f,
                        None => f,
                    };
                    let mut body = FunctionBody::new(&m, *a);
                    let e = body.entry;
                    let params = body.blocks[e]
                        .params
                        .iter()
                        .map(|a| a.1)
                        .collect::<Vec<_>>();
                    body.set_terminator(
                        e,
                        waffle::Terminator::ReturnCall {
                            func: f,
                            args: params,
                        },
                    );
                    m.funcs[g] = waffle::FuncDecl::Body(*a, format!("~{b}"), body);
                }
                waffle::FuncDecl::Lazy(_, _, _) => todo!(),
                waffle::FuncDecl::Body(s, n, b) => {
                    let mut body = b.clone();
                    let n = n.clone();
                    let s = *s;
                    for val in body.values.values_mut() {
                        if let ValueDef::Operator(o, _, _) = val {
                            if let Operator::Call { function_index } = o {
                                *function_index = self.get(*function_index, m)?;
                            }
                            if let Operator::RefFunc { func_index } = o {
                                *func_index = self.get(*func_index, m)?;
                            }
                        }
                    }
                    for k in body.blocks.values_mut() {
                        match &mut k.terminator {
                            Terminator::ReturnCall { func, args } => {
                                *func = self.get(*func, m)?;
                            }
                            _ => {}
                        }
                    }
                    m.funcs[g] = waffle::FuncDecl::Body(s, format!("~{n}"), body);
                }
                waffle::FuncDecl::Compiled(_, _, _) => todo!(),
                waffle::FuncDecl::None => {}
            }
        }
    }
}
