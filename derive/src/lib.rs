//! Derive macros for club-kdl
//!
//! Provides `#[derive(KdlDeserialize, KdlSerialize)]` macros.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprLit, Field, Fields, Lit, Type, Variant,
    parse_macro_input,
};

/// Derive `KdlDeserialize` for a struct
#[proc_macro_derive(KdlDeserialize, attributes(kdl))]
pub fn derive_kdl_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_kdl_deserialize(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive `KdlSerialize` for a struct
#[proc_macro_derive(KdlSerialize, attributes(kdl))]
pub fn derive_kdl_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_kdl_serialize(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

// ============================================================================
// Attribute parsing (syn 2.0 compatible)
// ============================================================================

#[derive(Debug, Default)]
struct ContainerAttrs {
    name: Option<String>,
    aliases: Vec<String>,
    document: bool,
}

#[derive(Debug, Default, Clone)]
struct FieldAttrs {
    kind: FieldKind,
    rename: Option<String>,
    index: Option<usize>,
    child_name: Option<String>,
    default: bool,
    skip: bool,
    unwrap_arg: bool,
    unwrap_args: bool,
}

#[derive(Debug, Default, Clone, PartialEq)]
enum FieldKind {
    #[default]
    Property,
    Argument,
    Arguments, // Collect all arguments into Vec
    Child,
    Children,
    ChildMap, // HashMap from child nodes (name -> first arg)
    Flatten,  // Flatten nested struct fields
}

fn parse_container_attrs(attrs: &[Attribute]) -> syn::Result<ContainerAttrs> {
    let mut result = ContainerAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("kdl") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    result.name = Some(s.value());
                }
            } else if meta.path.is_ident("alias") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    result.aliases.push(s.value());
                }
            } else if meta.path.is_ident("document") {
                result.document = true;
            }
            Ok(())
        })?;
    }

    Ok(result)
}

fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut result = FieldAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("kdl") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("argument") {
                result.kind = FieldKind::Argument;
                // Check for nested (index = N)
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("index") {
                            let value: Expr = nested.value()?.parse()?;
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Int(i), ..
                            }) = value
                            {
                                result.index = Some(i.base10_parse()?);
                            }
                        }
                        Ok(())
                    })?;
                }
            } else if meta.path.is_ident("arguments") {
                result.kind = FieldKind::Arguments;
            } else if meta.path.is_ident("property") {
                result.kind = FieldKind::Property;
                // Check for nested (rename = "...")
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("rename") {
                            let value: Expr = nested.value()?.parse()?;
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(s), ..
                            }) = value
                            {
                                result.rename = Some(s.value());
                            }
                        }
                        Ok(())
                    })?;
                }
            } else if meta.path.is_ident("child_map") {
                result.kind = FieldKind::ChildMap;
                // Check for nested (name = "...")
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("name") {
                            let value: Expr = nested.value()?.parse()?;
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(s), ..
                            }) = value
                            {
                                result.child_name = Some(s.value());
                            }
                        }
                        Ok(())
                    })?;
                }
            } else if meta.path.is_ident("child") {
                result.kind = FieldKind::Child;
                // Check for nested (name = "...")
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("name") {
                            let value: Expr = nested.value()?.parse()?;
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(s), ..
                            }) = value
                            {
                                result.child_name = Some(s.value());
                            }
                        }
                        Ok(())
                    })?;
                }
            } else if meta.path.is_ident("children") {
                result.kind = FieldKind::Children;
                // Check for nested (name = "...")
                if meta.input.peek(syn::token::Paren) {
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("name") {
                            let value: Expr = nested.value()?.parse()?;
                            if let Expr::Lit(ExprLit {
                                lit: Lit::Str(s), ..
                            }) = value
                            {
                                result.child_name = Some(s.value());
                            }
                        }
                        Ok(())
                    })?;
                }
            } else if meta.path.is_ident("rename") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    result.rename = Some(s.value());
                }
            } else if meta.path.is_ident("name") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    result.child_name = Some(s.value());
                }
            } else if meta.path.is_ident("index") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Int(i), ..
                }) = value
                {
                    result.index = Some(i.base10_parse()?);
                }
            } else if meta.path.is_ident("flatten") {
                result.kind = FieldKind::Flatten;
            } else if meta.path.is_ident("default") {
                result.default = true;
            } else if meta.path.is_ident("skip") {
                result.skip = true;
            } else if meta.path.is_ident("unwrap_arg") {
                result.unwrap_arg = true;
            } else if meta.path.is_ident("unwrap_args") {
                result.unwrap_args = true;
            }
            Ok(())
        })?;
    }

    Ok(result)
}

