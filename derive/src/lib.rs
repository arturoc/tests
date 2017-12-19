extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate syn;

use proc_macro::TokenStream;
use syn::{Ident, MacroInput, MetaItem, NestedMetaItem};
use quote::Tokens;

#[proc_macro_derive(Component, attributes(storage))]
pub fn component(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_component(&ast);
    gen.parse().unwrap()
}

fn impl_single_tuple_struct(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let storage = ast.attrs.first()
        .and_then(|attr| match attr.value {
            MetaItem::List(ref ident, ref items) if ident == "storage" => items.first(),
            _ => None,
        })
        .and_then(|attr| match *attr {
            NestedMetaItem::MetaItem(ref item) => Some(item),
            _ => None,
        })
        .and_then(|attr| match *attr {
            MetaItem::Word(ref ident) => Some(ident),
            _ => None,
        })
        .cloned()
        .unwrap_or(Ident::new("DenseVec"));

    if let &syn::Body::Struct(ref variants) = &ast.body{
        if let &syn::VariantData::Tuple(ref fields) = variants{
            if fields.len() == 1{
                let field_ty = &fields[0].ty;
                return quote! {
                    impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
                        type Storage = ::rinecs::#storage<#name>;
                        fn type_name() -> &'static str{
                            stringify!(#name)
                        }
                    }

                    impl #impl_generics ::std::ops::Deref for #name #ty_generics #where_clause {
                        type Target = #field_ty;
                        fn deref(&self) -> &#field_ty{
                            &self.0
                        }
                    }

                    impl #impl_generics ::std::ops::DerefMut for #name #ty_generics #where_clause {
                        fn deref_mut(&mut self) -> &mut #field_ty{
                            &mut self.0
                        }
                    }
                };
            }
        }
    }

    panic!("Unimplemented");
}

fn impl_other_structs(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let storage = ast.attrs.first()
        .and_then(|attr| match attr.value {
            MetaItem::List(ref ident, ref items) if ident == "storage" => items.first(),
            _ => None,
        })
        .and_then(|attr| match *attr {
            NestedMetaItem::MetaItem(ref item) => Some(item),
            _ => None,
        })
        .and_then(|attr| match *attr {
            MetaItem::Word(ref ident) => Some(ident),
            _ => None,
        })
        .cloned()
        .unwrap_or(Ident::new("DenseVec"));
    quote! {
        impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
            type Storage = ::rinecs::#storage<#name>;
            fn type_name() -> &'static str{
                stringify!(#name)
            }
        }
    }
}

fn impl_component(ast: &MacroInput) -> Tokens {
    match &ast.body{
		&syn::Body::Struct(ref variants) => {
            match variants{
        		&syn::VariantData::Tuple(ref fields) => {
                    if fields.len() == 1{
                        return impl_single_tuple_struct(ast);
                    }
                }
    		    _ => ()
            }
        }
        _ => ()
	}
    impl_other_structs(ast)
}



#[proc_macro_derive(HierarchicalComponent)]
pub fn hierarchical_component(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_hierarchical_component(&ast);
    gen.parse().unwrap()
}

fn impl_hierarchical_single_tuple_struct(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    if let &syn::Body::Struct(ref variants) = &ast.body{
        if let &syn::VariantData::Tuple(ref fields) = variants{
            if fields.len() == 1{
                let field_ty = &fields[0].ty;
                return quote! {
                    impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
                        type Storage = ::rinecs::Forest<#name>;
                        fn type_name() -> &'static str{
                            stringify!(#name)
                        }
                    }

                    impl #impl_generics ::std::ops::Deref for #name #ty_generics #where_clause {
                        type Target = #field_ty;
                        fn deref(&self) -> &#field_ty{
                            &self.0
                        }
                    }

                    impl #impl_generics ::std::ops::DerefMut for #name #ty_generics #where_clause {
                        fn deref_mut(&mut self) -> &mut #field_ty{
                            &mut self.0
                        }
                    }
                };
            }
        }
    }

    panic!("Unimplemented");
}

fn impl_hierarchical_other_structs(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote! {
        impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
            type Storage = ::rinecs::Forest<#name>;
            fn type_name() -> &'static str{
                stringify!(#name)
            }
        }
    }
}

fn impl_hierarchical_component(ast: &MacroInput) -> Tokens {
    match &ast.body{
		&syn::Body::Struct(ref variants) => {
            match variants{
        		&syn::VariantData::Tuple(ref fields) => {
                    if fields.len() == 1{
                        return impl_hierarchical_single_tuple_struct(ast);
                    }
                }
    		    _ => ()
            }
        }
        _ => ()
	}
    impl_hierarchical_other_structs(ast)
}


