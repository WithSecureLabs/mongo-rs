use inflector::cases::snakecase::to_snake_case;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Member, Type};

use crate::ast::{attr, Container, Data, Field, Style, Variant};

pub fn expand_derive_bson(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let container = Container::from(input)?;

    let body = match &container.data {
        Data::Struct(style, fields) => {
            match style {
                Style::Struct => {}
                _ => panic!("#[derive(Bson)] can only be derived on named structs"),
            }
            impl_struct(&container.ident, style, fields, &container.attrs)
        }
        Data::Enum(variants) => impl_enum(&container.ident, variants, &container.attrs),
    };

    Ok(quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const _: () = {
            extern crate mongod as _mongo;

            use std::convert::{TryFrom, TryInto};
            use _mongo::ext::bson::de::ErrorExt;

            #body
        };
    })
}

fn impl_enum(
    name: &Ident,
    variants: &[Variant],
    attrs: &attr::Container,
) -> proc_macro2::TokenStream {
    // This is how we handle conversion into bson, if people want more options then they will have
    // to use serde for now.
    // 1. If all variants are style `unit` then handle like a traditional enum.
    //      {
    //          "NAME": "VALUE",
    //      }
    // 2. Otherwise handle as a structure, where _type is injected into the struct.
    //      {
    //          "_type": "NAME",
    //          "0": "VALUE",
    //      }
    if variants.len() == variants.iter().filter(|v| v.style == Style::Unit).count() {
        impl_enum_unit(name, variants, attrs)
    } else {
        impl_enum_struct(name, variants, attrs)
    }
}

fn impl_enum_struct(
    name: &Ident,
    variants: &[Variant],
    attrs: &attr::Container,
) -> proc_macro2::TokenStream {
    let try_from_collection_fields = variants.iter().map(|v| {
        let id = &v.ident;
        let value = to_snake_case(&v.ident.to_string());
        let fields = v.fields.iter().map(|f| member_to_ident(&f.member));
        let values = v.fields.iter().map(|f| {
            let id = member_to_id(&f.member);
            let member = member_to_ident(&f.member);
            if f.attrs.serde {
                quote! {
                    doc.insert(#id, _mongo::bson::to_bson(&#member)?);
                }
            } else {
                quote! {
                    doc.insert(#id, _mongo::ext::bson::Bson::try_from(#member)?.0);
                }
            }
        });
        match v.style {
            Style::Struct => {
                quote! {
                    #name::#id { #(#fields),* } => {
                        doc.insert("_type", #value.to_owned());
                        #(#values)*
                    }
                }
            }
            Style::Tuple => {
                quote! {
                    #name::#id(#(#fields),*) => {
                        doc.insert("_type", #value.to_owned());
                        #(#values)*
                    }
                }
            }
            Style::Unit => quote! {
                #name::#id => {
                    doc.insert("_type", #value.to_owned());
                }
            },
        }
    });
    let try_from_bson_fields = variants.iter().map(|v| {
        let id = &v.ident;
        let value = to_snake_case(&v.ident.to_string());
        let options = v.fields.iter().map(|f| {
            let member = member_to_ident(&f.member);
            let ty = &f.ty;
            quote! {
                let mut #member: Option<#ty> = None;
            }
        });
        let values = v.fields.iter().map(|f| impl_struct_try_from_bson_field(f));
        let missing = v.fields.iter().map(|f| {
            let id = member_to_id(&f.member);
            let member = member_to_ident(&f.member);
            let msg = format!("'{}' is missing", id);
            quote! {
                if #member.is_none() {
                    return Err(_mongo::bson::de::Error::custom(
                        #msg.to_owned(),
                    ).into());
                }
            }
        });
        match v.style {
            Style::Struct => {
                let expects = v.fields.iter().map(|f| {
                    let id = member_to_id(&f.member);
                    let member = member_to_ident(&f.member);
                    let msg = format!("'{}' is missing", id);
                    quote! {
                        #member: #member.expect(#msg)
                    }
                });
                quote! {
                    Some(#value) => {
                        #(#options)*
                        #(#values)*
                        #(#missing)*
                        Ok(#name::#id {
                            #(#expects),*
                        })
                    }
                }
            }
            Style::Tuple => {
                let expects = v.fields.iter().map(|f| {
                    let id = member_to_id(&f.member);
                    let member = member_to_ident(&f.member);
                    let msg = format!("'{}' is missing", id);
                    quote! {
                            #member.expect(#msg)
                    }
                });
                quote! {
                    Some(#value) => {
                        #(#options)*
                        #(#values)*
                        #(#missing)*
                        Ok(#name::#id(
                            #(#expects),*
                        ))
                    }
                }
            }
            Style::Unit => quote! {
                Some(#value) => Ok(#name::#id),
            },
        }
    });
    let into = if attrs.into {
        let try_from_type = try_from_type_to_ext_bson(name);
        quote! {
            #[automatically_derived]
            impl TryFrom<#name> for _mongo::bson::Bson {
                type Error = _mongo::ext::bson::ser::Error;
                fn try_from(value: #name) -> core::result::Result<Self, Self::Error> {
                    let mut doc = _mongo::bson::Document::new();
                    match value {
                        #(#try_from_collection_fields),*
                    }
                    Ok(_mongo::bson::Bson::Document(doc))
                }
            }
            #try_from_type
        }
    } else {
        quote! {}
    };
    let from = if attrs.from {
        let try_from_ext = try_from_ext_bson_to_type(name);
        quote! {
            #[automatically_derived]
            impl TryFrom<_mongo::bson::Bson> for #name {
                type Error = _mongo::ext::bson::de::Error;
                fn try_from(bson: _mongo::bson::Bson) -> core::result::Result<Self, Self::Error> {
                    let mut doc = match bson {
                        _mongo::bson::Bson::Document(doc) => doc,
                        _ => return Err(_mongo::bson::de::Error::custom(
                            "not a BSON Document".to_owned()
                        ).into()),
                    };
                    let value = match doc.remove("_type") {
                        Some(v) => v,
                        None => return Err(_mongo::bson::de::Error::custom(
                            "enum type not found".to_owned()
                        ).into()),
                    };
                    match value.as_str() {
                        #(#try_from_bson_fields)*
                        _ => return Err(_mongo::bson::de::Error::custom(
                            "invalid variant".to_owned()
                        ).into()),
                    }
                }
            }
            #try_from_ext
        }
    } else {
        quote! {}
    };
    quote! {
        #into

        #from
    }
}

