use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, Ident, Type};

/// Path to the `typedflake` crate as the caller named it in its manifest.
pub fn crate_path() -> TokenStream {
    match crate_name("typedflake") {
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        // `typedflake` declares `extern crate self as typedflake`, and a crate
        // reaching it through a re-export has no better name to offer.
        Ok(FoundCrate::Itself) | Err(_) => quote!(::typedflake),
    }
}

/// Integer an ID is stored in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Repr {
    I64,
    U64,
}

impl Repr {
    pub fn usable_bits(self) -> u32 {
        match self {
            Self::I64 => 63,
            Self::U64 => 64,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::I64 => "i64",
            Self::U64 => "u64",
        }
    }
}

pub fn primitive_name(ty: &Type) -> Option<String> {
    let Type::Path(path) = ty else { return None };
    if path.qself.is_some() {
        return None;
    }
    path.path.get_ident().map(Ident::to_string)
}

/// Returns the single field of an ID newtype and its integer type.
pub fn id_field<'a>(input: &'a DeriveInput, macro_name: &str) -> syn::Result<(&'a Field, Repr)> {
    let shape_error = |span: &dyn quote::ToTokens| {
        syn::Error::new_spanned(
            span,
            format!(
                "{macro_name} supports only tuple structs with one `i64` or `u64` field, \
                 like `struct UserId(i64);`"
            ),
        )
    };

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            format!("{macro_name} does not support generic ID types"),
        ));
    }

    let Data::Struct(data) = &input.data else {
        return Err(shape_error(&input.ident));
    };
    let Fields::Unnamed(fields) = &data.fields else {
        return Err(shape_error(&input.ident));
    };
    let mut fields_iter = fields.unnamed.iter();
    let (Some(field), None) = (fields_iter.next(), fields_iter.next()) else {
        return Err(shape_error(&fields));
    };

    let repr = match primitive_name(&field.ty).as_deref() {
        Some("i64") => Repr::I64,
        Some("u64") => Repr::U64,
        _ => return Err(shape_error(&field.ty)),
    };
    Ok((field, repr))
}