#[proc_macro_derive(OneToNComponent)]
pub fn one_to_n_component(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_one_to_n_component(&ast);
    gen.parse().unwrap()
}

fn impl_one_to_n_single_tuple_struct(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    if let &syn::Body::Struct(ref variants) = &ast.body{
        if let &syn::VariantData::Tuple(ref fields) = variants{
            if fields.len() == 1{
                let field_ty = &fields[0].ty;
                return quote! {
                    impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
                        type Storage = ::rinecs::DenseOneToNVec<#name>;
                        fn type_name() -> &'static str{
                            stringify!(#name)
                        }
                    }

                    impl #impl_generics ::rinecs::OneToNComponent for #name #ty_generics #where_clause{

                    }

                    impl #impl_generics ::std::ops::Deref for #name #ty_generics #where_clause {
                        type Target = #field_ty;
                        fn deref(&self) -> &#field_ty{
                            &self.0
                        }
                    }

                    impl #impl_generics ::std::ops::DerefMut for #name #ty_generics #where_clause {
                        fn deref_mut(&mut self) -> &mut #field_ty{
                            &mut self.0
                        }
                    }
                };
            }
        }
    }

    panic!("Unimplemented");
}

fn impl_one_to_n_other_structs(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote! {
        impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
            type Storage = ::rinecs::DenseOneToNVec<#name>;
            fn type_name() -> &'static str{
                stringify!(#name)
            }
        }

        impl #impl_generics ::rinecs::OneToNComponent for #name #ty_generics #where_clause{

        }
    }
}

fn impl_one_to_n_component(ast: &MacroInput) -> Tokens {
    match &ast.body{
		&syn::Body::Struct(ref variants) => {
            match variants{
        		&syn::VariantData::Tuple(ref fields) => {
                    if fields.len() == 1{
                        return impl_one_to_n_single_tuple_struct(ast);
                    }
                }
    		    _ => ()
            }
        }
        _ => ()
	}
    impl_one_to_n_other_structs(ast)
}


#[proc_macro_derive(HierarchicalOneToNComponent)]
pub fn hierarchical_one_to_n_component(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();
    let gen = impl_hierarchical_one_to_n_component(&ast);
    gen.parse().unwrap()
}

fn impl_hierarchical_one_to_n_single_tuple_struct(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    if let &syn::Body::Struct(ref variants) = &ast.body{
        if let &syn::VariantData::Tuple(ref fields) = variants{
            if fields.len() == 1{
                let field_ty = &fields[0].ty;
                return quote! {
                    impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
                        type Storage = ::rinecs::OneToNForest<#name>;
                        fn type_name() -> &'static str{
                            stringify!(#name)
                        }
                    }

                    impl #impl_generics ::rinecs::HierarchicalOneToNComponent for #name #ty_generics #where_clause{

                    }

                    impl #impl_generics ::std::ops::Deref for #name #ty_generics #where_clause {
                        type Target = #field_ty;
                        fn deref(&self) -> &#field_ty{
                            &self.0
                        }
                    }

                    impl #impl_generics ::std::ops::DerefMut for #name #ty_generics #where_clause {
                        fn deref_mut(&mut self) -> &mut #field_ty{
                            &mut self.0
                        }
                    }
                };
            }
        }
    }

    panic!("Unimplemented");
}

fn impl_hierarchical_one_to_n_other_structs(ast: &MacroInput) -> Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    quote! {
        impl #impl_generics ::rinecs::Component for #name #ty_generics #where_clause {
            type Storage = ::rinecs::OneToNForest<#name>;
            fn type_name() -> &'static str{
                stringify!(#name)
            }
        }

        impl #impl_generics ::rinecs::HierarchicalOneToNComponent for #name #ty_generics #where_clause{

        }
    }
}

fn impl_hierarchical_one_to_n_component(ast: &MacroInput) -> Tokens {
    match &ast.body{
		&syn::Body::Struct(ref variants) => {
            match variants{
        		&syn::VariantData::Tuple(ref fields) => {
                    if fields.len() == 1{
                        return impl_hierarchical_one_to_n_single_tuple_struct(ast);
                    }
                }
    		    _ => ()
            }
        }
        _ => ()
	}
    impl_hierarchical_one_to_n_other_structs(ast)
}