// ============================================================================
// Variant attribute parsing (for enums)
// ============================================================================

#[derive(Debug, Default)]
struct VariantAttrs {
    rename: Option<String>,
}

fn parse_variant_attrs(attrs: &[Attribute]) -> syn::Result<VariantAttrs> {
    let mut result = VariantAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("kdl") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = value
                {
                    result.rename = Some(s.value());
                }
            }
            Ok(())
        })?;
    }

    Ok(result)
}

/// Get the KDL string name for a variant (rename or snake_case of ident)
fn variant_kdl_name(variant: &Variant, attrs: &VariantAttrs) -> String {
    attrs
        .rename
        .clone()
        .unwrap_or_else(|| to_snake_case(&variant.ident.to_string()))
}

// ============================================================================
// Field deserializer generation (shared between structs and enum variants)
// ============================================================================

fn generate_field_deserializers<'a>(
    fields: impl Iterator<Item = &'a Field>,
) -> syn::Result<Vec<TokenStream2>> {
    let mut field_deserializers = Vec::new();
    let mut arg_index = 0usize;

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_attrs = parse_field_attrs(&field.attrs)?;

        if field_attrs.skip {
            field_deserializers.push(quote! {
                #field_name: Default::default(),
            });
            continue;
        }

        let kdl_name = field_attrs
            .rename
            .clone()
            .unwrap_or_else(|| field_name.to_string());

        let deserializer = match field_attrs.kind {
            FieldKind::Argument => {
                let idx = field_attrs.index.unwrap_or_else(|| {
                    let i = arg_index;
                    arg_index += 1;
                    i
                });

                if field_attrs.default {
                    quote! {
                        #field_name: node.arg(#idx)
                            .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if is_option_type(&field.ty) {
                    quote! {
                        #field_name: node.arg(#idx)
                            .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?,
                    }
                } else {
                    quote! {
                        #field_name: node.arg(#idx)
                            .ok_or(::club_kdl::Error::MissingArgument(#idx))
                            .and_then(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))?,
                    }
                }
            }
            FieldKind::Property => {
                if field_attrs.default {
                    quote! {
                        #field_name: node.prop(#kdl_name)
                            .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if is_option_type(&field.ty) {
                    quote! {
                        #field_name: node.prop(#kdl_name)
                            .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?,
                    }
                } else {
                    quote! {
                        #field_name: node.prop(#kdl_name)
                            .ok_or(::club_kdl::Error::MissingField(#kdl_name))
                            .and_then(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))?,
                    }
                }
            }
            FieldKind::Child => {
                if field_attrs.unwrap_arg {
                    let child_name = field_attrs.child_name.as_ref().unwrap_or(&kdl_name);
                    if is_option_type(&field.ty) {
                        quote! {
                            #field_name: node.child(#child_name)
                                .and_then(|n| n.arg(0))
                                .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                                .transpose()?,
                        }
                    } else if field_attrs.default {
                        quote! {
                            #field_name: node.child(#child_name)
                                .and_then(|n| n.arg(0))
                                .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                                .transpose()?
                                .unwrap_or_default(),
                        }
                    } else {
                        quote! {
                            #field_name: {
                                let child_node = node.child(#child_name)
                                    .ok_or(::club_kdl::Error::MissingChild(#child_name))?;
                                let val = child_node.arg(0)
                                    .ok_or(::club_kdl::Error::MissingArgument(0))?;
                                ::club_kdl::FromKdlValue::from_kdl_value(val)?
                            },
                        }
                    }
                } else if field_attrs.unwrap_args {
                    let child_name = field_attrs.child_name.as_ref().unwrap_or(&kdl_name);
                    quote! {
                        #field_name: node.child(#child_name)
                            .map(|n| n.args()
                                .into_iter()
                                .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                                .collect::<::club_kdl::Result<Vec<_>>>())
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if let Some(ref explicit_name) = field_attrs.child_name {
                    if field_attrs.default {
                        quote! {
                            #field_name: node.child(#explicit_name)
                                .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                                .transpose()?
                                .unwrap_or_default(),
                        }
                    } else if is_option_type(&field.ty) {
                        quote! {
                            #field_name: node.child(#explicit_name)
                                .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                                .transpose()?,
                        }
                    } else {
                        quote! {
                            #field_name: node.child(#explicit_name)
                                .ok_or(::club_kdl::Error::MissingChild(#explicit_name))
                                .and_then(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))?,
                        }
                    }
                } else {
                    let inner_ty = extract_option_inner_type(&field.ty);
                    let fallback = &kdl_name;
                    if field_attrs.default {
                        quote! {
                            #field_name: {
                                let __child_name = <#inner_ty as ::club_kdl::KdlDeserialize>::kdl_node_name()
                                    .unwrap_or(#fallback);
                                node.child(__child_name)
                                    .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                                    .transpose()?
                                    .unwrap_or_default()
                            },
                        }
                    } else if is_option_type(&field.ty) {
                        quote! {
                            #field_name: {
                                let __child_name = <#inner_ty as ::club_kdl::KdlDeserialize>::kdl_node_name()
                                    .unwrap_or(#fallback);
                                node.child(__child_name)
                                    .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                                    .transpose()?
                            },
                        }
                    } else {
                        quote! {
                            #field_name: {
                                let __child_name = <#inner_ty as ::club_kdl::KdlDeserialize>::kdl_node_name()
                                    .unwrap_or(#fallback);
                                node.child(__child_name)
                                    .ok_or(::club_kdl::Error::MissingChild(__child_name))
                                    .and_then(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))?
                            },
                        }
                    }
                }
            }
            FieldKind::Children => {
                if let Some(ref explicit_name) = field_attrs.child_name {
                    quote! {
                        #field_name: node.children_by_name(#explicit_name)
                            .into_iter()
                            .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                            .collect::<::club_kdl::Result<Vec<_>>>()?,
                    }
                } else {
                    let inner_ty = extract_vec_inner_type(&field.ty);
                    let fallback = &kdl_name;
                    quote! {
                        #field_name: {
                            if <#inner_ty as ::club_kdl::KdlDeserialize>::kdl_matches_any_node() {
                                // Data enum: collect all children and dispatch by node name
                                node.all_children()
                                    .into_iter()
                                    .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                                    .collect::<::club_kdl::Result<Vec<_>>>()?
                            } else {
                                let __child_name = <#inner_ty as ::club_kdl::KdlDeserialize>::kdl_node_name()
                                    .unwrap_or(#fallback);
                                node.children_by_name(__child_name)
                                    .into_iter()
                                    .map(|n| ::club_kdl::KdlDeserialize::from_kdl_node(n))
                                    .collect::<::club_kdl::Result<Vec<_>>>()?
                            }
                        },
                    }
                }
            }
            FieldKind::Arguments => {
                quote! {
                    #field_name: node.args()
                        .into_iter()
                        .map(|v| ::club_kdl::FromKdlValue::from_kdl_value(v))
                        .collect::<::club_kdl::Result<Vec<_>>>()?,
                }
            }
            FieldKind::Flatten => {
                let field_ty = &field.ty;
                quote! {
                    #field_name: <#field_ty as ::club_kdl::KdlDeserialize>::from_kdl_node(node)?,
                }
            }
            FieldKind::ChildMap => {
                let child_name = field_attrs.child_name.clone();
                if let Some(name) = child_name {
                    quote! {
                        #field_name: {
                            let wrapper = node.child(#name);
                            if let Some(w) = wrapper {
                                w.all_children()
                                    .into_iter()
                                    .filter_map(|n| {
                                        let key = n.name().value().to_string();
                                        let value = n.arg(0)
                                            .and_then(|v| v.as_string())
                                            .map(|s| s.to_string())?;
                                        Some((key, value))
                                    })
                                    .collect()
                            } else {
                                ::std::collections::HashMap::new()
                            }
                        },
                    }
                } else {
                    quote! {
                        #field_name: node.all_children()
                            .into_iter()
                            .filter_map(|n| {
                                let key = n.name().value().to_string();
                                let value = n.arg(0)
                                    .and_then(|v| v.as_string())
                                    .map(|s| s.to_string())?;
                                Some((key, value))
                            })
                            .collect(),
                    }
                }
            }
        };

        field_deserializers.push(deserializer);
    }

    Ok(field_deserializers)
}

// ============================================================================
// Deserialize implementation
// ============================================================================

fn impl_kdl_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container_attrs = parse_container_attrs(&input.attrs)?;

    // Dispatch based on data type
    match &input.data {
        Data::Enum(data) => {
            if container_attrs.document {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[kdl(document)] is not supported on enums",
                ));
            }
            return impl_kdl_deserialize_enum(input, name, &data.variants);
        }
        Data::Struct(_) => {} // fall through to struct handling below
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "KdlDeserialize does not support unions",
            ));
        }
    }

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "KdlDeserialize only supports structs with named fields",
                ));
            }
        },
        _ => unreachable!(),
    };

    let field_deserializers = generate_field_deserializers(fields.iter())?;

    // Validate: #[kdl(document)] + #[kdl(name = "...")] is unsupported
    if container_attrs.document && container_attrs.name.is_some() {
        return Err(syn::Error::new_spanned(
            input,
            "#[kdl(document)] cannot be combined with #[kdl(name = \"...\")]; \
             document-level deserialization uses a virtual wrapper node",
        ));
    }

    // Generate kdl_node_name() if #[kdl(name = "...")] is specified
    let kdl_node_name_impl = if let Some(ref expected_name) = container_attrs.name {
        quote! {
            fn kdl_node_name() -> Option<&'static str> {
                Some(#expected_name)
            }
        }
    } else {
        quote! {}
    };

    // Check node name if specified (with alias support)
    let name_check = if let Some(expected_name) = &container_attrs.name {
        let aliases = &container_attrs.aliases;
        if aliases.is_empty() {
            quote! {
                if node.name().value() != #expected_name {
                    return Err(::club_kdl::Error::UnexpectedNode {
                        expected: #expected_name,
                        found: node.name().value().to_string(),
                    });
                }
            }
        } else {
            quote! {
                {
                    let __name = node.name().value();
                    if __name != #expected_name #(&& __name != #aliases)* {
                        return Err(::club_kdl::Error::UnexpectedNode {
                            expected: #expected_name,
                            found: __name.to_string(),
                        });
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let struct_name_str = name.to_string();

    if container_attrs.document {
        Ok(quote! {
            impl<'de> ::club_kdl::KdlDeserialize<'de> for #name {
                fn from_kdl_node(node: &'de ::club_kdl::KdlNode) -> ::club_kdl::Result<Self> {
                    use ::club_kdl::KdlNodeExt;
                    #name_check
                    (|| -> ::club_kdl::Result<Self> {
                        Ok(Self {
                            #(#field_deserializers)*
                        })
                    })().map_err(|e| e.in_context(#struct_name_str))
                }

                #kdl_node_name_impl

                fn from_kdl_doc(doc: &'de ::club_kdl::KdlDocument) -> ::club_kdl::Result<Self> {
                    let wrapper = ::club_kdl::doc_to_wrapper_node(doc);
                    <Self as ::club_kdl::KdlDeserialize>::from_kdl_node(&wrapper)
                }
            }
        })
    } else {
        Ok(quote! {
            impl<'de> ::club_kdl::KdlDeserialize<'de> for #name {
                fn from_kdl_node(node: &'de ::club_kdl::KdlNode) -> ::club_kdl::Result<Self> {
                    use ::club_kdl::KdlNodeExt;
                    #name_check
                    (|| -> ::club_kdl::Result<Self> {
                        Ok(Self {
                            #(#field_deserializers)*
                        })
                    })().map_err(|e| e.in_context(#struct_name_str))
                }

                #kdl_node_name_impl
            }
        })
    }
}

// ============================================================================
// Enum deserialization
// ============================================================================

fn impl_kdl_deserialize_enum(
    _input: &DeriveInput,
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    let has_data_variants = variants.iter().any(|v| !matches!(v.fields, Fields::Unit));

    if has_data_variants {
        impl_kdl_deserialize_data_enum(name, variants)
    } else {
        impl_kdl_deserialize_scalar_enum(name, variants)
    }
}

/// Scalar enum: all unit variants, maps to/from string values
fn impl_kdl_deserialize_scalar_enum(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    let mut match_arms = Vec::new();
    let mut variant_names = Vec::new();

    for variant in variants {
        let attrs = parse_variant_attrs(&variant.attrs)?;
        let kdl_name = variant_kdl_name(variant, &attrs);
        let variant_ident = &variant.ident;
        match_arms.push(quote! {
            #kdl_name => Ok(#name::#variant_ident),
        });
        variant_names.push(kdl_name);
    }

    let expected_msg = variant_names.join(", ");

    Ok(quote! {
        impl<'de> ::club_kdl::FromKdlValue<'de> for #name {
            fn from_kdl_value(value: &'de ::club_kdl::KdlValue) -> ::club_kdl::Result<Self> {
                let s = value.as_string()
                    .ok_or_else(|| ::club_kdl::Error::type_mismatch("string", value))?;
                match s {
                    #(#match_arms)*
                    other => Err(::club_kdl::Error::Custom(
                        format!("unknown variant '{}', expected one of: {}", other, #expected_msg)
                    )),
                }
            }
        }

        impl<'de> ::club_kdl::KdlDeserialize<'de> for #name {
            fn from_kdl_node(node: &'de ::club_kdl::KdlNode) -> ::club_kdl::Result<Self> {
                use ::club_kdl::KdlNodeExt;
                let value = node.arg(0)
                    .ok_or(::club_kdl::Error::MissingArgument(0))?;
                <Self as ::club_kdl::FromKdlValue>::from_kdl_value(value)
            }
        }
    })
}

/// Data enum: variants with fields, node name determines the variant
fn impl_kdl_deserialize_data_enum(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    let mut match_arms = Vec::new();
    let mut variant_names = Vec::new();

    for variant in variants {
        let variant_ident = &variant.ident;
        let attrs = parse_variant_attrs(&variant.attrs)?;
        let kdl_name = variant_kdl_name(variant, &attrs);
        variant_names.push(kdl_name.clone());

        match &variant.fields {
            Fields::Unit => {
                match_arms.push(quote! {
                    #kdl_name => Ok(#name::#variant_ident),
                });
            }
            Fields::Named(fields) => {
                let field_deserializers = generate_field_deserializers(fields.named.iter())?;
                let enum_name_str = name.to_string();
                let variant_ctx = format!("{}::{}", enum_name_str, variant_ident);
                match_arms.push(quote! {
                    #kdl_name => {
                        (|| -> ::club_kdl::Result<Self> {
                            Ok(#name::#variant_ident {
                                #(#field_deserializers)*
                            })
                        })().map_err(|e| e.in_context(#variant_ctx))
                    },
                });
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(
                        variant,
                        "KdlDeserialize for enum tuple variants only supports single-field (newtype) variants",
                    ));
                }
                let inner_ty = &fields.unnamed.first().unwrap().ty;
                match_arms.push(quote! {
                    #kdl_name => {
                        // For newtype variants, the inner type may have its own name_check.
                        // Rename the node to match the inner type's expected name if needed.
                        let __inner_name = <#inner_ty as ::club_kdl::KdlDeserialize>::kdl_node_name();
                        if let Some(expected) = __inner_name {
                            if node.name().value() != expected {
                                let mut __renamed = node.clone();
                                *__renamed.name_mut() = ::club_kdl::KdlIdentifier::from(expected);
                                return Ok(#name::#variant_ident(
                                    <#inner_ty as ::club_kdl::KdlDeserialize>::from_kdl_node(&__renamed)?
                                ));
                            }
                        }
                        Ok(#name::#variant_ident(
                            <#inner_ty as ::club_kdl::KdlDeserialize>::from_kdl_node(node)?
                        ))
                    },
                });
            }
        }
    }

    let expected_msg = variant_names.join(", ");

    Ok(quote! {
        impl<'de> ::club_kdl::KdlDeserialize<'de> for #name {
            fn from_kdl_node(node: &'de ::club_kdl::KdlNode) -> ::club_kdl::Result<Self> {
                use ::club_kdl::KdlNodeExt;
                let __name = node.name().value();
                match __name {
                    #(#match_arms)*
                    other => Err(::club_kdl::Error::Custom(
                        format!("unknown variant '{}', expected one of: {}", other, #expected_msg)
                    )),
                }
            }

            fn kdl_matches_any_node() -> bool {
                true
            }
        }
    })
}

