use std::collections::HashSet;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    braced, bracketed, parse::Parse, parse_macro_input, punctuated::Punctuated, Ident, LitInt,
    Token,
};

struct DeviceDefs(Vec<SharedDeviceDef>);

/// Definitions for one or more devices, along with their shared function mappings.
struct SharedDeviceDef {
    device_ids: Vec<DeviceId>,
    def: DeviceDef,
}

struct SingleDeviceDef<'a> {
    device_id: &'a DeviceId,
    def: &'a DeviceDef,
}

/// Device name (ex: `DeathadderV2ProWireless`) and USB product id
struct DeviceId {
    name: Ident,
    product_id: u16,
}

struct DeviceDef {
    transaction_id: u8,
    functions: Vec<FunctionMapping>,
}

/// A mapping of a trait method in `FeatureSet` to a concrete implementation
struct FunctionMapping {
    feature: Ident,
    impl_fn: Ident,
}

impl Parse for DeviceDefs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);

        let device_defs = Punctuated::<SharedDeviceDef, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();
        Ok(DeviceDefs(device_defs))
    }
}

impl Parse for SharedDeviceDef {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Parse one or more `name product_id` listed
        let device_ids = Punctuated::<DeviceId, Token![|]>::parse_separated_nonempty(input)?
            .into_iter()
            .collect();

        // Then the rest is in braces
        let content;
        braced!(content in input);

        // Transaction ID first
        let transaction_id_ident = content.parse::<Ident>().map_err(|err| {
            syn::Error::new(err.span(), "Expected \"transaction_id = 0xXX\" first")
        })?;
        if transaction_id_ident != "transaction_id" {
            return Err(syn::Error::new(
                transaction_id_ident.span(),
                "Must write \"transaction_id\"",
            ));
        }
        content.parse::<Token![=]>()?;
        let transaction_id: u8 = content.parse::<LitInt>()?.base10_parse()?;
        content.parse::<Token![,]>()?;

        // Zero or more feature: impl_fn mappings in the rbaces
        let functions = Punctuated::<FunctionMapping, Token![,]>::parse_terminated(&content)?
            .into_iter()
            .collect();
        Ok(SharedDeviceDef {
            device_ids,
            def: DeviceDef {
                transaction_id,
                functions,
            },
        })
    }
}

impl SharedDeviceDef {
    /// This flattens a `SharedDeviceDef` into a list of `SingleDeviceDef`s that can be iterated over easier
    /// as if they were each just one device and its implementation.
    fn flatten_devices(&self) -> impl Iterator<Item = SingleDeviceDef<'_>> {
        self.device_ids.iter().map(|id| SingleDeviceDef {
            device_id: id,
            def: &self.def,
        })
    }
}

impl Parse for DeviceId {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?;
        let product_id: u16 = input.parse::<LitInt>()?.base10_parse()?;
        Ok(DeviceId { name, product_id })
    }
}

impl DeviceId {
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

impl Parse for FunctionMapping {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let feature = input.parse()?;

        if input.peek(Token![:]) {
            // Specified impl
            input.parse::<Token![:]>()?;
            let impl_fn = input.parse()?;
            Ok(FunctionMapping { feature, impl_fn })
        } else {
            // Default
            let impl_fn = feature.clone();
            Ok(FunctionMapping { feature, impl_fn })
        }
    }
}

/// For each device, this macro:
///     - Defines a custom struct for it
///     - Defines the `FeatureSet` trait on it, only implementing listed methods using their described impls
///     - Defines the product_id of that device
///     - Adds a match arm to the `get_device_impl`, which maps from its product_id to its custom struct
/// We then end up with implementations of subsets of `FeatureSet`'s methods for
/// each device, as well as a method `get_device_impl` to take a product_id and return a `Box<dyn FeatureSet>` or error.
///
/// Example use:
/// ```
/// device_impls!([
///     DeathadderV2ProWired    0x007C |
///     DeathadderV2ProWireless 0x007D
///     {
///         transaction_id = 0x3f,
///         get_dpi,
///         set_dpi,
///     },
///     ViperMini 0x008A {
///         transaction_id = 0xXX,
///         get_dpi_stages: get_dpi_stages_custom_impl,
///     }
/// ]);
/// ```
#[proc_macro]
pub fn device_impls(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeviceDefs);
    device_impls_inner(&input).into()
}

