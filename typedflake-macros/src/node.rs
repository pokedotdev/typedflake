//! The `TypedNode` derive.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, LitInt};

use crate::util;

/// An ID needs at least one timestamp and one sequence bit.
const MAX_NODE_BITS: u32 = 62;

pub fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let shape_error = || {
        syn::Error::new_spanned(
            &input.ident,
            "`TypedNode` supports only structs with named `u8`, `u16`, or `u32` fields",
        )
    };

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "`TypedNode` does not support generic node types",
        ));
    }
    let Data::Struct(data) = &input.data else {
        return Err(shape_error());
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(shape_error());
    };
    if fields.named.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "`TypedNode` requires at least one field",
        ));
    }

    let widths = fields
        .named
        .iter()
        .map(field_bits)
        .collect::<syn::Result<Vec<u8>>>()?;
    let total: u32 = widths.iter().copied().map(u32::from).sum();
    if total > MAX_NODE_BITS {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!(
                "node fields add up to {total} bits, but an ID has room for at most \
                 {MAX_NODE_BITS} node bits"
            ),
        ));
    }

    let tf = util::crate_path();
    let name = &input.ident;
    let total = total as u8;

    let pack_steps = fields.named.iter().zip(&widths).map(|(field, bits)| {
        let ident = &field.ident;
        let label = ident.as_ref().map(ToString::to_string);
        quote! {
            let packed = #tf::__private::pack_field(
                packed,
                #label,
                ::core::convert::From::from(self.#ident),
                #bits,
            )?;
        }
    });

    // Fields are most significant first, so each sits above all later ones.
    let mut shift = total;
    let unpack_fields = fields.named.iter().zip(&widths).map(|(field, bits)| {
        shift -= bits;
        let ident = &field.ident;
        let ty = &field.ty;
        let mask = (1u64 << bits) - 1;
        quote!(#ident: ((raw >> #shift) & #mask) as #ty)
    });

    Ok(quote! {
        impl #tf::Node for #name {
            const BITS: ::core::option::Option<::core::primitive::u8> =
                ::core::option::Option::Some(#total);

            fn pack(
                self,
                _bits: ::core::primitive::u8,
            ) -> ::core::result::Result<::core::primitive::u64, #tf::NodeError> {
                let packed: ::core::primitive::u64 = 0;
                #(#pack_steps)*
                ::core::result::Result::Ok(packed)
            }

            fn unpack(raw: ::core::primitive::u64, _bits: ::core::primitive::u8) -> Self {
                Self { #(#unpack_fields),* }
            }
        }
    })
}

fn field_bits(field: &Field) -> syn::Result<u8> {
    let type_width: u8 = match util::primitive_name(&field.ty).as_deref() {
        Some("u8") => 8,
        Some("u16") => 16,
        Some("u32") => 32,
        _ => {
            return Err(syn::Error::new_spanned(
                &field.ty,
                "node fields must be `u8`, `u16`, or `u32`",
            ));
        }
    };

    let mut bits: Option<LitInt> = None;
    for attr in field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("node"))
    {
        attr.parse_nested_meta(|meta| {
            if !meta.path.is_ident("bits") {
                return Err(meta.error("unknown option; expected `bits`"));
            }
            if bits.is_some() {
                return Err(meta.error("duplicate `bits` option"));
            }
            bits = Some(meta.value()?.parse()?);
            Ok(())
        })?;
    }

    let Some(bits) = bits else {
        return Err(syn::Error::new_spanned(
            field,
            "missing `#[node(bits = N)]` on this field",
        ));
    };
    let width = bits.base10_parse::<u8>()?;
    if width == 0 || width > type_width {
        let ty = util::primitive_name(&field.ty).unwrap_or_default();
        return Err(syn::Error::new_spanned(
            &bits,
            format!("`bits` must be between 1 and {type_width} for a `{ty}` field"),
        ));
    }
    Ok(width)
}
