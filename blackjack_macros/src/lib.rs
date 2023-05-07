use std::collections::HashSet;

use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{self, DataStruct};

/// This macro is added before a method of `Simulator` struct in the impl block.
/// Use this macro to first check if current game phase is exactly the phase in
/// the attribute.
///
/// For example, `#[allowed_phase(PlaceBets)]` will make a method first check
/// if current game phase is `PlaceBets`. If not, the method will return an
/// error message.
#[proc_macro_attribute]
pub fn allowed_phase(attr: TokenStream1, item: TokenStream1) -> TokenStream1 {
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
    let early_return: TokenStream1 = code.parse().unwrap();
    let early_return: syn::Stmt = syn::parse(early_return).unwrap();
    ast.block.stmts.insert(0, early_return);
    ast.into_token_stream().into()
}

#[proc_macro_derive(ExpectationAfterSplit)]
pub fn expectation_after_split_derive(input: TokenStream1) -> TokenStream1 {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();
    let struct_name = &ast.ident;
    let mut field_name_set: HashSet<String> = HashSet::new();
    let data_struct = match ast.data {
        syn::Data::Struct(x) => x,
        _ => panic!("No data struct in ast.data!"),
    };
    let named_fields = match data_struct.fields {
        syn::Fields::Named(x) => x,
        _ => panic!("No named fields"),
    };
    let fields = named_fields.named;
    for field in &fields {
        field_name_set.insert(field.ident.as_ref().unwrap().to_string());
    }

    let allow_das = field_name_set.contains("double");
    let allow_late_surrender = field_name_set.contains("surrender");

    let line_stand = generate_getter_setter("stand", field_name_set.contains("stand"));
    let line_hit = generate_getter_setter("hit", field_name_set.contains("hit"));
    let line_double = generate_getter_setter("double", field_name_set.contains("double"));
    let line_surrender = generate_getter_setter("surrender", field_name_set.contains("surrender"));

    let default_struct = quote! {
        Self {
            stand: -f64::INFINITY,
            hit: -f64::INFINITY,
            double: -f64::INFINITY,
            surrender: -f64::INFINITY,
        }
    };
    let mut original_default_struct: syn::ExprStruct = syn::parse(default_struct.into()).unwrap();
    let mut cloned_default_struct = original_default_struct.clone();
    cloned_default_struct.fields.clear();
    for field in &original_default_struct.fields {
        let member = match &field.member {
            syn::Member::Named(x) => x,
            _ => panic!("No named field"),
        };
        if field_name_set.contains(&member.to_string()) {
            cloned_default_struct.fields.push(field.clone());
        }
    }

    let ts2 = quote! {
        impl ExpectationAfterSplit for #struct_name {
            const ALLOW_DAS: bool = #allow_das;
            const ALLOW_LATE_SURRENDER: bool = #allow_late_surrender;

            #line_stand
            #line_hit
            #line_double
            #line_surrender
        }

        impl Default for #struct_name {
            fn default() -> Self {
                #cloned_default_struct
            }
        }
    };
    ts2.into()
}

fn generate_getter_setter(expectation_name: &str, is_valid: bool) -> TokenStream2 {
    let getter_name = format_ident!("{}", expectation_name);
    let setter_name = format_ident!("set_{}", expectation_name);
    if is_valid {
        quote! {
            fn #getter_name(&self) -> f64 {
                self.#getter_name
            }
            fn #setter_name(&mut self, val: f64) {
                self.#getter_name = val;
            }
        }
    } else {
        let err_msg = format!("Cannot set expectation of {}", expectation_name);
        quote! {
            fn #getter_name(&self) -> f64 {
                -f64::INFINITY
            }
            fn #setter_name(&mut self, val: f64) {
                panic!("{}", #err_msg);
            }
        }
    }
}