fn impl_enum_unit(
    name: &Ident,
    variants: &[Variant],
    attrs: &attr::Container,
) -> proc_macro2::TokenStream {
    let try_from_collection_fields = variants.iter().map(|v| {
        let id = &v.ident;
        let value = to_snake_case(&v.ident.to_string());
        quote! {
            #name::#id => #value.to_owned()
        }
    });

    let try_from_bson_fields = variants.iter().map(|v| {
        let id = &v.ident;
        let value = to_snake_case(&v.ident.to_string());
        quote! {
            #value => Ok(#name::#id),
        }
    });

    let into = if attrs.into {
        let try_from_type = try_from_type_to_ext_bson(name);
        quote! {
            #[automatically_derived]
            impl TryFrom<#name> for _mongo::bson::Bson {
                type Error = _mongo::ext::bson::ser::Error;
                fn try_from(value: #name) -> core::result::Result<Self, Self::Error> {
                    let v = match value {
                        #(#try_from_collection_fields),*
                    };
                    Ok(_mongo::bson::Bson::String(v))
                }
            }
            #try_from_type
        }
    } else {
        quote! {}
    };

    let from = if attrs.from {
        let try_from_ext = try_from_ext_bson_to_type(name);
        quote! {
            #[automatically_derived]
            impl TryFrom<_mongo::bson::Bson> for #name {
                type Error = _mongo::ext::bson::de::Error;
                fn try_from(bson: _mongo::bson::Bson) -> core::result::Result<Self, Self::Error> {
                    let value = match bson {
                        _mongo::bson::Bson::String(s) => s,
                        _ => return Err(_mongo::bson::de::Error::custom(
                            "not a BSON String".to_owned()
                        ).into()),
                    };
                    match value.as_str() {
                        #(#try_from_bson_fields)*
                        _ => return Err(_mongo::bson::de::Error::custom(
                            "invalid variant".to_owned()
                        ).into()),
                    }
                }
            }
            #try_from_ext
        }
    } else {
        quote! {}
    };

    quote! {
        #into
        #from
    }
}

