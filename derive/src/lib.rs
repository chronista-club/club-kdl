//! Derive macros for unison-kdl
//!
//! Provides `#[derive(KdlDeserialize, KdlSerialize)]` macros.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, ExprLit, Field, Fields, Lit, Type, parse_macro_input,
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
}

#[derive(Debug, Default, Clone)]
struct FieldAttrs {
    kind: FieldKind,
    rename: Option<String>,
    index: Option<usize>,
    child_name: Option<String>,
    default: bool,
    skip: bool,
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
            }
            Ok(())
        })?;
    }

    Ok(result)
}

// ============================================================================
// Deserialize implementation
// ============================================================================

fn impl_kdl_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container_attrs = parse_container_attrs(&input.attrs)?;

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
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "KdlDeserialize only supports structs",
            ));
        }
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

                if field_attrs.default {
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

// ============================================================================
// Serialize implementation
// ============================================================================

fn impl_kdl_serialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container_attrs = parse_container_attrs(&input.attrs)?;

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
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "KdlSerialize only supports structs",
            ));
        }
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
// Helpers
// ============================================================================

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
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
