//! Derive macros for unison-kdl
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
// Deserialize implementation
// ============================================================================

fn impl_kdl_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container_attrs = parse_container_attrs(&input.attrs)?;

    // Dispatch based on data type
    match &input.data {
        Data::Enum(data) => {
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

    let mut arg_index = 0usize;
    let mut field_deserializers = Vec::new();

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
                            .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if is_option_type(&field.ty) {
                    quote! {
                        #field_name: node.arg(#idx)
                            .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?,
                    }
                } else {
                    quote! {
                        #field_name: node.arg(#idx)
                            .ok_or(::unison_kdl::Error::MissingArgument(#idx))
                            .and_then(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))?,
                    }
                }
            }
            FieldKind::Property => {
                if field_attrs.default {
                    quote! {
                        #field_name: node.prop(#kdl_name)
                            .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if is_option_type(&field.ty) {
                    quote! {
                        #field_name: node.prop(#kdl_name)
                            .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                            .transpose()?,
                    }
                } else {
                    quote! {
                        #field_name: node.prop(#kdl_name)
                            .ok_or(::unison_kdl::Error::MissingField(#kdl_name))
                            .and_then(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))?,
                    }
                }
            }
            FieldKind::Child => {
                let child_name = field_attrs.child_name.as_ref().unwrap_or(&kdl_name);

                if field_attrs.unwrap_arg {
                    // Child node → extract first argument value
                    if is_option_type(&field.ty) {
                        quote! {
                            #field_name: node.child(#child_name)
                                .and_then(|n| n.arg(0))
                                .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                                .transpose()?,
                        }
                    } else if field_attrs.default {
                        quote! {
                            #field_name: node.child(#child_name)
                                .and_then(|n| n.arg(0))
                                .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                                .transpose()?
                                .unwrap_or_default(),
                        }
                    } else {
                        quote! {
                            #field_name: {
                                let child_node = node.child(#child_name)
                                    .ok_or(::unison_kdl::Error::MissingChild(#child_name))?;
                                let val = child_node.arg(0)
                                    .ok_or(::unison_kdl::Error::MissingArgument(0))?;
                                ::unison_kdl::FromKdlValue::from_kdl_value(val)?
                            },
                        }
                    }
                } else if field_attrs.unwrap_args {
                    // Child node → extract all arguments as Vec
                    quote! {
                        #field_name: node.child(#child_name)
                            .map(|n| n.args()
                                .into_iter()
                                .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                                .collect::<::unison_kdl::Result<Vec<_>>>())
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if field_attrs.default {
                    quote! {
                        #field_name: node.child(#child_name)
                            .map(|n| ::unison_kdl::KdlDeserialize::from_kdl_node(n))
                            .transpose()?
                            .unwrap_or_default(),
                    }
                } else if is_option_type(&field.ty) {
                    quote! {
                        #field_name: node.child(#child_name)
                            .map(|n| ::unison_kdl::KdlDeserialize::from_kdl_node(n))
                            .transpose()?,
                    }
                } else {
                    quote! {
                        #field_name: node.child(#child_name)
                            .ok_or(::unison_kdl::Error::MissingChild(#child_name))
                            .and_then(|n| ::unison_kdl::KdlDeserialize::from_kdl_node(n))?,
                    }
                }
            }
            FieldKind::Children => {
                let child_name = field_attrs.child_name.as_ref().unwrap_or(&kdl_name);
                quote! {
                    #field_name: node.children_by_name(#child_name)
                        .into_iter()
                        .map(|n| ::unison_kdl::KdlDeserialize::from_kdl_node(n))
                        .collect::<::unison_kdl::Result<Vec<_>>>()?,
                }
            }
            FieldKind::Arguments => {
                // Collect all arguments into a Vec
                quote! {
                    #field_name: node.args()
                        .into_iter()
                        .map(|v| ::unison_kdl::FromKdlValue::from_kdl_value(v))
                        .collect::<::unison_kdl::Result<Vec<_>>>()?,
                }
            }
            FieldKind::ChildMap => {
                // Collect child nodes into a HashMap (node name -> first arg value)
                let child_name = field_attrs.child_name.clone();
                if let Some(name) = child_name {
                    // Filter by child name first, then build map
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
                    // Direct children to map
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

    // Check node name if specified
    let name_check = if let Some(expected_name) = &container_attrs.name {
        quote! {
            if node.name().value() != #expected_name {
                return Err(::unison_kdl::Error::UnexpectedNode {
                    expected: #expected_name,
                    found: node.name().value().to_string(),
                });
            }
        }
    } else {
        quote! {}
    };

    if container_attrs.document {
        // Document-level deserialization: treat document nodes as children
        // Generate a helper that works with a slice of nodes (shared between from_kdl_doc and from_kdl_node)
        Ok(quote! {
            impl<'de> ::unison_kdl::KdlDeserialize<'de> for #name {
                fn from_kdl_node(node: &'de ::unison_kdl::KdlNode) -> ::unison_kdl::Result<Self> {
                    use ::unison_kdl::KdlNodeExt;
                    #name_check
                    Ok(Self {
                        #(#field_deserializers)*
                    })
                }

                fn from_kdl_doc(doc: &'de ::unison_kdl::KdlDocument) -> ::unison_kdl::Result<Self> {
                    // Create a virtual wrapper node with the document's nodes as children
                    let wrapper = ::unison_kdl::doc_to_wrapper_node(doc);
                    <Self as ::unison_kdl::KdlDeserialize>::from_kdl_node(&wrapper)
                }
            }
        })
    } else {
        Ok(quote! {
            impl<'de> ::unison_kdl::KdlDeserialize<'de> for #name {
                fn from_kdl_node(node: &'de ::unison_kdl::KdlNode) -> ::unison_kdl::Result<Self> {
                    use ::unison_kdl::KdlNodeExt;
                    #name_check
                    Ok(Self {
                        #(#field_deserializers)*
                    })
                }
            }
        })
    }
}

// ============================================================================
// Enum deserialization (scalar string mapping)
// ============================================================================

fn impl_kdl_deserialize_enum(
    _input: &DeriveInput,
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    // Validate: all variants must be unit variants (no data)
    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "KdlDeserialize for enums only supports unit variants (no data)",
            ));
        }
    }

    let match_arms: Vec<_> = variants
        .iter()
        .map(|variant| {
            let attrs = parse_variant_attrs(&variant.attrs).unwrap_or_default();
            let kdl_name = variant_kdl_name(variant, &attrs);
            let variant_ident = &variant.ident;
            quote! {
                #kdl_name => Ok(#name::#variant_ident),
            }
        })
        .collect();

    let variant_names: Vec<_> = variants
        .iter()
        .map(|v| {
            let attrs = parse_variant_attrs(&v.attrs).unwrap_or_default();
            variant_kdl_name(v, &attrs)
        })
        .collect();
    let expected_msg = variant_names.join(", ");

    Ok(quote! {
        impl<'de> ::unison_kdl::FromKdlValue<'de> for #name {
            fn from_kdl_value(value: &'de ::unison_kdl::KdlValue) -> ::unison_kdl::Result<Self> {
                let s = value.as_string()
                    .ok_or_else(|| ::unison_kdl::Error::type_mismatch("string", value))?;
                match s {
                    #(#match_arms)*
                    other => Err(::unison_kdl::Error::Custom(
                        format!("unknown variant '{}', expected one of: {}", other, #expected_msg)
                    )),
                }
            }
        }

        impl<'de> ::unison_kdl::KdlDeserialize<'de> for #name {
            fn from_kdl_node(node: &'de ::unison_kdl::KdlNode) -> ::unison_kdl::Result<Self> {
                use ::unison_kdl::KdlNodeExt;
                // For scalar enums, take the first argument as the value
                let value = node.arg(0)
                    .ok_or(::unison_kdl::Error::MissingArgument(0))?;
                <Self as ::unison_kdl::FromKdlValue>::from_kdl_value(value)
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

    // Collect fields by type for ordered serialization
    let mut arguments: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut multi_arguments: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut properties: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut children: Vec<(&Field, FieldAttrs)> = Vec::new();
    let mut child_maps: Vec<(&Field, FieldAttrs)> = Vec::new();

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
        }
    }

    // Sort arguments by index
    arguments.sort_by_key(|(_, attrs)| attrs.index.unwrap_or(usize::MAX));

    // Generate argument serializers
    let arg_serializers: Vec<_> = arguments
        .iter()
        .map(|(field, _)| {
            let field_name = field.ident.as_ref().unwrap();
            quote! {
                builder = builder.arg(&self.#field_name);
            }
        })
        .collect();

    // Generate multi-arguments serializers (Vec of args)
    let multi_arg_serializers: Vec<_> = multi_arguments
        .iter()
        .map(|(field, _)| {
            let field_name = field.ident.as_ref().unwrap();
            quote! {
                for item in &self.#field_name {
                    builder = builder.arg(item);
                }
            }
        })
        .collect();

    // Generate property serializers
    let prop_serializers: Vec<_> = properties
        .iter()
        .map(|(field, attrs)| {
            let field_name = field.ident.as_ref().unwrap();
            let kdl_name = attrs
                .rename
                .clone()
                .unwrap_or_else(|| field_name.to_string());

            if is_option_type(&field.ty) {
                quote! {
                    if let Some(ref v) = self.#field_name {
                        builder = builder.prop(#kdl_name, v);
                    }
                }
            } else {
                quote! {
                    builder = builder.prop(#kdl_name, &self.#field_name);
                }
            }
        })
        .collect();

    // Generate child serializers
    let child_serializers: Vec<_> = children
        .iter()
        .map(|(field, attrs)| {
            let field_name = field.ident.as_ref().unwrap();

            match attrs.kind {
                FieldKind::Child => {
                    if is_option_type(&field.ty) {
                        quote! {
                            if let Some(ref v) = self.#field_name {
                                builder = builder.child(::unison_kdl::KdlSerialize::to_kdl_node(v)?);
                            }
                        }
                    } else {
                        quote! {
                            builder = builder.child(::unison_kdl::KdlSerialize::to_kdl_node(&self.#field_name)?);
                        }
                    }
                }
                FieldKind::Children => {
                    quote! {
                        for item in &self.#field_name {
                            builder = builder.child(::unison_kdl::KdlSerialize::to_kdl_node(item)?);
                        }
                    }
                }
                _ => unreachable!(),
            }
        })
        .collect();

    // Generate child map serializers (HashMap -> child nodes)
    let child_map_serializers: Vec<_> = child_maps
        .iter()
        .map(|(field, attrs)| {
            let field_name = field.ident.as_ref().unwrap();
            let wrapper_name = attrs.child_name.as_ref();

            if let Some(name) = wrapper_name {
                // Wrap in a parent node
                quote! {
                    if !self.#field_name.is_empty() {
                        let mut wrapper = ::unison_kdl::NodeBuilder::new(#name);
                        for (key, value) in &self.#field_name {
                            wrapper = wrapper.child(
                                ::unison_kdl::NodeBuilder::new(key.as_str()).arg(value).build()
                            );
                        }
                        builder = builder.child(wrapper.build());
                    }
                }
            } else {
                // Direct children
                quote! {
                    for (key, value) in &self.#field_name {
                        builder = builder.child(
                            ::unison_kdl::NodeBuilder::new(key.as_str()).arg(value).build()
                        );
                    }
                }
            }
        })
        .collect();

    Ok(quote! {
        impl ::unison_kdl::KdlSerialize for #name {
            fn to_kdl_node(&self) -> ::unison_kdl::Result<::unison_kdl::KdlNode> {
                let mut builder = ::unison_kdl::NodeBuilder::new(#node_name);
                #(#arg_serializers)*
                #(#multi_arg_serializers)*
                #(#prop_serializers)*
                #(#child_serializers)*
                #(#child_map_serializers)*
                Ok(builder.build())
            }
        }
    })
}

// ============================================================================
// Enum serialization (scalar string mapping)
// ============================================================================

fn impl_kdl_serialize_enum(
    _input: &DeriveInput,
    name: &syn::Ident,
    variants: &syn::punctuated::Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<TokenStream2> {
    // Validate: all variants must be unit variants
    for variant in variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "KdlSerialize for enums only supports unit variants (no data)",
            ));
        }
    }

    let to_kdl_value_arms: Vec<_> = variants
        .iter()
        .map(|variant| {
            let attrs = parse_variant_attrs(&variant.attrs).unwrap_or_default();
            let kdl_name = variant_kdl_name(variant, &attrs);
            let variant_ident = &variant.ident;
            quote! {
                #name::#variant_ident => ::unison_kdl::KdlValue::String(#kdl_name.to_string()),
            }
        })
        .collect();

    let node_name = to_snake_case(&name.to_string());

    Ok(quote! {
        impl ::unison_kdl::ToKdlValue for #name {
            fn to_kdl_value(&self) -> ::unison_kdl::KdlValue {
                match self {
                    #(#to_kdl_value_arms)*
                }
            }
        }

        impl ::unison_kdl::ToKdlValue for &#name {
            fn to_kdl_value(&self) -> ::unison_kdl::KdlValue {
                (*self).to_kdl_value()
            }
        }

        impl ::unison_kdl::KdlSerialize for #name {
            fn to_kdl_node(&self) -> ::unison_kdl::Result<::unison_kdl::KdlNode> {
                Ok(::unison_kdl::NodeBuilder::new(#node_name)
                    .arg(self)
                    .build())
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
