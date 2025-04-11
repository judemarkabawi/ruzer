use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{braced, parse::Parse, parse_macro_input, Ident, LitInt, Token};

#[derive(Debug)]
struct DeviceImpl {
    name: Ident,
    product_id: u16,
    functions: Vec<FunctionMapping>,
}

#[derive(Debug)]
struct FunctionMapping {
    feature: Ident,
    impl_fn: Ident,
}

impl Parse for DeviceImpl {
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

        Ok(DeviceImpl {
            name,
            product_id,
            functions,
        })
    }
}

#[proc_macro]
pub fn device_impl(input: TokenStream) -> TokenStream {
    let DeviceImpl {
        name,
        product_id,
        functions,
    } = parse_macro_input!(input as DeviceImpl);
    let struct_name_caps = Ident::new(&name.to_string(), name.span());
    let struct_name = Ident::new(&name.to_string().to_case(Case::Pascal), name.span());

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
            return err.to_compile_error().into();
        }
    };

    quote! {
        pub(crate) const #struct_name_caps: u16 = #product_id;
        struct #struct_name(Interface);
        #[async_trait]
        impl FeatureSet for #struct_name {
            #(#fn_impls)*
        }
    }
    .into()
}
