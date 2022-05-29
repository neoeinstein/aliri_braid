use quote::{quote, ToTokens};
use super::{OwnedCodeGen, RefCodeGen, check_mode::CheckMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImplOption {
    Implement,
    Omit,
}

impl ImplOption {
    fn map<F>(self, f: F) -> Option<proc_macro2::TokenStream>
    where F: FnOnce() -> proc_macro2::TokenStream {
        match self {
            Self::Implement => Some(f()),
            Self::Omit => None,
        }
    }
}

impl std::str::FromStr for ImplOption {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "impl" => Ok(Self::Implement),
            "none" => Ok(Self::Omit),
            _ => Err("valid values are: `impl` or `omit`"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelegatingImplOption {
    Implement,
    OwnedOnly,
    Omit,
}

impl DelegatingImplOption {
    fn map_owned<F>(self, f: F) -> Option<proc_macro2::TokenStream>
    where F: FnOnce() -> proc_macro2::TokenStream {
        match self {
            Self::Implement | Self::OwnedOnly => Some(f()),
            Self::Omit => None,
        }
    }

    fn map_ref<F>(self, f: F) -> Option<proc_macro2::TokenStream>
    where F: FnOnce() -> proc_macro2::TokenStream {
        match self {
            Self::Implement=> Some(f()),
            Self::Omit | Self::OwnedOnly => None,
        }
    }
}

impl std::str::FromStr for DelegatingImplOption {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "impl" => Ok(Self::Implement),
            "owned" => Ok(Self::OwnedOnly),
            "none" => Ok(Self::Omit),
            _ => Err("valid values are: `impl`, `owned`, or `omit`"),
        }
    }
}

#[derive(Debug, Default)]
pub struct Impls {
    pub clone: ImplClone,
    pub debug: ImplDebug,
    pub display: ImplDisplay,
    pub serde: ImplSerde,
}

pub(crate) trait ToImpl {
    fn to_owned_impl(&self, _gen: &OwnedCodeGen) -> Option<proc_macro2::TokenStream> {
        None
    }

    fn to_borrowed_impl(&self, _gen: &RefCodeGen) -> Option<proc_macro2::TokenStream> {
        None
    }
}

#[derive(Debug)]
pub struct ImplClone(ImplOption);

impl Default for ImplClone {
    fn default() -> Self {
        Self(ImplOption::Implement)
    }
}

impl From<ImplOption> for ImplClone {
    fn from(opt: ImplOption) -> Self {
        Self(opt)
    }
}

impl ToImpl for ImplClone {
    fn to_owned_impl(&self, _gen: &OwnedCodeGen) -> Option<proc_macro2::TokenStream> {
        self.0.map(|| quote! { #[derive(Clone)] })
    }
}

#[derive(Debug)]
pub struct ImplDisplay(DelegatingImplOption);

impl Default for ImplDisplay {
    fn default() -> Self {
        Self(DelegatingImplOption::Implement)
    }
}

impl From<DelegatingImplOption> for ImplDisplay {
    fn from(opt: DelegatingImplOption) -> Self {
        Self(opt)
    }
}
impl ToImpl for ImplDisplay {
    fn to_owned_impl(&self, gen: &OwnedCodeGen) -> Option<proc_macro2::TokenStream> {
        let ty = gen.ty;
        let ref_ty = gen.ref_ty;
        self.0.map_owned(|| quote! {
            impl<'a> ::std::fmt::Display for #ty {
                #[inline]
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    <#ref_ty as ::std::fmt::Display>::fmt(::std::ops::Deref::deref(self), f)
                }
            }
        })
    }

    fn to_borrowed_impl(&self, gen: &RefCodeGen) -> Option<proc_macro2::TokenStream> {
        let ty = &gen.ty;
        let field_name = gen.field.name;
        self.0.map_ref(|| quote! {
            impl ::std::fmt::Display for #ty {
                #[inline]
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    <str as ::std::fmt::Display>::fmt(&self.#field_name, f)
                }
            }
        })
    }
}

#[derive(Debug)]
pub struct ImplDebug(DelegatingImplOption);

impl Default for ImplDebug {
    fn default() -> Self {
        Self(DelegatingImplOption::Implement)
    }
}

impl From<DelegatingImplOption> for ImplDebug {
    fn from(opt: DelegatingImplOption) -> Self {
        Self(opt)
    }
}

