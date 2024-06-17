use std::iter::once;

use waffle::{Module, Operator, Type, ValueDef};
pub fn patch_ty(r: &Type, t: &mut Type) {
    if let Type::ExternRef = t.clone() {
        *t = *r
    }
}
pub fn instantiate(r: &Type, m: &mut Module) {
    for s in m.signatures.values_mut() {
        for p in s.params.iter_mut().chain(s.returns.iter_mut()) {
            patch_ty(r, p)
        }
    }
    for f in m.funcs.values_mut() {
        if let Some(b) = f.body_mut() {
            let x = b.type_pool.from_iter(once(Type::I32));
            for p in b
                .blocks
                .values_mut()
                .flat_map(|a| a.params.iter_mut().map(|a| &mut a.0))
                .chain(b.rets.iter_mut())
            {
                patch_ty(r, p)
            }
            for v in b.values.iter().collect::<Vec<_>>(){
                if let ValueDef::Operator(o, _1, tys) = &mut b.values[v]{
                    match o.clone(){
                        Operator::RefNull { ty } => {
                            if ty == Type::ExternRef{
                                *o = match r{
                                    Type::I32 => Operator::I32Const { value: 0 },
                                    Type::I64 => Operator::I64Const { value: 0 },
                                    Type::F32 => todo!(),
                                    Type::F64 => todo!(),
                                    Type::V128 => todo!(),
                                    Type::FuncRef => todo!(),
                                    Type::ExternRef => todo!(),
                                    Type::TypedFuncRef { nullable, sig_index } => todo!(),
                                }
                            }
                        },
                        Operator::RefIsNull => {
                            if b.type_pool[*tys][0] == Type::ExternRef{
                                *o = match r{
                                    Type::I32 => Operator::I32Eqz,
                                    Type::I64 => Operator::I64Eqz,
                                    Type::F32 => todo!(),
                                    Type::F64 => todo!(),
                                    Type::V128 => todo!(),
                                    Type::FuncRef => todo!(),
                                    Type::ExternRef => todo!(),
                                    Type::TypedFuncRef { nullable, sig_index } => todo!(),
                                }
                            }
                        },
                        _ => {

                        }
                    }
                }
            }
            for t in b.type_pool.storage.iter_mut() {
                patch_ty(r, t)
            }
        }
    }
}
