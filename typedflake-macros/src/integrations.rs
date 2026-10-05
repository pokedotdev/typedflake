//! Opt-in integration derives.
//!
//! Each derive only forwards to shared implementations in the runtime, which
//! reach the ID through its `Id` implementation.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Ident};

use crate::util::{self, Repr};

fn require_feature(enabled: bool, derive: &str, feature: &str) -> syn::Result<()> {
    if enabled {
        return Ok(());
    }
    Err(syn::Error::new(
        Span::call_site(),
        format!("`typedflake::{derive}` requires the `{feature}` feature of `typedflake`"),
    ))
}

/// PostgreSQL has no unsigned 64-bit integer, and a full-width `u64` is never
/// stored as a negative `BIGINT`.
fn require_signed(input: &DeriveInput, derive: &str) -> syn::Result<()> {
    let (field, repr) = util::id_field(input, &format!("`typedflake::{derive}`"))?;
    if repr == Repr::U64 {
        return Err(syn::Error::new_spanned(
            &field.ty,
            format!(
                "`typedflake::{derive}` supports only `i64` IDs; PostgreSQL `BIGINT` cannot \
                 hold every `u64`"
            ),
        ));
    }
    Ok(())
}

pub fn serde(input: &DeriveInput) -> syn::Result<TokenStream> {
    serde_impls(input, "Serde", "serialize", "deserialize")
}

pub fn serde_encoded(input: &DeriveInput) -> syn::Result<TokenStream> {
    serde_impls(
        input,
        "SerdeEncoded",
        "serialize_encoded",
        "deserialize_encoded",
    )
}

fn serde_impls(
    input: &DeriveInput,
    derive: &str,
    serialize: &str,
    deserialize: &str,
) -> syn::Result<TokenStream> {
    require_feature(cfg!(feature = "serde"), derive, "serde")?;
    util::id_field(input, &format!("`typedflake::{derive}`"))?;

    let tf = util::crate_path();
    let serde = quote!(#tf::__private::serde);
    let name = &input.ident;
    let serialize = Ident::new(serialize, Span::call_site());
    let deserialize = Ident::new(deserialize, Span::call_site());

    Ok(quote! {
        impl #serde::Serialize for #name {
            fn serialize<S: #serde::Serializer>(
                &self,
                serializer: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                #serde::#serialize(self, serializer)
            }
        }

        impl<'de> #serde::Deserialize<'de> for #name {
            fn deserialize<D: #serde::Deserializer<'de>>(
                deserializer: D,
            ) -> ::core::result::Result<Self, D::Error> {
                #serde::#deserialize(deserializer)
            }
        }
    })
}

pub fn sqlx_postgres(input: &DeriveInput) -> syn::Result<TokenStream> {
    require_feature(
        cfg!(feature = "sqlx-postgres"),
        "SqlxPostgres",
        "sqlx-postgres",
    )?;
    require_signed(input, "SqlxPostgres")?;

    let tf = util::crate_path();
    let sqlx = quote!(#tf::__private::sqlx_postgres);
    let name = &input.ident;

    Ok(quote! {
        impl #sqlx::Type<#sqlx::Postgres> for #name {
            fn type_info() -> #sqlx::PgTypeInfo {
                #sqlx::type_info()
            }

            fn compatible(ty: &#sqlx::PgTypeInfo) -> ::core::primitive::bool {
                #sqlx::compatible(ty)
            }
        }

        impl #sqlx::PgHasArrayType for #name {
            fn array_type_info() -> #sqlx::PgTypeInfo {
                #sqlx::array_type_info()
            }
        }

        impl<'q> #sqlx::Encode<'q, #sqlx::Postgres> for #name {
            fn encode_by_ref(
                &self,
                buffer: &mut #sqlx::PgArgumentBuffer,
            ) -> ::core::result::Result<#sqlx::IsNull, #sqlx::BoxDynError> {
                #sqlx::encode(self, buffer)
            }
        }

        impl<'r> #sqlx::Decode<'r, #sqlx::Postgres> for #name {
            fn decode(
                value: #sqlx::PgValueRef<'r>,
            ) -> ::core::result::Result<Self, #sqlx::BoxDynError> {
                #sqlx::decode(value)
            }
        }
    })
}

pub fn postgres(input: &DeriveInput) -> syn::Result<TokenStream> {
    require_feature(cfg!(feature = "postgres"), "Postgres", "postgres")?;
    require_signed(input, "Postgres")?;

    let tf = util::crate_path();
    let postgres = quote!(#tf::__private::postgres);
    let name = &input.ident;

    Ok(quote! {
        impl #postgres::ToSql for #name {
            fn to_sql(
                &self,
                ty: &#postgres::Type,
                out: &mut #postgres::BytesMut,
            ) -> ::core::result::Result<#postgres::IsNull, #postgres::BoxError> {
                #postgres::to_sql(self, ty, out)
            }

            fn accepts(ty: &#postgres::Type) -> ::core::primitive::bool {
                #postgres::accepts(ty)
            }

            #postgres::to_sql_checked!();
        }

        impl<'a> #postgres::FromSql<'a> for #name {
            fn from_sql(
                ty: &#postgres::Type,
                raw: &'a [::core::primitive::u8],
            ) -> ::core::result::Result<Self, #postgres::BoxError> {
                #postgres::from_sql(ty, raw)
            }

            fn accepts(ty: &#postgres::Type) -> ::core::primitive::bool {
                #postgres::accepts(ty)
            }
        }
    })
}
