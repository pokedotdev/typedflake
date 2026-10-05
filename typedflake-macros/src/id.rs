//! The `#[typedflake]` attribute.

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::meta::ParseNestedMeta;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Expr, Lit, LitInt, LitStr, Path, Token, Type, Visibility};

use crate::util::{self, Repr};

/// Traits the attribute implements itself.
const OWNED_TRAITS: [&str; 10] = [
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "PartialOrd",
    "Ord",
    "Hash",
    "Display",
    "FromStr",
];

pub fn expand(args: TokenStream, input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let (field, repr) = util::id_field(&input, "`#[typedflake]`")?;

    if !matches!(field.vis, Visibility::Inherited) {
        return Err(syn::Error::new_spanned(
            &field.vis,
            "the field of an ID must be private; read it with `.get()` and build IDs with \
             `try_from`",
        ));
    }
    check_attributes(&input.attrs)?;

    let args = Args::parse(args, repr)?;
    Ok(generate(&input, &field.ty, repr, &args))
}

fn check_attributes(attrs: &[Attribute]) -> syn::Result<()> {
    for attr in attrs {
        let path = attr.path();
        if path.is_ident("repr") {
            return Err(syn::Error::new_spanned(
                attr,
                "`#[typedflake]` already applies `#[repr(transparent)]`; remove this attribute",
            ));
        }
        if last_segment(path).as_deref() == Some("typedflake") {
            return Err(syn::Error::new_spanned(
                attr,
                "duplicate `#[typedflake]` attribute",
            ));
        }
        if path.is_ident("derive") {
            let derives = attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated)?;
            derives.iter().try_for_each(check_derive)?;
        }
    }
    Ok(())
}

fn check_derive(path: &Path) -> syn::Result<()> {
    let Some(name) = last_segment(path) else {
        return Ok(());
    };

    if OWNED_TRAITS.contains(&name.as_str()) {
        return Err(syn::Error::new_spanned(
            path,
            format!(
                "`{name}` is already implemented by `#[typedflake]`; remove it from the derive \
                 list"
            ),
        ));
    }

    // These derives build the ID straight from its integer, skipping the
    // validation every other constructor applies.
    let from_sqlx = path.segments.iter().any(|segment| segment.ident == "sqlx");
    let replacement = match name.as_str() {
        "Serialize" | "Deserialize" => "typedflake::Serde",
        "ToSql" | "FromSql" => "typedflake::Postgres",
        "Type" | "Encode" | "Decode" if from_sqlx => "typedflake::SqlxPostgres",
        _ => return Ok(()),
    };
    Err(syn::Error::new_spanned(
        path,
        format!("derive `{replacement}` instead; `{name}` would bypass ID validation"),
    ))
}

fn last_segment(path: &Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
}

struct Args {
    format: FormatSource,
    node: Option<Type>,
    alphabet: Option<Expr>,
}

enum FormatSource {
    Constant(Expr),
    Inline {
        epoch: (u16, u8, u8),
        bits: Option<[u8; 3]>,
    },
}

#[derive(Default)]
struct Bits {
    timestamp: Option<LitInt>,
    node: Option<LitInt>,
    sequence: Option<LitInt>,
}

