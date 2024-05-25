use quote::ToTokens;
use syn::punctuated::Punctuated;

pub const BSON: &str = "bson";
pub const COLLECTION: &str = "collection";
pub const FIELD: &str = "field";
pub const FILTER: &str = "filter";
pub const FROM: &str = "from";
pub const INTO: &str = "into";
pub const MONGO: &str = "mongo";
pub const OID: &str = "oid";
pub const SERDE: &str = "serde";
pub const SKIP: &str = "skip";
pub const UPDATE: &str = "update";

pub struct Container<'a> {
    pub ident: syn::Ident,

    pub attrs: attr::Container,
    pub data: Data<'a>,
    pub generics: &'a syn::Generics,

    pub raw: &'a syn::DeriveInput,
}

pub enum Data<'a> {
    Enum(Vec<Variant<'a>>),
    Struct(Style, Vec<Field<'a>>),
}

pub struct Variant<'a> {
    pub ident: syn::Ident,

    pub attrs: attr::Variant,
    pub fields: Vec<Field<'a>>,
    pub style: Style,

    pub raw: &'a syn::Variant,
}

pub struct Field<'a> {
    pub member: syn::Member,

    pub attrs: attr::Field,
    pub ty: &'a syn::Type,

    pub raw: &'a syn::Field,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Style {
    Struct,
    Tuple,
    Unit,
}

impl<'a> Container<'a> {
    pub fn from(input: &'a syn::DeriveInput) -> Result<Container<'a>, Vec<syn::Error>> {
        let mut errors: Vec<syn::Error> = vec![];
        let attrs = match attr::Container::from(input) {
            Ok(attrs) => Some(attrs),
            Err(errs) => {
                errors.extend(errs);
                None
            }
        };
        let data = match &input.data {
            syn::Data::Enum(data) => match enum_from(&data.variants) {
                Ok(v) => Some(Data::Enum(v)),
                Err(errs) => {
                    errors.extend(errs);
                    None
                }
            },
            syn::Data::Struct(data) => match struct_from(&data.fields) {
                Ok((s, f)) => Some(Data::Struct(s, f)),
                Err(errs) => {
                    errors.extend(errs);
                    None
                }
            },
            syn::Data::Union(_) => {
                errors.push(syn::Error::new_spanned(
                    input.into_token_stream(),
                    "Mongo does not support derive for unions",
                ));
                None
            }
        };

        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(Container {
            ident: input.ident.clone(),
            attrs: attrs.expect("could not get attributes"),
            data: data.expect("could not get data"),
            generics: &input.generics,
            raw: input,
        })
    }
}

fn enum_from(
    variants: &Punctuated<syn::Variant, Token![,]>,
) -> Result<Vec<Variant<'_>>, Vec<syn::Error>> {
    let mut errors: Vec<syn::Error> = vec![];
    let variants = variants
        .iter()
        .filter_map(|v| {
            let attrs = match attr::Variant::from(v) {
                Ok(attrs) => Some(attrs),
                Err(errs) => {
                    errors.extend(errs);
                    None
                }
            };
            let data = match struct_from(&v.fields) {
                Ok(data) => Some(data),
                Err(errs) => {
                    errors.extend(errs);
                    None
                }
            };
            if attrs.is_none() || data.is_none() {
                return None;
            }
            let (style, fields) = data.expect("could not get data");
            Some(Variant {
                ident: v.ident.clone(),
                attrs: attrs.expect("could not get attributes"),
                fields,
                style,
                raw: v,
            })
        })
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(variants)
}

fn fields_from(
    fields: &Punctuated<syn::Field, Token![,]>,
) -> Result<Vec<Field<'_>>, Vec<syn::Error>> {
    let mut errors: Vec<syn::Error> = vec![];
    let fields = fields
        .iter()
        .enumerate()
        .filter_map(|(i, f)| {
            let attrs = match attr::Field::from(i, f) {
                Ok(attrs) => Some(attrs),
                Err(errs) => {
                    errors.extend(errs);
                    None
                }
            };
            let member = match &f.ident {
                Some(ident) => syn::Member::Named(ident.clone()),
                None => syn::Member::Unnamed(i.into()),
            };
            attrs.as_ref()?;
            Some(Field {
                member,
                attrs: attrs.expect("could not get attributes"),
                ty: &f.ty,
                raw: f,
            })
        })
        .collect();
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(fields)
}