fn device_impls_inner(device_defs: &DeviceDefs) -> TokenStream2 {
    let device_defs: Vec<SingleDeviceDef> = device_defs
        .0
        .iter()
        .flat_map(|shared_def| shared_def.flatten_devices())
        .collect();

    let device_ids = device_defs.iter().map(|def| def.device_id);

    // Make sure all product IDs are unique. Duplicates are a guaranteed programmer bug
    if let Some(error) = find_first_duplicate(device_ids) {
        return error.into_compile_error();
    }

    let caps_names = device_defs.iter().map(|def| def.device_id.caps_name());
    let pascal_names = device_defs.iter().map(|def| def.device_id.pascal_name());
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

fn device_impl_inner(device_def: &SingleDeviceDef<'_>) -> TokenStream2 {
    let SingleDeviceDef { device_id, def } = device_def;
    let caps_name = device_id.caps_name();
    let pascal_name = device_id.pascal_name();
    let product_id = device_id.product_id;
    let transaction_id = def.transaction_id;

    let fn_impls: syn::Result<Vec<TokenStream2>> = def.functions.iter().map(|fn_map| {
            let FunctionMapping { feature, impl_fn } = fn_map;
            let feature_str = feature.to_string();
            match feature_str.as_str() {
                "get_dpi" => Ok(quote! {
                    async fn get_dpi(&self) -> Result<(u16, u16)> {
                        #impl_fn(self.0.clone(), #transaction_id, VarStoreId::NoStore).await
                    }
                }),
                "set_dpi" => Ok(quote! {
                    async fn set_dpi(&self, dpi: (u16, u16)) -> Result<()> {
                        #impl_fn(self.0.clone(), #transaction_id, VarStoreId::NoStore, dpi).await
                    }
                }),
                "get_dpi_stages" => Ok(quote! {
                    async fn get_dpi_stages(&self) -> Result<DpiStages> {
                        #impl_fn(self.0.clone(), #transaction_id).await
                    }
                }),
                "set_dpi_stages" => Ok(quote! {
                    async fn set_dpi_stages(&self, dpi_stages: &DpiStages) -> Result<()> {
                        #impl_fn(self.0.clone(), #transaction_id, dpi_stages).await
                    }
                }),
                "get_polling_rate" => Ok(quote! {
                    async fn get_polling_rate(&self) -> Result<u16> {
                        #impl_fn(self.0.clone(), #transaction_id).await
                    }
                }),
                "set_polling_rate" => Ok(quote! {
                    async fn set_polling_rate(&self, polling_rate: PollingRate) -> Result<()> {
                        #impl_fn(self.0.clone(), #transaction_id, polling_rate).await
                    }
                }),
                "get_battery_level" => Ok(quote! {
                    async fn get_battery_level(&self) -> Result<f32> {
                        #impl_fn(self.0.clone(), #transaction_id).await
                    }
                }),
                "get_charging_status" => Ok(quote! {
                    async fn get_charging_status(&self) -> Result<bool> {
                        #impl_fn(self.0.clone(), #transaction_id).await
                    }
                }),
                "chroma_logo_matrix_effect" => Ok(quote! {
                    async fn chroma_logo_matrix_effect(&self, effect: ExtendedMatrixEffect) -> Result<()> {
                        #impl_fn(self.0.clone(), #transaction_id, effect).await
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

/// Find duplicate product IDs in a list of device definitions for debugging.
fn find_first_duplicate<'a, T>(device_ids: T) -> Option<syn::Error>
where
    T: IntoIterator<Item = &'a DeviceId>,
{
    let mut set = HashSet::new();
    for device_id in device_ids {
        let product_id = device_id.product_id;

        if set.contains(&product_id) {
            return Some(syn::Error::new(
                device_id.name.span(),
                format!(
                    "{:#06x} is a duplicate USB product id.",
                    device_id.product_id
                ),
            ));
        }

        set.insert(product_id);
    }
    None
}
