use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, PathArguments, Type, parse_macro_input};

/// Derive macro for generating TypedFlake ID types.
///
/// Transforms a tuple struct wrapping `u64` into a fully-featured Snowflake-style ID type
/// with its own independent generator state, standard trait implementations, and typed
/// generator wrapper.
///
/// # Basic Usage
///
/// ```ignore
/// use typedflake::TypedFlake;
///
/// #[derive(TypedFlake)]
/// pub struct UserId(u64);
///
/// let id = UserId::generate();
/// ```
///
/// # Inline Configuration
///
/// ```ignore
/// use typedflake::TypedFlake;
///
/// #[derive(TypedFlake)]
/// #[typedflake(epoch = "2025-01-01")]
/// pub struct UserId(u64);
///
/// #[derive(TypedFlake)]
/// #[typedflake(layout = (42, 8, 4, 10), epoch = 1600000000000)]
/// pub struct OrderId(u64);
/// ```
///
/// # Custom Configuration via Const
///
/// ```ignore
/// use typedflake::{TypedFlake, Config, BitLayout, Epoch};
///
/// const CUSTOM: Config = Config::new(
///     BitLayout::new(42, 8, 4, 10),
///     Epoch::from_date(2025, 1, 1),
/// );
///
/// #[derive(TypedFlake)]
/// #[typedflake(config = CUSTOM)]
/// pub struct SessionId(u64);
/// ```
#[proc_macro_derive(TypedFlake, attributes(typedflake))]
pub fn derive_typed_flake(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_typed_flake(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

enum EpochValue {
    Millis(u64),
    Date { year: u16, month: u8, day: u8 },
}

enum TypedFlakeAttr {
    Config(syn::Expr),
    Inline {
        layout: Option<(u8, u8, u8, u8)>,
        epoch: Option<EpochValue>,
    },
}

fn parse_typedflake_attr(input: syn::parse::ParseStream) -> syn::Result<TypedFlakeAttr> {
    let mut config_expr: Option<syn::Expr> = None;
    let mut layout: Option<(u8, u8, u8, u8)> = None;
    let mut epoch: Option<EpochValue> = None;

    while !input.is_empty() {
        let ident: syn::Ident = input.parse()?;

        match ident.to_string().as_str() {
            "config" => {
                if layout.is_some() || epoch.is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "`config` cannot be combined with `layout` or `epoch`",
                    ));
                }
                if config_expr.is_some() {
                    return Err(syn::Error::new(ident.span(), "duplicate `config`"));
                }
                let _eq: syn::Token![=] = input.parse()?;
                config_expr = Some(input.parse()?);
            }
            "layout" => {
                if config_expr.is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "`layout` cannot be combined with `config`",
                    ));
                }
                if layout.is_some() {
                    return Err(syn::Error::new(ident.span(), "duplicate `layout`"));
                }
                let _eq: syn::Token![=] = input.parse()?;
                let content;
                syn::parenthesized!(content in input);
                let t: syn::LitInt = content.parse()?;
                let _: syn::Token![,] = content.parse()?;
                let w: syn::LitInt = content.parse()?;
                let _: syn::Token![,] = content.parse()?;
                let p: syn::LitInt = content.parse()?;
                let _: syn::Token![,] = content.parse()?;
                let s: syn::LitInt = content.parse()?;
                // Allow optional trailing comma
                let _ = content.parse::<syn::Token![,]>();

                let t_val: u8 = t.base10_parse()?;
                let w_val: u8 = w.base10_parse()?;
                let p_val: u8 = p.base10_parse()?;
                let s_val: u8 = s.base10_parse()?;

                let sum = t_val as u16 + w_val as u16 + p_val as u16 + s_val as u16;
                if sum != 64 {
                    return Err(syn::Error::new(
                        t.span(),
                        format!("bit allocation must sum to 64, got {sum}"),
                    ));
                }
                if t_val == 0 {
                    return Err(syn::Error::new(t.span(), "timestamp bits must be > 0"));
                }
                if s_val == 0 {
                    return Err(syn::Error::new(s.span(), "sequence bits must be > 0"));
                }

                layout = Some((t_val, w_val, p_val, s_val));
            }
            "epoch" => {
                if config_expr.is_some() {
                    return Err(syn::Error::new(
                        ident.span(),
                        "`epoch` cannot be combined with `config`",
                    ));
                }
                if epoch.is_some() {
                    return Err(syn::Error::new(ident.span(), "duplicate `epoch`"));
                }
                let _eq: syn::Token![=] = input.parse()?;

                let lookahead = input.lookahead1();
                if lookahead.peek(syn::LitStr) {
                    let lit: syn::LitStr = input.parse()?;
                    let val = lit.value();
                    let parts: Vec<&str> = val.split('-').collect();
                    if parts.len() != 3 {
                        return Err(syn::Error::new(
                            lit.span(),
                            "expected date format \"YYYY-MM-DD\"",
                        ));
                    }
                    let year: u16 = parts[0]
                        .parse()
                        .map_err(|_| syn::Error::new(lit.span(), "invalid year in date"))?;
                    let month: u8 = parts[1]
                        .parse()
                        .map_err(|_| syn::Error::new(lit.span(), "invalid month in date"))?;
                    let day: u8 = parts[2]
                        .parse()
                        .map_err(|_| syn::Error::new(lit.span(), "invalid day in date"))?;
                    epoch = Some(EpochValue::Date { year, month, day });
                } else if lookahead.peek(syn::LitInt) {
                    let lit: syn::LitInt = input.parse()?;
                    let millis: u64 = lit.base10_parse()?;
                    epoch = Some(EpochValue::Millis(millis));
                } else {
                    return Err(lookahead.error());
                }
            }
            other => {
                return Err(syn::Error::new(
                    ident.span(),
                    format!("unknown attribute `{other}`, expected `config`, `layout`, or `epoch`"),
                ));
            }
        }

        // consume optional trailing comma
        let _ = input.parse::<syn::Token![,]>();
    }

    if let Some(expr) = config_expr {
        Ok(TypedFlakeAttr::Config(expr))
    } else {
        Ok(TypedFlakeAttr::Inline { layout, epoch })
    }
}

