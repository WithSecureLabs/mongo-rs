use inflector::cases::pascalcase::to_pascal_case;
use inflector::cases::snakecase::to_snake_case;
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Ident, Member};

use crate::ast::{attr, Container, Data, Field, Style, BSON, MONGO};
use crate::bson::member_to_id;

pub fn expand_derive_mongo(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let container = Container::from(input)?;

    let body = match &container.data {
        Data::Struct(style, fields) => {
            impl_struct(&container.ident, style, fields, &container.attrs)
        }
        _ => {
            return Err(vec![syn::Error::new_spanned(
                input.into_token_stream(),
                "#[derive(Mongo)] can only be derived on structs",
            )])
        }
    };

    let namespace = Ident::new(
        &to_snake_case(&container.ident.to_string()),
        input.ident.span(),
    );

    let serde = if container.attrs.bson == attr::BsonMode::Serde {
        quote! {
            extern crate serde as _serde;
        }
    } else {
        quote! {}
    };

    // We dont use unnamed constants because we need to allow the namespace to be pollutable
    Ok(quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        mod #namespace {
            extern crate mongod as _mongo;
            #serde

            use std::convert::TryFrom;

            use super::*;

            #body
        };
    })
}

fn impl_struct(
    name: &Ident,
    _style: &Style,
    fields: &[Field],
    attrs: &attr::Container,
) -> proc_macro2::TokenStream {
    let collection = if let Some(col) = &attrs.collection {
        let from = if attrs.bson == attr::BsonMode::Serde {
            quote! {
                _mongo::bson::from_document(document).map_err(_mongo::Error::invalid_document)
            }
        } else {
            quote! {
                Self::try_from(_mongo::bson::Bson::Document(document)).map_err(_mongo::Error::invalid_document)
            }
        };
        let into = if attrs.bson == attr::BsonMode::Serde {
            quote! {
                let b = _mongo::bson::to_bson(&self).map_err(_mongo::Error::invalid_document)?;
            }
        } else {
            quote! {
                let b = _mongo::bson::Bson::try_from(self).map_err(_mongo::Error::invalid_document)?;
            }
        };
        quote! {
            #[automatically_derived]
            impl _mongo::Collection for #name {
                const COLLECTION: &'static str = #col;

                fn from_document(document: _mongo::bson::Document) -> core::result::Result<Self, _mongo::Error> {
                    #from
                }

                fn into_document(self) -> core::result::Result<_mongo::bson::Document, _mongo::Error> {
                    #into
                    match b {
                        _mongo::bson::Bson::Document(doc) => Ok(doc),
                        _ => Err(_mongo::Error::invalid_document("not a bson document")),
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let field = if attrs.field {
        let matches = fields.iter().map(|f| {
            let (id, name) = match &f.member {
                Member::Named(name) => (to_pascal_case(&name.to_string()), stringify!(name)),
                Member::Unnamed(idx) => (idx.index.to_string(), stringify!(idx)),
            };
            let id = Ident::new(&id, Span::call_site());
            quote! {
                Field::#id => #name.to_owned()
            }
        });
        let variants = fields.iter().map(|f| {
            let id = match &f.member {
                Member::Named(name) => to_pascal_case(&name.to_string()),
                Member::Unnamed(idx) => idx.index.to_string(),
            };
            let id = Ident::new(&id, Span::call_site());
            quote! {
                #id
            }
        });
        quote! {
            #[automatically_derived]
            pub enum Field {
                #(#variants),*
            }
            #[automatically_derived]
            impl _mongo::AsField<Field> for #name {}
            #[automatically_derived]
            impl _mongo::Field for Field {}
            #[automatically_derived]
            impl From<Field> for String {
                fn from(field: Field) -> String {
                    match field {
                        #(#matches),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let filter = if attrs.filter {
        let filter_fields = fields.iter().filter_map(|f| {
            if f.attrs.skip {
                return None;
            }
            let ty = &f.ty;
            let name = match &f.member {
                Member::Named(name) => name,
                _ => panic!("#[derive(Mongo)] can only be derived on named structs"),
            };
            let inner = if f.attrs.serde || attrs.bson == attr::BsonMode::Serde {
                quote! { _mongo::ext::bson::Ser<#ty> }
            } else {
                quote! { #ty }
            };
            Some(quote! {
                pub #name: Option<_mongo::Comparator<#inner>>
            })
        });
        let into_bson = fields.iter().filter_map(|f| {
            if f.attrs.skip {
                return None;
            }
            let member = &f.member;
            let id = member_to_id(&f.member);
            Some(quote! {
                if let Some(__value) = value.#member {
                    doc.insert(#id, _mongo::ext::bson::Bson::try_from(__value)?.0);
                }
            })
        });
        let into_filter = fields.iter().filter_map(|f| {
            if f.attrs.skip {
                return None;
            }
            let name = match &f.member {
                Member::Named(name) => name,
                _ => panic!("#[derive(Mongo)] can only be derived on named structs"),
            };
            let inner = if f.attrs.serde || attrs.bson == attr::BsonMode::Serde {
                quote! { _mongo::ext::bson::Ser(self.#name) }
            } else {
                quote! { self.#name }
            };
            Some(quote! {
                #name: Some(_mongo::Comparator::Eq(#inner))
            })
        });
        let filter_field_oid = if attrs.oid {
            quote! {
                pub _id: Option<_mongo::Comparator<_mongo::bson::oid::ObjectId>>,
            }
        } else {
            quote! {}
        };
        let into_bson_oid = if attrs.oid {
            quote! {
                if let Some(__value) = value._id {
                    doc.insert("_id", _mongo::ext::bson::Bson::try_from(__value)?.0);
                }
            }
        } else {
            quote! {}
        };
        let into_filter_oid = if attrs.oid {
            quote! {
                _id: None,
            }
        } else {
            quote! {}
        };
        quote! {
            #[automatically_derived]
            #[derive(Default)]
            pub struct Filter {
                #filter_field_oid
                #(#filter_fields),*
            }
            #[automatically_derived]
            impl TryFrom<Filter> for _mongo::bson::Bson {
                type Error = _mongo::ext::bson::ser::Error;
                fn try_from(value: Filter) -> core::result::Result<Self, Self::Error> {
                    let mut doc = _mongo::bson::Document::new();
                    #into_bson_oid
                    #(#into_bson)*
                    Ok(_mongo::bson::Bson::Document(doc))
                }
            }
            #[automatically_derived]
            impl TryFrom<Filter> for _mongo::ext::bson::Bson {
                type Error = _mongo::ext::bson::ser::Error;
                fn try_from(value: Filter) -> core::result::Result<Self, Self::Error> {
                    Ok(_mongo::ext::bson::Bson(_mongo::bson::Bson::try_from(value)?))
                }
            }
            #[automatically_derived]
            impl _mongo::Filter for Filter {
                fn new() -> Self {
                    Self::default()
                }
                fn into_document(self) -> core::result::Result<_mongo::bson::Document, _mongo::Error> {
                    let b = _mongo::bson::Bson::try_from(self).map_err(_mongo::Error::invalid_document)?;
                    match b {
                        _mongo::bson::Bson::Document(doc) => Ok(doc),
                        _ => Err(_mongo::Error::invalid_document("not a bson document")),
                    }
                }
            }
            #[automatically_derived]
            impl _mongo::AsFilter<Filter> for #name {
                fn filter() -> Filter {
                    Filter::default()
                }
                fn into_filter(self) -> Filter {
                    Filter {
                        #into_filter_oid
                        #(#into_filter),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    let update = if attrs.update {
        let (derive, bson, into) = if attrs.bson == attr::BsonMode::Serde {
            (
                quote! {
                    #[derive(Default, _serde::Serialize)]
                },
                quote! {},
                quote! {
                        let b = _mongo::bson::to_bson(&self).map_err(_mongo::Error::invalid_document)?;
                },
            )
        } else {
            let into_bson = fields.iter().filter_map(|f| {
                if f.attrs.skip {
                    return None;
                }
                let member = &f.member;
                let id = member_to_id(&f.member);
                if f.attrs.serde {
                    Some(quote! {
                    if let Some(__value) = value.#member {
                        doc.insert(#id, _mongo::bson::to_bson(&__value)?);
                    }
                    })
                } else {
                    Some(quote! {
                        if let Some(__value) = value.#member {
                            doc.insert(#id, _mongo::ext::bson::Bson::try_from(__value)?.0);
                        }
                    })
                }
            });
            (
                quote! {
                    #[derive(Default)]
                },
                quote! {
                    #[automatically_derived]
                    impl TryFrom<Update> for _mongo::bson::Bson {
                        type Error = _mongo::ext::bson::ser::Error;
                        fn try_from(value: Update) -> core::result::Result<Self, Self::Error> {
                            let mut doc = _mongo::bson::Document::new();
                            #(#into_bson)*
                            Ok(_mongo::bson::Bson::Document(doc))
                        }
                    }
                    #[automatically_derived]
                    impl TryFrom<Update> for _mongo::ext::bson::Bson {
                        type Error = _mongo::ext::bson::ser::Error;
                        fn try_from(value: Update) -> core::result::Result<Self, Self::Error> {
                            Ok(_mongo::ext::bson::Bson(_mongo::bson::Bson::try_from(value)?))
                        }
                    }
                },
                quote! {
                        let b = _mongo::bson::Bson::try_from(self).map_err(_mongo::Error::invalid_document)?;
                },
            )
        };
        let update_fields = fields.iter().filter_map(|f| {
            if f.attrs.skip {
                return None;
            }
            let ty = &f.ty;
            let name = match &f.member {
                Member::Named(name) => name,
                _ => panic!("#[derive(Mongo)] can only be derived on named structs"),
            };
            // Pass the attrs along so we can just derive the bson... but make sure that local
            // attrs are stripped!
            let raw_attrs = &f
                .raw
                .attrs
                .iter()
                .filter_map(|a| {
                    if a.path.is_ident(BSON) || a.path.is_ident(MONGO) {
                        None
                    } else {
                        Some(a)
                    }
                })
                .collect::<Vec<_>>();
            if attrs.bson == attr::BsonMode::Serde {
                Some(quote! {
                    #(#raw_attrs),*
                    #[serde(skip_serializing_if="Option::is_none")]
                    pub #name: Option<#ty>
                })
            } else {
                Some(quote! {
                    #(#raw_attrs),*
                    pub #name: Option<#ty>
                })
            }
        });
        let into_update = fields.iter().filter_map(|f| {
            if f.attrs.skip {
                return None;
            }
            let name = match &f.member {
                Member::Named(name) => name,
                _ => panic!("#[derive(Mongo)] can only be derived on named structs"),
            };
            Some(quote! {
                #name: Some(self.#name)
            })
        });
        quote! {
            #[automatically_derived]
            #derive
            pub struct Update {
                #(#update_fields),*
            }
            #bson
            #[automatically_derived]
            impl _mongo::Update for Update {
                fn new() -> Self {
                    Self::default()
                }
                fn into_document(self) -> core::result::Result<_mongo::bson::Document, _mongo::Error> {
                    #into
                    match b {
                        _mongo::bson::Bson::Document(doc) => Ok(doc),
                        _ => Err(_mongo::Error::invalid_document("not a bson document")),
                    }
                }
            }
            #[automatically_derived]
            impl _mongo::AsUpdate<Update> for #name {
                fn update() -> Update {
                    Update::default()
                }
                fn into_update(self) -> Update {
                    Update {
                        #(#into_update),*
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #collection
        #field
        #filter
        #update
    }
}
