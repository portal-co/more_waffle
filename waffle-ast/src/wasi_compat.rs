use waffle::{BlockTarget, ExportKind, FunctionBody, MemoryArg, Module, Operator, Type, Value};

pub fn poll_oneoff(f: &mut FunctionBody, m: &mut Module) {
    let mem = m
        .exports
        .iter()
        .find_map(|x| {
            if x.name == "memory" {
                match &x.kind {
                    ExportKind::Memory(m) => Some(*m),
                    _ => None,
                }
            } else {
                None
            }
        })
        .unwrap();
    let [sptr, eptr, subs, ret0] =
        <&[(Type, Value)] as TryInto<[(Type, Value); 4]>>::try_into(&f.blocks[f.entry].params)
            .unwrap()
            .map(|a| a.1);
    let me_i32 = MemoryArg {
        offset: 0,
        align: 2,
        memory: mem,
    };
    f.add_op(
        f.entry,
        Operator::I32Store {
            memory: me_i32.clone(),
        },
        &[ret0, subs],
        &[],
    );
    let zero = f.add_op(f.entry, Operator::I32Const { value: 0 }, &[], &[Type::I32]);
    let one = f.add_op(f.entry, Operator::I32Eqz, &[zero], &[Type::I32]);
    let x = f.add_block();
    f.set_terminator(
        f.entry,
        waffle::Terminator::Br {
            target: BlockTarget {
                block: x,
                args: vec![subs],
            },
        },
    );
    let subs = f.add_blockparam(x, f.values[subs].ty(&f.type_pool).unwrap());
    let y = f.add_block();
    let z = f.add_block();
    f.set_terminator(
        x,
        waffle::Terminator::Select {
            value: subs,
            targets: vec![BlockTarget {
                block: z,
                args: vec![],
            }],
            default: BlockTarget {
                block: y,
                args: vec![],
            },
        },
    );
    f.set_terminator(z, waffle::Terminator::Return { values: vec![zero] });
    let sp2 = f.add_op(y, Operator::I32Add, &[sptr, subs], &[Type::I32]);
    let ep2 = f.add_op(y, Operator::I32Add, &[eptr, subs], &[Type::I32]);
    let sp2 = f.add_op(
        y,
        Operator::I32Load {
            memory: me_i32.clone(),
        },
        &[sp2],
        &[Type::I32],
    );
    let ep2 = f.add_op(
        y,
        Operator::I32Load {
            memory: me_i32.clone(),
        },
        &[ep2],
        &[Type::I32],
    );
    let eu = f.add_op(
        y,
        Operator::I32Load {
            memory: me_i32.clone(),
        },
        &[sp2],
        &[Type::I32],
    );
    f.add_op(
        y,
        Operator::I32Store {
            memory: me_i32.clone(),
        },
        &[ep2, eu],
        &[],
    );
    let subs = f.add_op(y, Operator::I32Sub, &[subs, one], &[Type::I32]);
    f.set_terminator(
        y,
        waffle::Terminator::Br {
            target: BlockTarget {
                block: x,
                args: vec![subs],
            },
        },
    );
}
