use std::mem::take;

use serde::{Deserialize, Serialize};
use waffle::{
    Export, ExportKind, FuncDecl, FunctionBody, Import, ImportKind, Memory, MemoryData, Module,
    Operator,
};

use waffle_ast::add_op;

// use super::mem_fusing::get_exports;
#[derive(Deserialize, Serialize, Clone)]
pub enum Action {
    CopyIn(ImportMem),
    CopyOut(ImportMem),
}
#[derive(Serialize, Deserialize, Clone)]
pub enum ImportMem {
    Import {
        module: String,
        name: String,
        memory64: bool,
    },
    Export {
        name: String,
        memory64: bool,
    },
}
impl ImportMem {
    pub fn need(&self, m: &mut Module) -> Memory {
        match self {
            ImportMem::Import {
                module,
                name,
                memory64,
            } => {
                for i in m.imports.iter() {
                    if i.module == *module && i.name == *name {
                        if let ImportKind::Memory(m) = &i.kind {
                            return *m;
                        }
                    }
                }
                let me = m.memories.push(MemoryData {
                    initial_pages: 0,
                    maximum_pages: None,
                    segments: vec![],
                    memory64: *memory64,
                });
                m.imports.push(Import {
                    module: module.clone(),
                    name: name.clone(),
                    kind: ImportKind::Memory(me),
                });
                return me;
            }
            ImportMem::Export { name, memory64 } => {
                for e in m.exports.iter() {
                    if e.name == *name {
                        if let ExportKind::Memory(m) = &e.kind {
                            return *m;
                        }
                    }
                }
                let me = m.memories.push(MemoryData {
                    initial_pages: 0,
                    maximum_pages: None,
                    segments: vec![],
                    memory64: *memory64,
                });
                m.exports.push(Export {
                    name: name.clone(),
                    kind: ExportKind::Memory(me),
                });
                return me;
            }
        }
    }
}
pub fn go(m: &mut Module) -> anyhow::Result<()> {
    let mut base_memory = m.exports.iter();
    // let Some(ExportKind::Memory(base_memory)) = get_exports(m).get("memory").cloned() else {
    //     anyhow::bail!("base memory not found")
    // };
    let base_memory = loop {
        let Some(e) = base_memory.next() else {
            anyhow::bail!("base memory not found")
        };
        if e.name != "memory" {
            continue;
        }
        let ExportKind::Memory(b) = &e.kind else {
            continue;
        };
        break *b;
    };
    for i in take(&mut m.imports) {
        if i.module == "memory" {
            let x: Action = serde_bencode::from_str(&i.name)?;
            match (x, i.kind) {
                (Action::CopyIn(a), ImportKind::Func(f)) => {
                    let me = a.need(m);
                    let mut body = FunctionBody::new(&m, m.funcs[f].sig());
                    let p = (body.blocks[body.entry]
                        .params
                        .iter()
                        .map(|a| a.1)
                        .collect::<Vec<_>>());
                    let r = body.rets.clone();
                    let o = add_op(
                        &mut body,
                        &p,
                        &r,
                        Operator::MemoryCopy {
                            dst_mem: base_memory,
                            src_mem: me,
                        },
                    );
                    body.append_to_block(body.entry, o);
                    body.set_terminator(body.entry, waffle::Terminator::Return { values: vec![o] });
                    m.funcs[f] = FuncDecl::Body(m.funcs[f].sig(), i.name, body);
                }
                (Action::CopyOut(a), ImportKind::Func(f)) => {
                    let me = a.need(m);
                    let mut body = FunctionBody::new(&m, m.funcs[f].sig());
                    let p = (body.blocks[body.entry]
                        .params
                        .iter()
                        .map(|a| a.1)
                        .collect::<Vec<_>>());
                    let r = body.rets.clone();
                    let o = add_op(
                        &mut body,
                        &p,
                        &r,
                        Operator::MemoryCopy {
                            dst_mem: me,
                            src_mem: base_memory,
                        },
                    );
                    body.append_to_block(body.entry, o);
                    body.set_terminator(body.entry, waffle::Terminator::Return { values: vec![o] });
                    m.funcs[f] = FuncDecl::Body(m.funcs[f].sig(), i.name, body);
                }
                _ => anyhow::bail!("invalid rewrite"),
            }
        } else {
            m.imports.push(i)
        }
    }
    return Ok(());
}
