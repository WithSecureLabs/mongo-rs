use quote::ToTokens;
use syn::punctuated::Punctuated;

pub const BSON: &str = "bson";
pub const COLLECTION: &str = "collection";
pub const FIELD: &str = "field";
pub const FILTER: &str = "filter";
pub const FROM: &str = "from";
pub const INTO: &str = "into";
pub const MONGO: &str = "mongo";
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

    use syn::Meta::{List, NameValue, Path};
    use syn::NestedMeta::{Lit, Meta};

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
            let mut update = false;

            for meta in item
                .attrs
                .iter()
                .flat_map(|attr| get_bson_meta_items(attr))
                .flatten()
            {
                match &meta {
                    // Parse `#[bson(from)]`
                    Meta(Path(word)) if word.is_ident(FROM) => {
                        from = true;
                    }
                    // Parse `#[bson(into)]`
                    Meta(Path(word)) if word.is_ident(INTO) => {
                        into = true;
                    }

                    Meta(item) => {
                        let path = item.path().into_token_stream().to_string().replace(' ', "");
                        errors.push(syn::Error::new_spanned(
                            item.path(),
                            format!("unknown bson container attribute `{}`", path),
                        ));
                    }

                    Lit(lit) => {
                        errors.push(syn::Error::new_spanned(
                            lit,
                            "unexpected literal in bson container attribute",
                        ));
                    }
                }
            }

            for meta in item
                .attrs
                .iter()
                .flat_map(|attr| get_mongo_meta_items(attr))
                .flatten()
            {
                match &meta {
                    // Parse `#[mongo(bson = "convert")]`
                    Meta(NameValue(m)) if m.path.is_ident(BSON) => {
                        match get_lit_str(BSON, &m.lit) {
                            Ok(s) => match str::parse::<BsonMode>(&s.value()) {
                                Ok(x) => bson = x,
                                Err(_) => errors.push(syn::Error::new_spanned(
                                    m.path.clone(),
                                    format!(
                                        "unknown mongo container attribute value `{}`",
                                        s.value()
                                    ),
                                )),
                            },
                            Err(e) => errors.push(e),
                        }
                    }
                    // Parse `#[mongo(collection = "foo")]`
                    Meta(NameValue(m)) if m.path.is_ident(COLLECTION) => {
                        match get_lit_str(COLLECTION, &m.lit) {
                            Ok(s) => collection = Some(s.value()),
                            Err(e) => errors.push(e),
                        }
                    }
                    // Parse `#[mongo(field)]`
                    Meta(Path(word)) if word.is_ident(FIELD) => {
                        field = true;
                    }
                    // Parse `#[mongo(filter)]`
                    Meta(Path(word)) if word.is_ident(FILTER) => {
                        filter = true;
                    }
                    // Parse `#[mongo(update)]`
                    Meta(Path(word)) if word.is_ident(UPDATE) => {
                        update = true;
                    }

                    Meta(item) => {
                        let path = item.path().into_token_stream().to_string().replace(' ', "");
                        errors.push(syn::Error::new_spanned(
                            item.path(),
                            format!("unknown mongo container attribute `{}`", path),
                        ));
                    }

                    Lit(lit) => {
                        errors.push(syn::Error::new_spanned(
                            lit,
                            "unexpected literal in mongo container attribute",
                        ));
                    }
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
                update,
            })
        }
    }

    impl Field {
        pub fn from(_index: usize, field: &syn::Field) -> Result<Self, Vec<syn::Error>> {
            let mut errors: Vec<syn::Error> = Vec::new();

            let mut serde = false;
            let mut skip = false;

            for meta in field
                .attrs
                .iter()
                .flat_map(|attr| get_bson_meta_items(attr))
                .flatten()
            {
                match &meta {
                    // Parse `#[bson(serde)]`
                    Meta(Path(word)) if word.is_ident(SERDE) => {
                        serde = true;
                    }

                    Meta(item) => {
                        let path = item.path().into_token_stream().to_string().replace(' ', "");
                        errors.push(syn::Error::new_spanned(
                            item.path(),
                            format!("unknown bson field attribute `{}`", path),
                        ));
                    }

                    Lit(lit) => {
                        errors.push(syn::Error::new_spanned(
                            lit,
                            "unexpected literal in bson field attribute",
                        ));
                    }
                }
            }

            for meta in field
                .attrs
                .iter()
                .flat_map(|attr| get_mongo_meta_items(attr))
                .flatten()
            {
                match &meta {
                    // Parse `#[mongo(serde)]`
                    Meta(Path(word)) if word.is_ident(SERDE) => {
                        serde = true;
                    }
                    // Parse `#[mongo(skip)]`
                    Meta(Path(word)) if word.is_ident(SKIP) => {
                        skip = true;
                    }

                    Meta(item) => {
                        let path = item.path().into_token_stream().to_string().replace(' ', "");
                        errors.push(syn::Error::new_spanned(
                            item.path(),
                            format!("unknown mongo field attribute `{}`", path),
                        ));
                    }

                    Lit(lit) => {
                        errors.push(syn::Error::new_spanned(
                            lit,
                            "unexpected literal in mongo field attribute",
                        ));
                    }
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

    pub fn get_lit_str<'a>(
        attr_name: &'static str,
        lit: &'a syn::Lit,
    ) -> Result<&'a syn::LitStr, syn::Error> {
        if let syn::Lit::Str(lit) = lit {
            Ok(lit)
        } else {
            Err(syn::Error::new_spanned(
                lit,
                format!(
                    "expected mongo {} attribute to be a string: `{} = \"...\"`",
                    attr_name, attr_name
                ),
            ))
        }
    }

    pub fn get_bson_meta_items(
        attr: &syn::Attribute,
    ) -> Result<Vec<syn::NestedMeta>, Vec<syn::Error>> {
        if !attr.path.is_ident(BSON) {
            return Ok(Vec::new());
        }
        match attr.parse_meta() {
            Ok(List(meta)) => Ok(meta.nested.into_iter().collect()),
            Ok(other) => Err(vec![syn::Error::new_spanned(
                other.into_token_stream(),
                "expected #[bson(...)]",
            )]),
            Err(err) => Err(vec![err]),
        }
    }

    pub fn get_mongo_meta_items(
        attr: &syn::Attribute,
    ) -> Result<Vec<syn::NestedMeta>, Vec<syn::Error>> {
        if !attr.path.is_ident(MONGO) {
            return Ok(Vec::new());
        }
        match attr.parse_meta() {
            Ok(List(meta)) => Ok(meta.nested.into_iter().collect()),
            Ok(other) => Err(vec![syn::Error::new_spanned(
                other.into_token_stream(),
                "expected #[mongo(...)]",
            )]),
            Err(err) => Err(vec![err]),
        }
    }
}
