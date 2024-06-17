use crate::pass::{Action, ImportMem};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;
#[derive(Clone, Debug)]
pub struct Opts {

}
pub fn new_mem(x: ImportMem, name: Option<String>, opts: &Opts) -> TokenStream {
    let xi = name.unwrap_or_else(|| serde_bencode::to_string(&x).unwrap());
    let id = Ident::new(&format!("mem_{}", bindname(&xi)), Span::call_site());
    // let x = ImportMem::Import { module: modu, name };
    let a = serde_bencode::to_string(&Action::CopyIn(x.clone())).unwrap();
    let b = serde_bencode::to_string(&Action::CopyOut(x.clone())).unwrap();
    let t = match x{
        ImportMem::Import { module, name, memory64 } => memory64,
        ImportMem::Export { name, memory64 } => memory64,
    };
    let t = if t{
        quote! {u64}
    }else{
        quote! {u32}
    };
    return quote! {
        mod #id{
            mod _base{
                #[link(wasm_import_module = "memory")]
                extern "C"{
                    #[link(wasm_import_name = #a)]
                    fn r#in(a: *mut u8, b: #t, c: u32);
                    #[link(wasm_import_name = #b)]
                    fn out(a: #t, b: *const u8, c: u32);
                }
            }
            pub fn r#in(a: &mut [u8], b: #t){
                unsafe{
                    _base::r#in(a.as_mut_ptr(),b,a.len() as u32)
                };
            }
            pub fn out(a: &[u8], b: #t){
                unsafe{
                    _base::out(b,a.as_ptr(),a.len() as u32)
                };
            }
        }
    };
}
pub fn bindname(a: &str) -> String {
    let mut v = vec![];
    for k in a.chars() {
        if k.is_alphanumeric() {
            v.push(k)
        } else {
            v.extend(format!("_{}_", k as u32).chars());
        }
    }
    return v.into_iter().collect();
}
