use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro_crate::FoundCrate;
use syn::{Attribute, Data, DeriveInput, Meta};

fn find_serde_crate() -> proc_macro2::TokenStream {
    match proc_macro_crate::crate_name("serde") {
        Ok(FoundCrate::Itself) => quote::quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(name.as_str(), Span::call_site());
            quote::quote!(::#ident)
        }
        Err(_) => {
            panic!("serde is a co-dependency of serde-split")
        }
    }
}

fn filter_attrs(attrs: &mut Vec<Attribute>, is_json: bool) {
    let replace = if is_json { "json" } else { "bin" };

    let mut current = 0;
    while current < attrs.len() {
        if attrs[current].path().is_ident(replace) {
            match &mut attrs[current].meta {
                Meta::Path(path) => *path = syn::parse_quote!(serde),
                Meta::List(list) => list.path = syn::parse_quote!(serde),
                Meta::NameValue(name_value) => name_value.path = syn::parse_quote!(serde),
            }
        } else if !attrs[current].path().is_ident("serde") {
            attrs.remove(current);
            continue;
        }

        current += 1;
    }
}

fn filter_data(input: &mut DeriveInput, is_json: bool) {
    filter_attrs(&mut input.attrs, is_json);

    match &mut input.data {
        Data::Struct(data) => {
            data.fields
                .iter_mut()
                .for_each(|field| filter_attrs(&mut field.attrs, is_json));
        }
        Data::Enum(data) => {
            data.variants.iter_mut().for_each(|variant| {
                filter_attrs(&mut variant.attrs, is_json);
                variant
                    .fields
                    .iter_mut()
                    .for_each(|field| filter_attrs(&mut field.attrs, is_json));
            });
        }
        Data::Union(data) => {
            data.fields.named.iter_mut().for_each(|field| {
                filter_attrs(&mut field.attrs, is_json);
            });
        }
    }
}

#[proc_macro_derive(Serialize, attributes(json, bin, serde))]
pub fn derive_serialize(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as syn::DeriveInput);

    let ident = input.ident.clone();

    let mut json = input.clone();
    let mut bin = input;

    filter_data(&mut json, true);
    filter_data(&mut bin, false);

    json.ident = quote::format_ident!("{}JsonImpl", ident);
    bin.ident = quote::format_ident!("{}BinaryImpl", ident);

    let json_ident = &json.ident;
    let bin_ident = &bin.ident;

    let ident_str = syn::LitStr::new(ident.to_string().as_str(), ident.span());

    let serde = find_serde_crate();

    quote::quote! {
        const _: () = {
            #[derive(#serde::Serialize)]
            #[serde(remote = #ident_str)]
            #json

            #[derive(#serde::Serialize)]
            #[serde(remote = #ident_str)]
            #bin

            impl #serde::Serialize for #ident {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where S: #serde::Serializer
                {
                    if serializer.is_human_readable() {
                        #json_ident::serialize(self, serializer)
                    } else {
                        #bin_ident::serialize(self, serializer)
                    }
                }
            }
        };
    }
    .into()
}

#[proc_macro_derive(Deserialize, attributes(json, bin, serde))]
pub fn derive_deserialize(tokens: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(tokens as syn::DeriveInput);

    let ident = input.ident.clone();

    let mut json = input.clone();
    let mut bin = input;

    filter_data(&mut json, true);
    filter_data(&mut bin, true);

    json.ident = quote::format_ident!("{}JsonImpl", ident);
    bin.ident = quote::format_ident!("{}BinaryImpl", ident);

    let json_ident = &json.ident;
    let bin_ident = &bin.ident;

    let ident_str = syn::LitStr::new(ident.to_string().as_str(), ident.span());

    let serde = find_serde_crate();

    quote::quote! {
        const _: () = {
            #[derive(#serde::Deserialize)]
            #[serde(remote = #ident_str)]
            #json

            #[derive(#serde::Deserialize)]
            #[serde(remote = #ident_str)]
            #bin

            impl<'de> #serde::Deserialize<'de> for #ident {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                    where D: #serde::Deserializer<'de>
                {
                    if deserializer.is_human_readable() {
                        #json_ident::deserialize(deserializer)
                    } else {
                        #bin_ident::deserialize(deserializer)
                    }
                }
            }
        };
    }
    .into()
}