fn impl_typed_flake(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;

    validate_struct(input)?;

    let config_expr = parse_config_attr(input)?;
    let generator_name = format_ident!("{}Generator", name);

    let config_init = match config_expr {
        Some(expr) => quote! { #expr },
        None => quote! { ::typedflake::global::get_default_config() },
    };

    let standard_traits = generate_standard_traits(name);
    let generator_struct = generate_generator_struct(name, vis, &generator_name);
    let inherent_methods = generate_inherent_methods(name, &generator_name, &config_init);
    let conversion_impls = generate_conversion_impls(name);
    let serde_impls = generate_serde_impls(name);

    Ok(quote! {
        #standard_traits
        #generator_struct
        #inherent_methods
        #conversion_impls
        #serde_impls
    })
}

fn validate_struct(input: &DeriveInput) -> syn::Result<()> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(
                        fields,
                        "TypedFlake requires exactly one field: `struct Name(u64)`",
                    ));
                }
                let field = &fields.unnamed[0];
                if !is_u64_type(&field.ty) {
                    return Err(syn::Error::new_spanned(
                        &field.ty,
                        "TypedFlake field must be `u64`",
                    ));
                }
                Ok(())
            }
            _ => Err(syn::Error::new_spanned(
                input,
                "TypedFlake can only be derived on tuple structs: `struct Name(u64)`",
            )),
        },
        _ => Err(syn::Error::new_spanned(
            input,
            "TypedFlake can only be derived on structs",
        )),
    }
}

fn is_u64_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if type_path.qself.is_none() && type_path.path.segments.len() == 1 {
            let segment = &type_path.path.segments[0];
            return segment.ident == "u64" && matches!(segment.arguments, PathArguments::None);
        }
    }
    false
}

