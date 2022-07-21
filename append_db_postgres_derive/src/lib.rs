use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, Fields};

#[proc_macro_derive(VersionedState)]
pub fn versioned_state_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_versioned_state(&ast)
}

fn impl_versioned_state(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        use append_db_postgres::update::{UpdateBodyError, SNAPSHOT_TAG};

        impl VersionedState for #name {
            fn deserialize_with_version(
                version: u16,
                value: serde_json::Value,
            ) -> Result<Self, append_db_postgres::update::UpdateBodyError> {
                serde_json::from_value(value.clone()).map_err(|e| {
                    append_db_postgres::update::UpdateBodyError::Deserialize(version, std::borrow::Cow::Borrowed(SNAPSHOT_TAG), e, value)
                })
            }
            fn get_version(&self) -> u16 {
                0
            }
            fn serialize(&self) -> Result<serde_json::Value, append_db_postgres::update::UpdateBodyError> {
                Ok(serde_json::to_value(&self)
                    .map_err(|e| append_db_postgres::update::UpdateBodyError::Serialize(std::borrow::Cow::Borrowed(SNAPSHOT_TAG), e))?)
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(HasUpdateTag)]
pub fn has_update_tag_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation
    impl_has_update_tag(&ast)
}

fn impl_has_update_tag(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let data = &ast.data;

    let deserialize_by_tag_body = impl_deserialize_by_tag(name, data);
    let impl_get_tag_body = impl_get_tag(name, data);
    let impl_serialize_untagged_body = impl_serialize_untagged(name, data);

    let gen = quote! {
        impl HasUpdateTag for #name {
            fn deserialize_by_tag(
                tag: &append_db_postgres::update::UpdateTag,
                version: u16,
                value: serde_json::Value,
            ) -> Result<Self, append_db_postgres::update::UpdateBodyError>
            where
                Self: std::marker::Sized,
            {
                #deserialize_by_tag_body
            }
            fn get_tag(&self) -> append_db_postgres::update::UpdateTag {
                #impl_get_tag_body
            }
            fn get_version(&self) -> u16 {
                0
            }
            fn serialize_untagged(&self) -> Result<serde_json::Value, append_db_postgres::update::UpdateBodyError> {
                #impl_serialize_untagged_body
            }
        }
    };
    gen.into()
}

fn enum_tags(data: &syn::Data) -> Vec<syn::Ident> {
    match data {
        Data::Enum(data_enum) => data_enum.variants.iter().map(|x| x.ident.clone()).collect(),
        _ => panic!("HasUpdateTag is only implemented for enums"),
    }
}

fn enum_tag(name: &syn::Ident) -> String {
    name.to_string().to_case(Case::Snake)
}

fn impl_deserialize_by_tag(name: &syn::Ident, data: &syn::Data) -> TokenStream2 {
    let tags = enum_tags(data);
    let mut variant_checkers = TokenStream2::new();
    for (i, tag) in tags.iter().enumerate() {
        let tag_str = enum_tag(&tag);
        if i == 0 {
            variant_checkers.extend(quote! {
                if tag == #tag_str {
                    Ok(#name::#tag(
                        serde_json::from_value(value.clone()).map_err(|e| {
                            append_db_postgres::update::UpdateBodyError::Deserialize(version, tag.to_owned(), e, value)
                        })?,
                    ))
                }
            })
        } else {
            variant_checkers.extend(quote! {
                else if tag == #tag_str {
                    Ok(#name::#tag(
                        serde_json::from_value(value.clone()).map_err(|e| {
                            append_db_postgres::update::UpdateBodyError::Deserialize(version, tag.to_owned(), e, value)
                        })?,
                    ))
                }
            })
        }
    }
    if !tags.is_empty() {
        variant_checkers.extend(quote! {
            else {
                Err(append_db_postgres::update::UpdateBodyError::UnknownTag(append_db_postgres::update::UnknownUpdateTag(
                    tag.to_string(),
                )))
            }
        });
    } else {
        variant_checkers.extend(quote! {
            Err(append_db_postgres::update::UpdateBodyError::UnknownTag(append_db_postgres::update::UnknownUpdateTag(
                tag.to_string(),
            )))
        });
    }
    
    variant_checkers
}

fn impl_get_tag(name: &syn::Ident, data: &syn::Data) -> TokenStream2 {
    let mut matches = TokenStream2::new();
    match data {
        Data::Enum(data_enum) if data_enum.variants.is_empty() => {
            matches.extend(quote! {
                _ => todo!(),
            })
        }
        Data::Enum(data_enum) => {
            for variant in data_enum.variants.iter() {
                // Variant's name
                let variant_name = &variant.ident;
                // Variant can have unnamed fields like `Variant(i32, i64)`
                // Variant can have named fields like `Variant {x: i32, y: i32}`
                // Variant can be named Unit like `Variant`
                let fields_in_variant = match &variant.fields {
                    Fields::Unnamed(_) => quote_spanned! {variant.span()=> (..) },
                    Fields::Unit => quote_spanned! { variant.span()=> },
                    Fields::Named(_) => quote_spanned! {variant.span()=> {..} },
                };
                let tag_str = enum_tag(variant_name);

                matches.extend(quote! {
                    #name::#variant_name #fields_in_variant => std::borrow::Cow::Borrowed(#tag_str),
                })
            }
        }
        _ => panic!("HasUpdateTag is only implemented for enums"),
    }

    quote! {
        match self {
            #matches
        }
    }
}

fn impl_serialize_untagged(name: &syn::Ident, data: &syn::Data) -> TokenStream2 {
    let mut matches = TokenStream2::new();
    match data {
        Data::Enum(data_enum) if data_enum.variants.is_empty() => {
            matches.extend(quote! {
                _ => todo!(),
            })
        }
        Data::Enum(data_enum) => {
            for variant in data_enum.variants.iter() {
                // Variant's name
                let variant_name = &variant.ident;
                let fields_in_variant = match &variant.fields {
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                        quote_spanned! {variant.span()=> (v) }
                    }
                    _ => panic!(
                        "HasUpdateTag deriving requires that every enum has one unamed field"
                    ),
                };

                matches.extend(quote! {
                    #name::#variant_name #fields_in_variant => Ok(serde_json::to_value(&v)
                    .map_err(|e| append_db_postgres::update::UpdateBodyError::Serialize(self.get_tag(), e))?),
                })
            }
        }
        _ => panic!("HasUpdateTag is only implemented for enums"),
    }

    quote! {
        match self {
            #matches
        }
    }
}
