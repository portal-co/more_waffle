use syn::{parse::Parse, parse_macro_input, LitStr, Token};
use wasm_multi_memory_polyfill::{pass::ImportMem, rust::Opts};
struct X{
    pub wrapped: ImportMem
}
impl Parse for X{
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let a: LitStr = input.parse()?;
        let o: Option<Token![memory64]> = input.parse()?;
        return Ok(X{wrapped:match a.value().split_once("."){
            Some((a,b)) => ImportMem::Import { module: a.to_owned(), name: b.to_owned(), memory64: o.is_some() },
            None => ImportMem::Export { name: a.value(), memory64: o.is_some() },
        }});
    }
}
#[proc_macro]
pub fn memory(a: proc_macro::TokenStream) -> proc_macro::TokenStream{
    let x = parse_macro_input!(a as X);
    return wasm_multi_memory_polyfill::rust::new_mem(x.wrapped, None, &Opts{}).into();
}