// ============================================================================
// Serialize implementation
// ============================================================================

fn impl_kdl_serialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container_attrs = parse_container_attrs(&input.attrs)?;

    // Dispatch based on data type
    match &input.data {
        Data::Enum(data) => {
            if container_attrs.document {
                return Err(syn::Error::new_spanned(
                    input,
                    "#[kdl(document)] is not supported on enums",
                ));
            }
            return impl_kdl_serialize_enum(input, name, &data.variants);
        }
        Data::Struct(_) => {} // fall through to struct handling below
        Data::Union(_) => {
            return Err(syn::Error::new_spanned(
                input,
                "KdlSerialize does not support unions",
            ));
        }
    }

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "KdlSerialize only supports structs with named fields",
                ));
            }
        },
        _ => unreachable!(),
    };

    let node_name = container_attrs
        .name
        .unwrap_or_else(|| to_snake_case(&name.to_string()));

    let all_serializers = generate_field_serializers(fields.iter(), false)?;

    if container_attrs.document {
        Ok(quote! {
            impl ::club_kdl::KdlSerialize for #name {
                fn to_kdl_node(&self) -> ::club_kdl::Result<::club_kdl::KdlNode> {
                    let mut builder = ::club_kdl::NodeBuilder::new("__document__");
                    #(#all_serializers)*
                    Ok(builder.build())
                }

                fn to_kdl_doc(&self) -> ::club_kdl::Result<::club_kdl::KdlDocument> {
                    let wrapper = self.to_kdl_node()?;
                    let mut doc = ::club_kdl::KdlDocument::new();
                    if let Some(children) = wrapper.children() {
                        for node in children.nodes() {
                            doc.nodes_mut().push(node.clone());
                        }
                    }
                    Ok(doc)
                }
            }
        })
    } else {
        Ok(quote! {
            impl ::club_kdl::KdlSerialize for #name {
                fn to_kdl_node(&self) -> ::club_kdl::Result<::club_kdl::KdlNode> {
                    let mut builder = ::club_kdl::NodeBuilder::new(#node_name);
                    #(#all_serializers)*
                    Ok(builder.build())
                }
            }
        })
    }
}

