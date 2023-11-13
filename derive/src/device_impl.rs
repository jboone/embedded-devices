//! This crate provides procedural helper macro for embedded-devices

use darling::ast::NestedMeta;
use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;

#[derive(Debug, FromMeta)]
struct DeviceImplArgs {}

/// This adds accessor functions for the given register and
/// proxies the embedded_registers::register attribute.
pub(crate) fn device_impl(args: TokenStream, orig_input: TokenStream) -> syn::Result<TokenStream> {
    let args_span = args.span();
    let _args = DeviceImplArgs::from_list(&NestedMeta::parse_meta_list(args)?)?;
    let mut item_impl = syn::parse2::<syn::ItemImpl>(orig_input.clone())?;

    let syn::Type::Path(self_ty_path) = item_impl.self_ty.as_ref() else {
        return Err(syn::Error::new(
            args_span,
            "Could not parse device identifier as path. This should not happen, please report this as a bug.",
        ));
    };
    let mut register_marker = self_ty_path.clone();
    let Some(last_segment) = register_marker.path.segments.last_mut() else {
        return Err(syn::Error::new(
            args_span,
            "Could not parse device identifier as path. This should not happen, please report this as a bug.",
        ));
    };
    last_segment.ident = format_ident!("{}Register", last_segment.ident);
    last_segment.arguments = syn::PathArguments::None;

    let read_register_doc = format!("Reads from the given register. For a list of all available registers, refer to implentors of [`{}`].", (&register_marker).to_token_stream());
    let write_register_doc = format!("Writes to the given register. For a list of all available registers, refer to implentors of [`{}`].", (&register_marker).to_token_stream());

    let additional_items = vec![
        quote! {
            #[doc = #read_register_doc]
            #[inline]
            pub async fn read_register<R>(&mut self) -> Result<R, I::Error>
            where
                R: embedded_registers::ReadableRegister + #register_marker
            {
                self.interface.read_register::<R>().await
            }
        },
        quote! {
            #[doc = #write_register_doc]
            #[inline]
            pub async fn write_register<R>(&mut self, register: &R) -> Result<(), I::Error>
            where
                R: embedded_registers::WritableRegister + #register_marker
            {
                self.interface.write_register::<R>(register).await
            }
        },
    ];

    for i in additional_items {
        item_impl.items.push(syn::parse2::<syn::ImplItem>(i)?);
    }

    let output = quote! {
        #[maybe_async_cfg::maybe(
            idents(hal(sync = "embedded_hal", async = "embedded_hal_async")),
            sync(not(feature = "async")),
            async(feature = "async"),
            keep_self
        )]
        #item_impl
    };

    Ok(output)
}
