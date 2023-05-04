use proc_macro::TokenStream;
use quote::ToTokens;
use syn;

/// This macro is added before a method of `Simulator` struct in the impl block.
/// Use this macro to first check if current game phase is exactly the phase in
/// the attribute.
///
/// For example, `#[allowed_phase(PlaceBets)]` will make a method first check
/// if current game phase is `PlaceBets`. If not, the method will return an
/// error message.
#[proc_macro_attribute]
pub fn allowed_phase(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast: syn::ImplItemFn = syn::parse(item).unwrap();
    let phase = attr.to_string();
    let function_name = ast.sig.ident.to_string();
    let err_msg = format!("{} is only allowed in {} phase", function_name, phase);
    let code = format!(
        r#"
    if self.current_game_phase != GamePhase::{} {{
        return Err(String::from("{}"));
    }}
"#,
        phase, err_msg
    );
    let early_return: TokenStream = code.parse().unwrap();
    let early_return: syn::Stmt = syn::parse(early_return).unwrap();
    ast.block.stmts.insert(0, early_return);
    ast.into_token_stream().into()
}
