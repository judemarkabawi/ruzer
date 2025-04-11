use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{braced, bracketed, parse::Parse, parse_macro_input, Ident, LitInt, Token};

struct DeviceDefs(Vec<DeviceDef>);

impl Parse for DeviceDefs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);

        let mut device_defs = Vec::new();
        while !content.is_empty() {
            let device_impl: DeviceDef = content.parse()?;
            device_defs.push(device_impl);

            content.parse::<Token![,]>()?;
        }

        Ok(DeviceDefs(device_defs))
    }
}
struct DeviceDef {
    name: Ident,
    product_id: u16,
    functions: Vec<FunctionMapping>,
}

struct FunctionMapping {
    feature: Ident,
    impl_fn: Ident,
}

impl Parse for DeviceDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?;
        let product_id: u16 = input.parse::<LitInt>()?.base10_parse()?;

        let content;
        braced!(content in input);

        let mut functions = Vec::new();
        while !content.is_empty() {
            let feature = content.parse()?;
            content.parse::<Token![:]>()?;
            let impl_fn = content.parse()?;
            functions.push(FunctionMapping { feature, impl_fn });

            // Even last line should end in a comma for better git diffs
            content.parse::<Token![,]>()?;
        }

        Ok(DeviceDef {
            name,
            product_id,
            functions,
        })
    }
}

impl DeviceDef {
    fn pascal_name(&self) -> Ident {
        Ident::new(
            &self.name.to_string().to_case(Case::Pascal),
            self.name.span(),
        )
    }

    fn caps_name(&self) -> Ident {
        Ident::new(
            &self.name.to_string().to_case(Case::Constant),
            self.name.span(),
        )
    }
}

#[proc_macro]
pub fn device_impls(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeviceDefs);
    device_impls_inner(&input).into()
}

fn device_impls_inner(device_defs: &DeviceDefs) -> TokenStream2 {
    let DeviceDefs(device_defs) = device_defs;

    // Make sure all product IDs are unique. Duplicates are a guaranteed programmer bug
    if let Some(error) = find_first_duplicate(device_defs) {
        return error.into_compile_error();
    }

    let caps_names = device_defs.iter().map(|device_def| device_def.caps_name());
    let pascal_names = device_defs
        .iter()
        .map(|device_def| device_def.pascal_name());
    let device_impls = device_defs.iter().map(device_impl_inner);

    quote! {
        fn get_device_impl(product_id: u16, interface: Interface) -> Result<Box<dyn FeatureSet>> {
            match product_id {
                #(
                id if id == #caps_names => Ok(Box::new(#pascal_names(interface))),
                )*
                _ => Err(anyhow!("Unsupported device")),
            }
        }

        #(
            #device_impls
        )*
    }
}

fn device_impl_inner(device_def: &DeviceDef) -> TokenStream2 {
    let DeviceDef {
        name: _,
        product_id,
        functions,
    } = device_def;
    let caps_name = device_def.caps_name();
    let pascal_name = device_def.pascal_name();

    let fn_impls: syn::Result<Vec<TokenStream2>> = functions.iter().map(|fn_map| {
            let FunctionMapping { feature, impl_fn } = fn_map;
            let feature_str = feature.to_string();
            match feature_str.as_str() {
                "get_dpi" => Ok(quote! {
                    async fn get_dpi(&self) -> Result<(u16, u16)> {
                        #impl_fn(self.0.clone()).await
                    }
                }),
                "set_dpi" => Ok(quote! {
                    async fn set_dpi(&self, dpi: (u16, u16)) -> Result<()> {
                        #impl_fn(self.0.clone(), dpi).await
                    }
                }),
                "get_dpi_stages" => Ok(quote! {
                    async fn get_dpi_stages(&self) -> Result<DpiStages> {
                        #impl_fn(self.0.clone()).await
                    }
                }),
                "set_dpi_stages" => Ok(quote! {
                    async fn set_dpi_stages(&self, dpi_stages: &DpiStages) -> Result<()> {
                        #impl_fn(self.0.clone(), dpi_stages).await
                    }
                }),
                "get_polling_rate" => Ok(quote! {
                    async fn get_polling_rate(&self) -> Result<u16> {
                        #impl_fn(self.0.clone()).await
                    }
                }),
                "get_battery_level" => Ok(quote! {
                    async fn get_battery_level(&self) -> Result<f32> {
                        #impl_fn(self.0.clone()).await
                    }
                }),
                "get_charging_status" => Ok(quote! {
                    async fn get_charging_status(&self) -> Result<bool> {
                        #impl_fn(self.0.clone()).await
                    }
                }),
                "chroma_logo_matrix_effect" => Ok(quote! {
                    async fn chroma_logo_matrix_effect(&self, effect: ExtendedMatrixEffect) -> Result<()> {
                        #impl_fn(self.0.clone(), effect).await
                    }
                }),
                _ => {
                    Err(syn::Error::new(feature.span(), format!("Invalid feature: {}", feature_str)))
                },
            }
        }).collect();

    let fn_impls = match fn_impls {
        Ok(fn_impls) => fn_impls,
        Err(err) => {
            return err.to_compile_error();
        }
    };

    quote! {
        pub(crate) const #caps_name: u16 = #product_id;
        struct #pascal_name(Interface);
        #[async_trait]
        impl FeatureSet for #pascal_name {
            #(#fn_impls)*
        }
    }
}

fn find_first_duplicate<'a, T>(device_defs: T) -> Option<syn::Error>
where
    T: IntoIterator<Item = &'a DeviceDef>,
{
    let mut set = HashSet::new();
    for device_def in device_defs {
        let product_id = device_def.product_id;

        if set.contains(&product_id) {
            return Some(syn::Error::new(
                device_def.name.span(),
                format!(
                    "{:#06x} is a duplicate USB product id.",
                    device_def.product_id
                ),
            ));
        }

        set.insert(product_id);
    }
    None
}