impl Args {
    fn parse(args: TokenStream, repr: Repr) -> syn::Result<Self> {
        let mut epoch: Option<(Path, LitStr)> = None;
        let mut format: Option<(Path, Expr)> = None;
        let mut bits: Option<(Path, Bits)> = None;
        let mut node: Option<(Path, Type)> = None;
        let mut alphabet: Option<(Path, Expr)> = None;

        let parser = syn::meta::parser(|meta| {
            let key = meta.path.get_ident().map(ToString::to_string);
            match key.as_deref() {
                Some("epoch") => {
                    let value = parse_epoch_literal(&meta)?;
                    set_once(&mut epoch, &meta, value)
                }
                Some("format") => {
                    let value = meta.value()?.parse()?;
                    set_once(&mut format, &meta, value)
                }
                Some("node") => {
                    let value = meta.value()?.parse()?;
                    set_once(&mut node, &meta, value)
                }
                Some("bits") => {
                    let value = parse_bits(&meta)?;
                    set_once(&mut bits, &meta, value)
                }
                Some("alphabet") => {
                    let value = meta.value()?.parse()?;
                    set_once(&mut alphabet, &meta, value)
                }
                _ => Err(meta.error(
                    "unknown option; expected `epoch`, `bits`, `format`, `node`, or `alphabet`",
                )),
            }
        });
        parser.parse2(args)?;

        let format = match (format, epoch, bits) {
            (Some((_, constant)), None, None) => FormatSource::Constant(constant),
            (Some(_), Some((path, _)), _) | (Some(_), None, Some((path, _))) => {
                return Err(syn::Error::new_spanned(
                    path,
                    "`format` already sets the epoch and bits; remove this option or `format`",
                ));
            }
            (None, Some((_, epoch)), bits) => FormatSource::Inline {
                epoch: parse_date(&epoch)?,
                bits: bits
                    .map(|(path, bits)| bits.validate(&path, repr))
                    .transpose()?,
            },
            (None, None, _) => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "`#[typedflake]` requires `epoch = \"YYYY-MM-DD\"` or `format = CONSTANT`",
                ));
            }
        };

        Ok(Self {
            format,
            node: node.map(|(_, node)| node),
            alphabet: alphabet.map(|(_, alphabet)| alphabet),
        })
    }
}

fn set_once<T>(
    slot: &mut Option<(Path, T)>,
    meta: &ParseNestedMeta<'_>,
    value: T,
) -> syn::Result<()> {
    if slot.is_some() {
        let name = meta.path.to_token_stream();
        return Err(meta.error(format!("duplicate `{name}` option")));
    }
    *slot = Some((meta.path.clone(), value));
    Ok(())
}

fn parse_epoch_literal(meta: &ParseNestedMeta<'_>) -> syn::Result<LitStr> {
    match meta.value()?.parse()? {
        Lit::Str(date) => Ok(date),
        other => Err(syn::Error::new_spanned(
            other,
            "`epoch` must be a date string like \"2025-01-01\"; for any other instant use \
             `format = CONSTANT` with `Epoch::new(unix_millis)`",
        )),
    }
}

fn parse_bits(meta: &ParseNestedMeta<'_>) -> syn::Result<Bits> {
    if meta.input.peek(Token![=]) {
        return Err(meta.error("expected `bits(timestamp = N, node = N, sequence = N)`"));
    }

    let mut bits = Bits::default();
    meta.parse_nested_meta(|field| {
        let slot = if field.path.is_ident("timestamp") {
            &mut bits.timestamp
        } else if field.path.is_ident("node") {
            &mut bits.node
        } else if field.path.is_ident("sequence") {
            &mut bits.sequence
        } else {
            return Err(field.error("unknown width; expected `timestamp`, `node`, or `sequence`"));
        };

        if slot.is_some() {
            let name = field.path.to_token_stream();
            return Err(field.error(format!("duplicate `{name}` width")));
        }
        *slot = Some(field.value()?.parse()?);
        Ok(())
    })?;
    Ok(bits)
}

impl Bits {
    /// Checks inline widths here so errors point at the offending value; the
    /// const validation in the runtime covers shared `Format` constants.
    fn validate(self, bits_path: &Path, repr: Repr) -> syn::Result<[u8; 3]> {
        let (Some(timestamp), Some(node), Some(sequence)) =
            (self.timestamp, self.node, self.sequence)
        else {
            return Err(syn::Error::new_spanned(
                bits_path,
                "`bits(...)` requires all of `timestamp`, `node`, and `sequence`",
            ));
        };

        for (literal, name) in [(&timestamp, "timestamp"), (&sequence, "sequence")] {
            if literal.base10_parse::<u8>()? == 0 {
                return Err(syn::Error::new_spanned(
                    literal,
                    format!("{name} width must be greater than zero"),
                ));
            }
        }

        let widths = [
            timestamp.base10_parse::<u8>()?,
            node.base10_parse::<u8>()?,
            sequence.base10_parse::<u8>()?,
        ];
        let total: u32 = widths.iter().copied().map(u32::from).sum();
        if total > repr.usable_bits() {
            return Err(syn::Error::new_spanned(
                bits_path,
                format!(
                    "bit widths add up to {total}, but `{}` has {} usable bits",
                    repr.name(),
                    repr.usable_bits()
                ),
            ));
        }
        Ok(widths)
    }
}

