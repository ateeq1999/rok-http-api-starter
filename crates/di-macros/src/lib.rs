use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Fields, GenericArgument, ItemStruct, PathArguments, Token, Type};

/// Returns `Some(inner)` if `ty` is syntactically `Arc<inner>` (single type argument).
fn extract_arc_inner(ty: &Type) -> Option<Type> {
    let Type::Path(type_path) = ty else { return None };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Arc" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else { return None };
    if args.args.len() != 1 {
        return None;
    }
    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    }
}

/// `#[injectable]` on a named-field struct:
/// - generates `impl Struct { pub fn new(field1: Ty1, ...) -> Self }` for all fields.
/// - if every field is `#[inject]`, additionally generates `impl di::Injectable`,
///   resolving each field's inner type (`T` from `Arc<T>`) from the container.
/// - `#[inject]` fields must be `Arc<T>` or `Arc<dyn Trait>`.
#[proc_macro_attribute]
pub fn injectable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let struct_name = input.ident.clone();

    let Fields::Named(fields_named) = &mut input.fields else {
        return syn::Error::new_spanned(
            &input,
            "#[injectable] only supports structs with named fields",
        )
        .to_compile_error()
        .into();
    };

    struct FieldInfo {
        ident: syn::Ident,
        ty: Type,
        injected: bool,
        inner_ty: Option<Type>,
    }

    let mut field_infos = Vec::new();
    let mut error: Option<syn::Error> = None;

    for field in fields_named.named.iter_mut() {
        let ident = field.ident.clone().unwrap();
        let inject_idx = field.attrs.iter().position(|a| a.path().is_ident("inject"));
        let injected = inject_idx.is_some();
        if let Some(idx) = inject_idx {
            field.attrs.remove(idx);
        }

        let inner_ty = extract_arc_inner(&field.ty);

        if injected && inner_ty.is_none() && error.is_none() {
            error = Some(syn::Error::new_spanned(
                &field.ty,
                "#[inject] fields must be `Arc<T>` or `Arc<dyn Trait>`",
            ));
        }

        field_infos.push(FieldInfo {
            ident,
            ty: field.ty.clone(),
            injected,
            inner_ty,
        });
    }

    if let Some(e) = error {
        return e.to_compile_error().into();
    }

    let ctor_params = field_infos.iter().map(|f| {
        let ident = &f.ident;
        let ty = &f.ty;
        quote! { #ident: #ty }
    });
    let ctor_field_idents = field_infos.iter().map(|f| &f.ident);

    let new_impl = quote! {
        impl #struct_name {
            pub fn new(#(#ctor_params),*) -> Self {
                Self { #(#ctor_field_idents),* }
            }
        }
    };

    // Vacuously true for a zero-field struct — a leaf provider with no dependencies.
    let all_injected = field_infos.iter().all(|f| f.injected);

    let injectable_impl = if all_injected {
        let struct_name_str = struct_name.to_string();
        let resolves = field_infos.iter().map(|f| {
            let ident = &f.ident;
            let inner = f.inner_ty.as_ref().unwrap();
            let field_name_str = ident.to_string();
            quote! {
                let #ident = container.get::<#inner>().ok_or_else(|| {
                    ::di::DiError::missing(#struct_name_str, #field_name_str, ::std::any::type_name::<#inner>())
                })?;
            }
        });
        let field_idents = field_infos.iter().map(|f| &f.ident);
        quote! {
            impl ::di::Injectable for #struct_name {
                fn construct(container: &::di::ContainerBuilder) -> ::std::result::Result<::std::sync::Arc<Self>, ::di::DiError> {
                    #(#resolves)*
                    Ok(::std::sync::Arc::new(Self::new(#(#field_idents),*)))
                }
            }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        #input
        #new_impl
        #injectable_impl
    };
    output.into()
}

struct ProviderEntry {
    ty: Type,
    as_trait: Option<Type>,
}

impl Parse for ProviderEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ty: Type = input.parse()?;
        let as_trait = if input.peek(Token![as]) {
            input.parse::<Token![as]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(ProviderEntry { ty, as_trait })
    }
}

enum ModuleField {
    Providers(Vec<ProviderEntry>),
    Imports(Vec<Type>),
    Controllers(Vec<Type>),
}

impl Parse for ModuleField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let content;
        syn::bracketed!(content in input);
        match ident.to_string().as_str() {
            "providers" => {
                let items: Punctuated<ProviderEntry, Token![,]> =
                    Punctuated::parse_terminated(&content)?;
                Ok(ModuleField::Providers(items.into_iter().collect()))
            }
            "imports" => {
                let items: Punctuated<Type, Token![,]> = Punctuated::parse_terminated(&content)?;
                Ok(ModuleField::Imports(items.into_iter().collect()))
            }
            "controllers" => {
                let items: Punctuated<Type, Token![,]> = Punctuated::parse_terminated(&content)?;
                Ok(ModuleField::Controllers(items.into_iter().collect()))
            }
            other => Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown #[module] key `{other}`, expected `providers`, `imports`, or `controllers`"
                ),
            )),
        }
    }
}