impl ToImpl for ImplDebug {
    fn to_owned_impl(&self, gen: &OwnedCodeGen) -> Option<proc_macro2::TokenStream> {
        let ty = gen.ty;
        let ref_ty = gen.ref_ty;
        self.0.map_owned(|| quote! {
            impl<'a> ::std::fmt::Debug for #ty {
                #[inline]
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    <#ref_ty as ::std::fmt::Debug>::fmt(::std::ops::Deref::deref(self), f)
                }
            }
        })
    }

    fn to_borrowed_impl(&self, gen: &RefCodeGen) -> Option<proc_macro2::TokenStream> {
        let ty = &gen.ty;
        let field_name = gen.field.name;
        self.0.map_ref(|| quote! {
            impl ::std::fmt::Debug for #ty {
                #[inline]
                fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                    <str as ::std::fmt::Debug>::fmt(&self.#field_name, f)
                }
            }
        })
    }
}

#[derive(Debug)]
pub struct ImplSerde(ImplOption);

impl Default for ImplSerde {
    fn default() -> Self {
        Self(ImplOption::Omit)
    }
}

impl From<ImplOption> for ImplSerde {
    fn from(opt: ImplOption) -> Self {
        Self(opt)
    }
}

impl ToImpl for ImplSerde {
    fn to_owned_impl(&self, gen: &OwnedCodeGen) -> Option<proc_macro2::TokenStream> {
        self.0.map(|| {
            let handle_failure = gen.check_mode.serde_err_handler();

            let name = gen.ty;
            let field_name = gen.field.name;
            let wrapped_type = &gen.field.ty;

            quote! {
                impl ::serde::Serialize for #name {
                    fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        <#wrapped_type as ::serde::Serialize>::serialize(&self.#field_name, serializer)
                    }
                }

                #[allow(clippy::needless_question_mark)]
                impl<'de> ::serde::Deserialize<'de> for #name {
                    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                        let raw = <#wrapped_type as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                        Ok(Self::new(raw)#handle_failure)
                    }
                }
            }
        })
    }

    fn to_borrowed_impl(&self, gen: &RefCodeGen) -> Option<proc_macro2::TokenStream> {
        self.0.map(|| {
            let ty = &gen.ty;
            let owned_ty = gen.owned_ty;
            let check_mode = gen.check_mode;

            let handle_failure = check_mode.serde_err_handler();

            let deserialize_boxed = quote! {
                impl<'de> ::serde::Deserialize<'de> for Box<#ty> {
                    fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                        let owned = <#owned_ty as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                        Ok(owned.into_boxed_ref())
                    }
                }
            };

            let deserialize = if matches!(check_mode, CheckMode::Normalize(_)) {
                let deserialize_doc = format!(
                    "Deserializes a `{ty}` in normalized form\n\
                    \n\
                    This deserializer _requires_ that the value already be in normalized form. \
                    If values may require normalization, then deserialized as [`{owned}`] or \
                    [`Cow`][std::borrow::Cow]`<{ty}>` instead.",
                    ty = ty.to_token_stream(),
                    owned = owned_ty.to_token_stream(),
                );

                quote! {
                    // impl<'de: 'a, 'a> ::serde::Deserialize<'de> for ::std::borrow::Cow<'a, #name> {
                    //     fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                    //         let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                    //         Ok(#name::from_str(raw)#handle_failure)
                    //     }
                    // }
                    //
                    #[doc = #deserialize_doc]
                    #[allow(clippy::needless_question_mark)]
                    impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a #ty {
                        fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                            let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                            Ok(#ty::from_normalized_str(raw)#handle_failure)
                        }
                    }
                }
            } else {
                quote! {
                    #[allow(clippy::needless_question_mark)]
                    impl<'de: 'a, 'a> ::serde::Deserialize<'de> for &'a #ty {
                        fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                            let raw = <&str as ::serde::Deserialize<'de>>::deserialize(deserializer)?;
                            Ok(#ty::from_str(raw)#handle_failure)
                        }
                    }
                }
            };

            quote! {
                impl ::serde::Serialize for #ty {
                    fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                        <str as ::serde::Serialize>::serialize(self.as_str(), serializer)
                    }
                }

                #deserialize
                #deserialize_boxed
            }
        })
    }
}