fn epoch_to_tokens(epoch: &EpochValue) -> proc_macro2::TokenStream {
    match epoch {
        EpochValue::Millis(ms) => quote! { ::typedflake::Epoch::new(#ms) },
        EpochValue::Date { year, month, day } => {
            quote! { ::typedflake::Epoch::from_date(#year, #month, #day) }
        }
    }
}

fn parse_config_attr(input: &DeriveInput) -> syn::Result<Option<syn::Expr>> {
    for attr in &input.attrs {
        if attr.path().is_ident("typedflake") {
            match &attr.meta {
                syn::Meta::List(_) => {
                    let parsed: TypedFlakeAttr = attr.parse_args_with(parse_typedflake_attr)?;
                    match parsed {
                        TypedFlakeAttr::Config(expr) => return Ok(Some(expr)),
                        TypedFlakeAttr::Inline {
                            layout: None,
                            epoch: None,
                        } => {
                            return Err(syn::Error::new_spanned(
                                attr,
                                "empty #[typedflake(...)] attribute",
                            ));
                        }
                        TypedFlakeAttr::Inline { layout, epoch } => {
                            let layout_tokens = match layout {
                                Some((t, w, p, s)) => {
                                    quote! { ::typedflake::BitLayout::new(#t, #w, #p, #s) }
                                }
                                None => quote! { ::typedflake::BitLayout::DEFAULT },
                            };
                            let epoch_tokens = match &epoch {
                                Some(ev) => epoch_to_tokens(ev),
                                None => quote! { ::typedflake::Epoch::DEFAULT },
                            };
                            let expr: syn::Expr = syn::parse_quote! {
                                ::typedflake::Config::new(#layout_tokens, #epoch_tokens)
                            };
                            return Ok(Some(expr));
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "expected #[typedflake(...)], e.g. #[typedflake(config = EXPR)] or #[typedflake(epoch = \"2025-01-01\")]",
                    ));
                }
            }
        }
    }
    Ok(None)
}

fn generate_standard_traits(name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                f.debug_tuple(::core::stringify!(#name))
                    .field(&self.0)
                    .finish()
            }
        }

        impl ::core::clone::Clone for #name {
            #[inline]
            fn clone(&self) -> Self {
                *self
            }
        }

        impl ::core::marker::Copy for #name {}

        impl ::core::cmp::PartialEq for #name {
            #[inline]
            fn eq(&self, other: &Self) -> ::core::primitive::bool {
                self.0 == other.0
            }
        }

        impl ::core::cmp::Eq for #name {}

        impl ::core::hash::Hash for #name {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }

        impl ::core::cmp::PartialOrd for #name {
            #[inline]
            fn partial_cmp(&self, other: &Self) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::option::Option::Some(::core::cmp::Ord::cmp(self, other))
            }
        }

        impl ::core::cmp::Ord for #name {
            #[inline]
            fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.0.cmp(&other.0)
            }
        }
    }
}

fn generate_generator_struct(
    name: &syn::Ident,
    vis: &syn::Visibility,
    generator_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    quote! {
        /// Typed generator wrapper that produces the correct ID type
        #vis struct #generator_name {
            inner: ::typedflake::Generator,
        }

        impl #generator_name {
            /// Generate an ID using pre-injected state, blocking on sequence exhaustion
            pub fn generate(&self) -> #name {
                let id = self.inner.generate();
                #name(id)
            }

            /// Get the worker_id this generator is bound to
            pub fn worker_id(&self) -> u64 {
                self.inner.worker_id()
            }

            /// Get the process_id this generator is bound to
            pub fn process_id(&self) -> u64 {
                self.inner.process_id()
            }
        }
    }
}