#[derive(Default)]
struct ModuleArgs {
    providers: Vec<ProviderEntry>,
    imports: Vec<Type>,
    controllers: Vec<Type>,
}

impl Parse for ModuleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = ModuleArgs::default();
        let fields: Punctuated<ModuleField, Token![,]> = Punctuated::parse_terminated(input)?;
        for field in fields {
            match field {
                ModuleField::Providers(v) => args.providers = v,
                ModuleField::Imports(v) => args.imports = v,
                ModuleField::Controllers(v) => args.controllers = v,
            }
        }
        Ok(args)
    }
}

/// `#[module(providers = [Impl as dyn Trait, Plain], imports = [OtherModule], controllers = [Ctrl])]`
/// on a (typically unit) marker struct.
///
/// - `providers` are constructed in declared order via `di::Injectable::construct`; list leaf
///   dependencies before the things that depend on them. `Impl as dyn Trait` binds the built
///   instance under the trait's type; a bare `Impl` registers it under its own concrete type.
/// - `imports` registers other modules first (each import is only ever constructed once, even
///   if multiple modules import it).
/// - `controllers` is inert in v1: parsed (so a typo is a compile error) but only emitted as a
///   documentation-only `CONTROLLERS` const — Axum's free-function-handler model doesn't need
///   controller-class construction the way Nest does.
#[proc_macro_attribute]
pub fn module(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ModuleArgs);
    let input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    let import_calls = args.imports.iter().map(|m| {
        quote! { <#m as ::di::Module>::register(builder)?; }
    });

    let provider_calls = args.providers.iter().map(|p| {
        let ty = &p.ty;
        if let Some(trait_ty) = &p.as_trait {
            quote! {
                let __instance = <#ty as ::di::Injectable>::construct(builder)?;
                builder.insert_arc::<#ty>(__instance.clone());
                builder.bind::<#trait_ty>().to_arc(__instance as ::std::sync::Arc<#trait_ty>);
            }
        } else {
            quote! {
                let __instance = <#ty as ::di::Injectable>::construct(builder)?;
                builder.insert_arc::<#ty>(__instance);
            }
        }
    });

    let controllers = &args.controllers;
    let controller_names = controllers.iter().map(|c| quote! { stringify!(#c) });
    // Forces each controller path to actually resolve, so a typo is a compile error
    // even though the const below only stores its name as a string.
    let controller_type_asserts = controllers.iter().map(|c| {
        quote! { let _: ::std::marker::PhantomData<#c>; }
    });

    let output = quote! {
        #input

        impl #struct_name {
            #[allow(dead_code)]
            pub const CONTROLLERS: &'static [&'static str] = &[ #(#controller_names),* ];

            #[allow(dead_code, unused_variables, path_statements)]
            fn __assert_controllers_resolve() {
                #(#controller_type_asserts)*
            }
        }

        impl ::di::Module for #struct_name {
            fn register(builder: &mut ::di::ContainerBuilder) -> ::std::result::Result<(), ::di::DiError> {
                if !builder.mark_registered::<#struct_name>() {
                    return Ok(());
                }
                #(#import_calls)*
                #(#provider_calls)*
                Ok(())
            }
        }
    };
    output.into()
}