fn impl_struct(
    name: &Ident,
    _style: &Style,
    fields: &[Field],
    attrs: &attr::Container,
) -> TokenStream {
    let try_from_collection_fields = fields
        .iter()
        .map(|f| {
            let member = &f.member;
            let id = member_to_id(&f.member);
            if f.attrs.serde {
                quote! {
                    doc.insert(#id, _mongo::bson::to_bson(&value.#member)?);
                }
            } else {
                quote! {
                    doc.insert(#id, _mongo::ext::bson::Bson::try_from(value.#member)?.0);
                }
            }
        })
        .collect::<Vec<_>>();
    let into = if attrs.into {
        let try_from_type = try_from_type_to_ext_bson(name);
        quote! {
            #[automatically_derived]
            impl TryFrom<#name> for _mongo::bson::Bson {
                type Error = _mongo::ext::bson::ser::Error;
                fn try_from(value: #name) -> core::result::Result<Self, Self::Error> {
                    let mut doc = _mongo::bson::Document::new();
                    #(#try_from_collection_fields)*
                    Ok(_mongo::bson::Bson::Document(doc))
                }
            }
            #try_from_type
        }
    } else {
        quote! {}
    };

    let options = fields.iter().map(|f| {
        let member = &f.member;
        let ty = &f.ty;
        quote! {
            let mut #member: Option<#ty> = None;
        }
    });
    let values = fields.iter().map(|f| impl_struct_try_from_bson_field(f));
    let missing = fields.iter().map(|f| {
        let id = member_to_id(&f.member);
        let member = &f.member;
        let msg = format!("'{}' is missing", id);
        quote! {
            if #member.is_none() {
                return Err(_mongo::bson::de::Error::custom(
                    #msg.to_owned(),
                ).into());
            }
        }
    });
    let expects = fields.iter().map(|f| {
        let id = member_to_id(&f.member);
        let member = &f.member;
        let msg = format!("'{}' is missing", id);
        quote! {
            #member: #member.expect(#msg)
        }
    });

    let from = if attrs.from {
        let try_from_ext = try_from_ext_bson_to_type(name);
        quote! {
            #[automatically_derived]
            impl TryFrom<_mongo::bson::Bson> for #name {
                type Error = _mongo::ext::bson::de::Error;
                fn try_from(bson: _mongo::bson::Bson) -> core::result::Result<Self, Self::Error> {
                    let mut doc = match bson {
                        _mongo::bson::Bson::Document(doc) => doc,
                        _ => return Err(_mongo::bson::de::Error::custom(
                            "not a BSON Document".to_owned()
                        ).into()),
                    };
                    #(#options)*
                    #(#values)*
                    #(#missing)*
                    Ok(Self {
                        #(#expects),*
                    })
                }
            }
            #try_from_ext
        }
    } else {
        quote! {}
    };

    quote! {
        #into
        #from
    }
}

fn impl_struct_try_from_bson_field(f: &Field) -> TokenStream {
    let member = member_to_ident(&f.member);
    let id = member_to_id(&f.member);
    let optional = is_option(f.ty);
    let ty = &f.ty;
    if f.attrs.serde {
        quote! {
            if let Some(__value) = doc.remove(#id) {
                #member = Some(_mongo::bson::from_bson(__value)?);
            }
        }
    } else if optional {
        quote! {
            if let Some(__value) = doc.remove(#id) {
                let wrap = _mongo::ext::bson::Bson(__value);
                let opt = match Option::<_mongo::bson::Bson>::from(wrap) {
                    Some(v) => Some(_mongo::ext::bson::Bson(v).try_into()?),
                    None => None,
                };
                #member = Some(opt);
            }
        }
    } else {
        quote! {
            if let Some(__value) = doc.remove(#id) {
                let wrap = _mongo::ext::bson::Bson(__value);
                #member = Some(<#ty>::try_from(wrap)?);
            }
        }
    }
}

// FIXME: Crude attempt to handle Option<T> as blanket impls prevent us from being
// truly generic... yay!
fn is_option(ty: &Type) -> bool {
    if let Type::Path(path) = ty {
        if let Some(segment) = path.path.segments.iter().next() {
            if segment.ident == "Option" {
                return true;
            }
        }
    }
    false
}

pub fn member_to_id(member: &Member) -> String {
    match member {
        Member::Named(name) => to_snake_case(&name.to_string()),
        Member::Unnamed(idx) => idx.index.to_string(),
    }
}

fn member_to_ident(member: &Member) -> Ident {
    match member {
        Member::Named(name) => name.clone(),
        Member::Unnamed(idx) => {
            Ident::new(&format!("_{}", idx.index), proc_macro2::Span::call_site())
        }
    }
}

fn try_from_ext_bson_to_type(name: &Ident) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl TryFrom<_mongo::ext::bson::Bson> for #name {
            type Error = _mongo::ext::bson::de::Error;
            fn try_from(bson: _mongo::ext::bson::Bson) -> core::result::Result<Self, Self::Error> {
                Self::try_from(bson.0)
            }
        }
    }
}

fn try_from_type_to_ext_bson(name: &Ident) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl TryFrom<#name> for _mongo::ext::bson::Bson {
            type Error = _mongo::ext::bson::ser::Error;
            fn try_from(value: #name) -> core::result::Result<Self, Self::Error> {
                Ok(_mongo::ext::bson::Bson(_mongo::bson::Bson::try_from(value)?))
            }
        }
    }
}