fn parse_date(literal: &LitStr) -> syn::Result<(u16, u8, u8)> {
    let text = literal.value();
    let format_error = || {
        syn::Error::new_spanned(
            literal,
            "`epoch` must be a date in `YYYY-MM-DD` form, like \"2025-01-01\"",
        )
    };

    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return Err(format_error());
    };
    let all_digits = |part: &str, len: usize| {
        part.len() == len && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    if !(all_digits(year, 4) && all_digits(month, 2) && all_digits(day, 2)) {
        return Err(format_error());
    }
    let (year, month, day): (u16, u8, u8) = (
        year.parse().map_err(|_| format_error())?,
        month.parse().map_err(|_| format_error())?,
        day.parse().map_err(|_| format_error())?,
    );

    if year < 1970 {
        return Err(syn::Error::new_spanned(
            literal,
            "`epoch` must not be before 1970-01-01",
        ));
    }
    let is_leap_year =
        (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year => 29,
        2 => 28,
        _ => 0,
    };
    if day == 0 || day > days_in_month {
        return Err(syn::Error::new_spanned(
            literal,
            format!("`{text}` is not a calendar date"),
        ));
    }
    Ok((year, month, day))
}

fn generate(input: &DeriveInput, field_ty: &Type, repr: Repr, args: &Args) -> TokenStream {
    let tf = util::crate_path();
    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let repr_ty = match repr {
        Repr::I64 => quote!(::core::primitive::i64),
        Repr::U64 => quote!(::core::primitive::u64),
    };
    let node_ty = match &args.node {
        Some(node) => node.to_token_stream(),
        None => quote!(::core::primitive::u32),
    };

    let format = match &args.format {
        FormatSource::Constant(constant) => constant.to_token_stream(),
        FormatSource::Inline {
            epoch: (year, month, day),
            bits,
        } => {
            let bits = match bits {
                Some([timestamp, node, sequence]) => quote! {
                    #tf::BitLayout {
                        timestamp: #timestamp,
                        node: #node,
                        sequence: #sequence,
                    }
                },
                None => quote!(#tf::__private::default_bits::<#repr_ty>()),
            };
            quote! {
                #tf::Format {
                    epoch: #tf::Epoch::from_date(#year, #month, #day),
                    bits: #bits,
                }
            }
        }
    };

    let generate_async = cfg!(feature = "tokio").then(|| {
        quote! {
            /// Generates an ID with the default node, waiting for sequence
            /// capacity on a Tokio timer.
            ///
            /// To bound the wait, wrap the call in `tokio::time::timeout`.
            pub async fn generate_async()
            -> ::core::result::Result<Self, #tf::GenerateError> {
                <Self as #tf::Id>::generate_async().await
            }
        }
    });

    let encoding = args.alphabet.as_ref().map(|alphabet| {
        quote! {
            impl #tf::EncodedId for #name {
                const ALPHABET: #tf::Alphabet = #alphabet;
            }

            // Reports an invalid alphabet at the declaration, instead of at
            // the first `encode` or `decode`.
            const _: #tf::Alphabet = <#name as #tf::EncodedId>::ALPHABET;

            impl #name {
                /// Writes the ID in its alphabet, without allocating.
                ///
                /// Every ID of the type has the same length.
                pub fn encode(self) -> #tf::Encoded {
                    <Self as #tf::EncodedId>::encode(self)
                }

                /// Reads an ID written by `encode`. Only that exact form is
                /// accepted.
                pub fn decode(
                    text: &str,
                ) -> ::core::result::Result<Self, #tf::DecodeIdError> {
                    <Self as #tf::EncodedId>::decode(text)
                }
            }
        }
    });

    quote! {
        #(#attrs)*
        #[derive(
            ::core::clone::Clone,
            ::core::marker::Copy,
            ::core::cmp::PartialEq,
            ::core::cmp::Eq,
            ::core::cmp::PartialOrd,
            ::core::cmp::Ord,
            ::core::hash::Hash,
        )]
        #[repr(transparent)]
        // Integration derives inspect the field, so it stays as written.
        #vis struct #name(#field_ty);

        // Validates the format against the integer and node type at the
        // declaration, instead of at the first use. Panicking here, not in a
        // called function, keeps the compile error pointed at this item.
        const _: () = {
            if let ::core::option::Option::Some(message) =
                #tf::__private::format_error_message::<#repr_ty, #node_ty>(
                    &<#name as #tf::Id>::FORMAT,
                )
            {
                ::core::panic!("{}", message);
            }
        };

        impl #tf::Id for #name {
            type Repr = #repr_ty;
            type Node = #node_ty;

            const FORMAT: #tf::Format = #format;

            #[inline]
            fn get(self) -> #repr_ty {
                self.0
            }

            #[inline]
            fn __from_repr(repr: #repr_ty) -> Self {
                Self(repr)
            }

            fn __default_generator()
            -> &'static #tf::__private::OnceLock<#tf::Generator<Self>> {
                static DEFAULT: #tf::__private::OnceLock<#tf::Generator<#name>> =
                    #tf::__private::OnceLock::new();
                &DEFAULT
            }
        }

        impl #name {
            /// Generates an ID with the node installed by `typedflake::init`.
            ///
            /// Returns immediately; if this millisecond's sequence is used up
            /// it returns an error instead of waiting.
            pub fn generate() -> ::core::result::Result<Self, #tf::GenerateError> {
                <Self as #tf::Id>::generate()
            }

            /// Generates an ID with the default node, blocking the thread
            /// while it waits for sequence capacity.
            pub fn generate_blocking() -> ::core::result::Result<Self, #tf::GenerateError> {
                <Self as #tf::Id>::generate_blocking()
            }

            #generate_async

            /// Returns a generator for an explicit node, without global
            /// initialization.
            ///
            /// Generators for the same node share their state.
            pub fn generator(
                node: #node_ty,
            ) -> ::core::result::Result<#tf::Generator<Self>, #tf::GeneratorError> {
                <Self as #tf::Id>::generator(node)
            }

            /// Returns the stored integer.
            #[inline]
            pub const fn get(self) -> #repr_ty {
                self.0
            }

            /// Splits the ID into its timestamp, node, and sequence.
            pub fn parts(self) -> #tf::Parts<#node_ty> {
                <Self as #tf::Id>::parts(self)
            }

            /// Builds an ID from its parts, checking each against the format.
            ///
            /// This does not reserve a sequence number or guarantee uniqueness.
            pub fn from_parts(
                parts: #tf::Parts<#node_ty>,
            ) -> ::core::result::Result<Self, #tf::InvalidId> {
                <Self as #tf::Id>::from_parts(parts)
            }

            /// Returns when the ID was created, in milliseconds since the Unix
            /// epoch.
            pub fn unix_millis(self) -> ::core::result::Result<u64, #tf::TimestampError> {
                <Self as #tf::Id>::unix_millis(self)
            }
        }

        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_tuple(::core::stringify!(#name)).field(&self.0).finish()
            }
        }

        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, f)
            }
        }

        impl ::core::str::FromStr for #name {
            type Err = #tf::ParseIdError;

            fn from_str(text: &str) -> ::core::result::Result<Self, Self::Err> {
                #tf::__private::parse(text)
            }
        }

        impl ::core::convert::TryFrom<::core::primitive::i64> for #name {
            type Error = #tf::InvalidId;

            fn try_from(raw: ::core::primitive::i64) -> ::core::result::Result<Self, Self::Error> {
                #tf::__private::from_i64(raw)
            }
        }

        impl ::core::convert::TryFrom<::core::primitive::u64> for #name {
            type Error = #tf::InvalidId;

            fn try_from(raw: ::core::primitive::u64) -> ::core::result::Result<Self, Self::Error> {
                #tf::__private::from_u64(raw)
            }
        }

        impl ::core::convert::From<#name> for #repr_ty {
            #[inline]
            fn from(id: #name) -> Self {
                id.0
            }
        }

        #encoding
    }
}