fn struct_from(fields: &syn::Fields) -> Result<(Style, Vec<Field<'_>>), Vec<syn::Error>> {
    Ok(match fields {
        syn::Fields::Named(fields) => {
            let fields = fields_from(&fields.named)?;
            (Style::Struct, fields)
        }
        syn::Fields::Unnamed(fields) => {
            let fields = fields_from(&fields.unnamed)?;
            (Style::Tuple, fields)
        }
        syn::Fields::Unit => (Style::Unit, vec![]),
    })
}

pub mod attr {
    use super::*;

    use syn::meta::ParseNestedMeta;

    #[derive(PartialEq)]
    pub enum BsonMode {
        Convert,
        Serde,
    }

    impl std::str::FromStr for BsonMode {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "convert" => Ok(Self::Convert),
                "serde" => Ok(Self::Serde),
                _ => Err(format!("unknown `BsonMode` variant {}", s)),
            }
        }
    }

    pub struct Container {
        pub bson: BsonMode,
        pub collection: Option<String>,
        pub field: bool,
        pub filter: bool,
        pub from: bool,
        pub into: bool,
        pub oid: bool,
        pub update: bool,
    }
    pub struct Field {
        pub serde: bool,
        pub skip: bool,
    }
    pub struct Variant {}

    impl Container {
        pub fn from(item: &syn::DeriveInput) -> Result<Self, Vec<syn::Error>> {
            let mut errors: Vec<syn::Error> = Vec::new();

            let mut bson = BsonMode::Convert;
            let mut collection = None;
            let mut field = false;
            let mut filter = false;
            let mut from = false;
            let mut into = false;
            let mut oid = false;
            let mut update = false;

            for attr in &item.attrs {
                if !attr.path().is_ident(BSON) {
                    continue;
                }

                if let syn::Meta::List(meta) = &attr.meta {
                    if meta.tokens.is_empty() {
                        continue;
                    }
                }

                if let Err(err) = attr.parse_nested_meta(|meta| {
                    // Parse `#[bson(from)]`
                    if meta.path.is_ident(FROM) {
                        from = true;
                    // Parse `#[bson(from)]`
                    } else if meta.path.is_ident(INTO) {
                        into = true;
                    } else {
                        let path = meta.path.to_token_stream().to_string().replace(' ', "");
                        return Err(syn::Error::new_spanned(
                            meta.path,
                            format!("unknown bson container attribute `{}`", path),
                        ));
                    }
                    Ok(())
                }) {
                    errors.push(err);
                }
            }

            for attr in &item.attrs {
                if !attr.path().is_ident(MONGO) {
                    continue;
                }

                if let syn::Meta::List(meta) = &attr.meta {
                    if meta.tokens.is_empty() {
                        continue;
                    }
                }

                if let Err(err) = attr.parse_nested_meta(|meta| {
                    // Parse `#[mongo(bson = "convert")]`
                    if meta.path.is_ident(BSON) {
                        match get_lit_str(BSON, &meta) {
                            Ok(s) => match str::parse::<BsonMode>(&s.value()) {
                                Ok(x) => bson = x,
                                Err(_) => errors.push(syn::Error::new_spanned(
                                    meta.path.clone(),
                                    format!(
                                        "unknown mongo container attribute value `{}`",
                                        s.value()
                                    ),
                                )),
                            },
                            Err(e) => errors.push(e),
                        }
                    // Parse `#[mongo(collection = "foo")]`
                    } else if meta.path.is_ident(COLLECTION) {
                        match get_lit_str(COLLECTION, &meta) {
                            Ok(s) => collection = Some(s.value()),
                            Err(e) => errors.push(e),
                        }
                    // Parse `#[mongo(field)]`
                    } else if meta.path.is_ident(FIELD) {
                        field = true;
                    // Parse `#[mongo(filter)]`
                    } else if meta.path.is_ident(FILTER) {
                        filter = true;
                        // Parse `#[mongo(oid)]`
                    } else if meta.path.is_ident(OID) {
                        oid = true;
                    // Parse `#[mongo(update)]`
                    } else if meta.path.is_ident(UPDATE) {
                        update = true;
                    } else {
                        let path = meta.path.to_token_stream().to_string().replace(' ', "");
                        return Err(syn::Error::new_spanned(
                            meta.path,
                            format!("unknown mongo container attribute `{}`", path),
                        ));
                    }
                    Ok(())
                }) {
                    errors.push(err);
                }
            }

            if !from && !into {
                from = true;
                into = true;
            }
            if !errors.is_empty() {
                return Err(errors);
            }
            Ok(Container {
                bson,
                collection,
                field,
                filter,
                from,
                into,
                oid,
                update,
            })
        }
    }

    impl Field {
        pub fn from(_index: usize, field: &syn::Field) -> Result<Self, Vec<syn::Error>> {
            let mut errors: Vec<syn::Error> = Vec::new();

            let mut serde = false;
            let mut skip = false;

            for attr in &field.attrs {
                if !attr.path().is_ident(BSON) {
                    continue;
                }

                if let syn::Meta::List(meta) = &attr.meta {
                    if meta.tokens.is_empty() {
                        continue;
                    }
                }

                if let Err(err) = attr.parse_nested_meta(|meta| {
                    // Parse `#[bson(serde)]`
                    if meta.path.is_ident(SERDE) {
                        serde = true;
                    } else {
                        let path = meta.path.to_token_stream().to_string().replace(' ', "");
                        return Err(syn::Error::new_spanned(
                            meta.path,
                            format!("unknown bson field attribute `{}`", path),
                        ));
                    }

                    Ok(())
                }) {
                    errors.push(err);
                }
            }

            for attr in &field.attrs {
                if !attr.path().is_ident(MONGO) {
                    continue;
                }

                if let syn::Meta::List(meta) = &attr.meta {
                    if meta.tokens.is_empty() {
                        continue;
                    }
                }

                if let Err(err) = attr.parse_nested_meta(|meta| {
                    // Parse `#[mongo(serde)]`
                    if meta.path.is_ident(SERDE) {
                        serde = true;
                    // Parse `#[mongo(skip)]`
                    } else if meta.path.is_ident(SKIP) {
                        skip = true;
                    } else {
                        let path = meta.path.to_token_stream().to_string().replace(' ', "");
                        return Err(syn::Error::new_spanned(
                            meta.path,
                            format!("unknown mongo field attribute `{}`", path),
                        ));
                    }

                    Ok(())
                }) {
                    errors.push(err);
                }
            }

            if !errors.is_empty() {
                return Err(errors);
            }
            Ok(Field { serde, skip })
        }
    }

    impl Variant {
        pub fn from(_variant: &syn::Variant) -> Result<Self, Vec<syn::Error>> {
            Ok(Variant {})
        }
    }

    pub fn get_lit_str(
        attr_name: &'static str,
        meta: &ParseNestedMeta,
    ) -> Result<syn::LitStr, syn::Error> {
        let expr: syn::Expr = meta.value()?.parse()?;
        let mut value = &expr;
        while let syn::Expr::Group(e) = value {
            value = &e.expr;
        }
        if let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) = value
        {
            let suffix = lit.suffix();
            if !suffix.is_empty() {
                return Err(syn::Error::new_spanned(
                    lit,
                    format!("unexpected suffix `{}` on string literal", suffix),
                ));
            }
            Ok(lit.clone())
        } else {
            Err(syn::Error::new_spanned(
                expr,
                format!(
                    "expected mongo {} attribute to be a string: `{} = \"...\"`",
                    attr_name, attr_name
                ),
            ))
        }
    }
}