fn generate_inherent_methods(
    name: &syn::Ident,
    generator_name: &syn::Ident,
    config_init: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        impl #name {
            /// Returns the singleton IdContext for this ID type (lazy static initialization)
            fn context() -> &'static ::typedflake::IdContext {
                static CONTEXT: ::std::sync::OnceLock<::typedflake::IdContext> =
                    ::std::sync::OnceLock::new();
                CONTEXT.get_or_init(|| ::typedflake::IdContext::new(#config_init))
            }

            /// Generate a new ID with default instance, blocking until next millisecond if sequence is exhausted
            pub fn generate() -> Self {
                let id = Self::context().default_generator().generate();
                Self(id)
            }

            /// Create a typed generator wrapper with specific worker_id and process_id
            pub fn instance(
                worker_id: u64,
                process_id: u64,
            ) -> ::core::result::Result<#generator_name, ::typedflake::ValidationError> {
                let inner = Self::context().create_generator(worker_id, process_id)?;
                ::core::result::Result::Ok(#generator_name { inner })
            }

            /// Create a typed generator wrapper with specific worker_id and default process_id
            pub fn worker(
                worker_id: u64,
            ) -> ::core::result::Result<#generator_name, ::typedflake::ValidationError> {
                let inner = Self::context().create_worker(worker_id)?;
                ::core::result::Result::Ok(#generator_name { inner })
            }

            /// Create a typed generator wrapper with default worker_id and specific process_id
            pub fn process(
                process_id: u64,
            ) -> ::core::result::Result<#generator_name, ::typedflake::ValidationError> {
                let inner = Self::context().create_process(process_id)?;
                ::core::result::Result::Ok(#generator_name { inner })
            }

            /// Decompose the ID into its components as a tuple
            pub fn decompose(self) -> (u64, u64, u64, u64) {
                Self::context().default_generator().decompose(self.0)
            }

            /// Get the ID components as a struct
            pub fn components(self) -> ::typedflake::IdComponents {
                Self::context().default_generator().components(self.0)
            }

            /// Get just the timestamp component
            pub fn timestamp(self) -> u64 {
                Self::context().default_generator().extract_timestamp(self.0)
            }

            /// Get just the worker ID component
            pub fn worker_id(self) -> u64 {
                Self::context().default_generator().extract_worker_id(self.0)
            }

            /// Get just the process ID component
            pub fn process_id(self) -> u64 {
                Self::context().default_generator().extract_process_id(self.0)
            }

            /// Get just the sequence component
            pub fn sequence(self) -> u64 {
                Self::context().default_generator().extract_sequence(self.0)
            }

            /// Compose ID with validation (uses default worker_id and process_id)
            pub fn compose(
                timestamp: u64,
                sequence: u64,
            ) -> ::core::result::Result<Self, ::typedflake::ValidationError> {
                let id = Self::context()
                    .default_generator()
                    .compose(timestamp, sequence)?;
                ::core::result::Result::Ok(Self(id))
            }

            /// Compose ID without validation (uses defaults, masks overflow)
            pub fn compose_unchecked(timestamp: u64, sequence: u64) -> Self {
                let id = Self::context()
                    .default_generator()
                    .compose_unchecked(timestamp, sequence);
                Self(id)
            }

            /// Compose an ID from all components with validation
            pub fn compose_custom(
                timestamp: u64,
                worker_id: u64,
                process_id: u64,
                sequence: u64,
            ) -> ::core::result::Result<Self, ::typedflake::ValidationError> {
                let id = Self::context().default_generator().compose_custom(
                    timestamp, worker_id, process_id, sequence,
                )?;
                ::core::result::Result::Ok(Self(id))
            }

            /// Compose from all components without validation (values exceeding limits are masked)
            pub fn compose_custom_unchecked(
                timestamp: u64,
                worker_id: u64,
                process_id: u64,
                sequence: u64,
            ) -> Self {
                let id = Self::context().default_generator().compose_custom_unchecked(
                    timestamp, worker_id, process_id, sequence,
                );
                Self(id)
            }

            /// Create an ID from a raw u64 value without validation.
            pub fn from_u64_unchecked(id: u64) -> Self {
                Self(id)
            }

            /// Create an ID from a raw u64 value with validation.
            pub fn try_from_u64(
                id: u64,
            ) -> ::core::result::Result<Self, ::typedflake::ValidationError> {
                Self::context().config().validate_id(id)?;
                ::core::result::Result::Ok(Self(id))
            }

            /// Get the raw u64 value of this ID
            pub fn as_u64(self) -> u64 {
                self.0
            }
        }
    }
}

fn generate_conversion_impls(name: &syn::Ident) -> proc_macro2::TokenStream {
    quote! {
        impl ::core::fmt::Display for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::write!(f, "{}", self.0)
            }
        }

        impl ::core::convert::TryFrom<u64> for #name {
            type Error = ::typedflake::ValidationError;

            fn try_from(id: u64) -> ::core::result::Result<Self, Self::Error> {
                Self::try_from_u64(id)
            }
        }

        impl ::core::convert::From<#name> for u64 {
            fn from(id: #name) -> u64 {
                id.0
            }
        }

        impl ::core::str::FromStr for #name {
            type Err = ::typedflake::ValidationError;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                let id = s.parse::<u64>()?;
                Self::try_from_u64(id)
            }
        }
    }
}

fn generate_serde_impls(name: &syn::Ident) -> proc_macro2::TokenStream {
    if cfg!(feature = "serde") {
        quote! {
            impl ::typedflake::__serde::Serialize for #name {
                fn serialize<S>(
                    &self,
                    serializer: S,
                ) -> ::core::result::Result<S::Ok, S::Error>
                where
                    S: ::typedflake::__serde::Serializer,
                {
                    serializer.serialize_str(&::std::string::ToString::to_string(self))
                }
            }

            impl<'de> ::typedflake::__serde::Deserialize<'de> for #name {
                fn deserialize<D>(
                    deserializer: D,
                ) -> ::core::result::Result<Self, D::Error>
                where
                    D: ::typedflake::__serde::Deserializer<'de>,
                {
                    let s = <::std::string::String as ::typedflake::__serde::Deserialize>::deserialize(deserializer)?;
                    ::core::str::FromStr::from_str(&s)
                        .map_err(::typedflake::__serde::de::Error::custom)
                }
            }
        }
    } else {
        quote! {}
    }
}