// ============================================================================
// Field serializer generation (shared between structs and enum variants)
// ============================================================================

/// Generate serializer tokens for struct fields.
///
/// When `is_variant` is `false` (struct context), fields are accessed via `self.field_name`.
/// When `is_variant` is `true` (enum variant context), fields are local bindings from match destructuring.
///
/// Returns tokens in order: arguments → multi-args → properties → children → child_maps → flattens.
fn generate_field_serializers<'a>(
    fields: impl Iterator<Item = &'a Field>,
    is_variant: bool,
) -> syn::Result<Vec<TokenStream2>> {
    let mut arguments: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut multi_arguments: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut properties: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut children: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut child_maps: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut flattens: Vec<(&Field, FieldAttrs)> = Vec::new();

    for field in fields {
        let attrs = parse_field_attrs(&field.attrs)?;
        if attrs.skip {
            continue;
        }
        match attrs.kind {
            FieldKind::Argument => arguments.push((field, attrs)),
            FieldKind::Arguments => multi_arguments.push((field, attrs)),
            FieldKind::Property => properties.push((field, attrs)),
            FieldKind::Child | FieldKind::Children => children.push((field, attrs)),
            FieldKind::ChildMap => child_maps.push((field, attrs)),
            FieldKind::Flatten => flattens.push((field, attrs)),
        }
    }

    arguments.sort_by_key(|(_, attrs)| attrs.index.unwrap_or(usize::MAX));

    let mut serializers = Vec::new();

    // Helper: generate the field access token and ref patterns based on mode
    macro_rules! field_access {
        ($field_name:ident) => {
            if is_variant {
                quote! { #$field_name }
            } else {
                quote! { self.#$field_name }
            }
        };
    }
    macro_rules! field_ref {
        ($field_name:ident) => {
            if is_variant {
                quote! { #$field_name }
            } else {
                quote! { &self.#$field_name }
            }
        };
    }
    macro_rules! option_some {
        ($field_name:ident) => {
            if is_variant {
                quote! { if let Some(v) = #$field_name }
            } else {
                quote! { if let Some(ref v) = self.#$field_name }
            }
        };
    }

    // Arguments
    for (field, _) in &arguments {
        let field_name = field.ident.as_ref().unwrap();
        let ref_access = field_ref!(field_name);
        serializers.push(quote! {
            builder = builder.arg(#ref_access);
        });
    }

    // Multi-arguments
    for (field, _) in &multi_arguments {
        let field_name = field.ident.as_ref().unwrap();
        let ref_access = field_ref!(field_name);
        serializers.push(quote! {
            for item in #ref_access {
                builder = builder.arg(item);
            }
        });
    }

    // Properties
    for (field, attrs) in &properties {
        let field_name = field.ident.as_ref().unwrap();
        let kdl_name = attrs
            .rename
            .clone()
            .unwrap_or_else(|| field_name.to_string());

        if is_option_type(&field.ty) {
            let opt_pat = option_some!(field_name);
            serializers.push(quote! {
                #opt_pat {
                    builder = builder.prop(#kdl_name, v);
                }
            });
        } else {
            let ref_access = field_ref!(field_name);
            serializers.push(quote! {
                builder = builder.prop(#kdl_name, #ref_access);
            });
        }
    }

    // Children (child + children)
    for (field, attrs) in &children {
        let field_name = field.ident.as_ref().unwrap();
        let kdl_name = attrs
            .child_name
            .clone()
            .unwrap_or_else(|| field_name.to_string());

        let s = match attrs.kind {
            FieldKind::Child if attrs.unwrap_arg => {
                if is_option_type(&field.ty) {
                    let opt_pat = option_some!(field_name);
                    quote! {
                        #opt_pat {
                            builder = builder.child(
                                ::club_kdl::NodeBuilder::new(#kdl_name).arg(v).build()
                            );
                        }
                    }
                } else {
                    let ref_access = field_ref!(field_name);
                    quote! {
                        builder = builder.child(
                            ::club_kdl::NodeBuilder::new(#kdl_name).arg(#ref_access).build()
                        );
                    }
                }
            }
            FieldKind::Child if attrs.unwrap_args => {
                let access = field_access!(field_name);
                let ref_access = field_ref!(field_name);
                quote! {
                    if !#access.is_empty() {
                        let mut __node = ::club_kdl::NodeBuilder::new(#kdl_name);
                        for item in #ref_access {
                            __node = __node.arg(item);
                        }
                        builder = builder.child(__node.build());
                    }
                }
            }
            FieldKind::Child => {
                if is_option_type(&field.ty) {
                    let opt_pat = option_some!(field_name);
                    quote! {
                        #opt_pat {
                            builder = builder.child(::club_kdl::KdlSerialize::to_kdl_node(v)?);
                        }
                    }
                } else {
                    let ref_access = field_ref!(field_name);
                    quote! {
                        builder = builder.child(::club_kdl::KdlSerialize::to_kdl_node(#ref_access)?);
                    }
                }
            }
            FieldKind::Children => {
                let ref_access = field_ref!(field_name);
                quote! {
                    for item in #ref_access {
                        builder = builder.child(::club_kdl::KdlSerialize::to_kdl_node(item)?);
                    }
                }
            }
            _ => unreachable!(),
        };
        serializers.push(s);
    }

    // Child maps
    for (field, attrs) in &child_maps {
        let field_name = field.ident.as_ref().unwrap();
        let access = field_access!(field_name);
        let ref_access = field_ref!(field_name);
        let wrapper_name = attrs.child_name.as_ref();

        if let Some(wname) = wrapper_name {
            serializers.push(quote! {
                if !#access.is_empty() {
                    let mut wrapper = ::club_kdl::NodeBuilder::new(#wname);
                    for (key, value) in #ref_access {
                        wrapper = wrapper.child(
                            ::club_kdl::NodeBuilder::new(key.as_str()).arg(value).build()
                        );
                    }
                    builder = builder.child(wrapper.build());
                }
            });
        } else {
            serializers.push(quote! {
                for (key, value) in #ref_access {
                    builder = builder.child(
                        ::club_kdl::NodeBuilder::new(key.as_str()).arg(value).build()
                    );
                }
            });
        }
    }

    // Flattens
    for (field, _) in &flattens {
        let field_name = field.ident.as_ref().unwrap();
        let ref_access = field_ref!(field_name);
        serializers.push(quote! {
            {
                let __flat_node = ::club_kdl::KdlSerialize::to_kdl_node(#ref_access)?;
                for entry in __flat_node.entries() {
                    builder = builder.entry(entry.clone());
                }
                if let Some(children) = __flat_node.children() {
                    for child in children.nodes() {
                        builder = builder.child(child.clone());
                    }
                }
            }
        });
    }

    Ok(serializers)
}

// ============================================================================
// Enum serialization
// ============================================================================

fn impl_kdl_serialize_enum(
    _input: &DeriveInput,
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    let has_data_variants = variants.iter().any(|v| !matches!(v.fields, Fields::Unit));

    if has_data_variants {
        impl_kdl_serialize_data_enum(name, variants)
    } else {
        impl_kdl_serialize_scalar_enum(name, variants)
    }
}

/// Scalar enum serialize: all unit variants → ToKdlValue + KdlSerialize
fn impl_kdl_serialize_scalar_enum(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    let mut to_kdl_value_arms = Vec::new();

    for variant in variants {
        let attrs = parse_variant_attrs(&variant.attrs)?;
        let kdl_name = variant_kdl_name(variant, &attrs);
        let variant_ident = &variant.ident;
        to_kdl_value_arms.push(quote! {
            #name::#variant_ident => ::club_kdl::KdlValue::String(#kdl_name.to_string()),
        });
    }

    let node_name = to_snake_case(&name.to_string());

    Ok(quote! {
        impl ::club_kdl::ToKdlValue for #name {
            fn to_kdl_value(&self) -> ::club_kdl::KdlValue {
                match self {
                    #(#to_kdl_value_arms)*
                }
            }
        }

        impl ::club_kdl::ToKdlValue for &#name {
            fn to_kdl_value(&self) -> ::club_kdl::KdlValue {
                (*self).to_kdl_value()
            }
        }

        impl ::club_kdl::KdlSerialize for #name {
            fn to_kdl_node(&self) -> ::club_kdl::Result<::club_kdl::KdlNode> {
                Ok(::club_kdl::NodeBuilder::new(#node_name)
                    .arg(self)
                    .build())
            }
        }
    })
}

/// Data enum serialize: variants with fields → node name = variant name
fn impl_kdl_serialize_data_enum(
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    let mut match_arms = Vec::new();

    for variant in variants {
        let variant_ident = &variant.ident;
        let attrs = parse_variant_attrs(&variant.attrs)?;
        let kdl_name = variant_kdl_name(variant, &attrs);

        match &variant.fields {
            Fields::Unit => {
                match_arms.push(quote! {
                    #name::#variant_ident => {
                        Ok(::club_kdl::NodeBuilder::new(#kdl_name).build())
                    },
                });
            }
            Fields::Named(fields) => {
                let field_serializers = generate_field_serializers(fields.named.iter(), true)?;
                let field_names: Vec<_> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().unwrap())
                    .collect();

                match_arms.push(quote! {
                    #name::#variant_ident { #(#field_names),* } => {
                        let mut builder = ::club_kdl::NodeBuilder::new(#kdl_name);
                        #(#field_serializers)*
                        Ok(builder.build())
                    },
                });
            }
            Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    return Err(syn::Error::new_spanned(
                        variant,
                        "KdlSerialize for enum tuple variants only supports single-field (newtype) variants",
                    ));
                }
                match_arms.push(quote! {
                    #name::#variant_ident(inner) => {
                        let mut __node = ::club_kdl::KdlSerialize::to_kdl_node(inner)?;
                        // Override the node name with the variant name
                        *__node.name_mut() = ::club_kdl::KdlIdentifier::from(#kdl_name);
                        Ok(__node)
                    },
                });
            }
        }
    }

    Ok(quote! {
        impl ::club_kdl::KdlSerialize for #name {
            fn to_kdl_node(&self) -> ::club_kdl::Result<::club_kdl::KdlNode> {
                match self {
                    #(#match_arms)*
                }
            }
        }
    })
}

// ============================================================================
// Helpers
// ============================================================================

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

/// Extract `T` from `Option<T>`. Returns the original type if not `Option`.
fn extract_option_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
    {
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                return inner;
            }
        }
    }
    ty
}

/// Extract `T` from `Vec<T>`. Returns the original type if not `Vec`.
fn extract_vec_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Vec"
    {
        if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                return inner;
            }
        }
    }
    ty
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
