use super::{impls::ToImpl, AttrList, CheckMode, Field, Impls};
use quote::{quote, ToTokens};

pub struct OwnedCodeGen<'a> {
    pub common_attrs: &'a [syn::Attribute],
    pub attrs: &'a AttrList<'a>,
    pub body: &'a syn::ItemStruct,
    pub ty: &'a syn::Ident,
    pub field: Field<'a>,
    pub check_mode: &'a CheckMode,
    pub ref_ty: &'a syn::Type,
    pub impls: &'a Impls,
}

impl<'a> OwnedCodeGen<'a> {
    fn constructor(&self) -> proc_macro2::TokenStream {
        match &self.check_mode {
            CheckMode::None => self.infallible_constructor(),
            CheckMode::Validate(validator) => self.fallible_constructor(validator),
            CheckMode::Normalize(normalizer) => self.normalized_constructor(normalizer),
        }
    }

    fn infallible_constructor(&self) -> proc_macro2::TokenStream {
        let doc_comment = format!("Constructs a new {}", self.ty);

        let param = self.field.name.input_name();
        let create = self.field.self_constructor();
        let ref_ty = self.ref_ty;
        let wrapped_type = self.field.ty;

        quote! {
            #[doc = #doc_comment]
            #[inline]
            pub const fn new(#param: #wrapped_type) -> Self {
                // const fn ensure_infallible<T: ::aliri_braid::OwnedValue<str, Error=::std::convert::Infallible>>(_: &T) {}
                // ensure_infallible(&#param);
                #create
            }

            #[inline]
            #[doc = #doc_comment]
            #[track_caller]
            pub fn from_static(raw: &'static str) -> Self {
                #ref_ty::from_str(raw).to_owned()
            }
        }
    }

    fn fallible_constructor(&self, validator: &syn::Type) -> proc_macro2::TokenStream {
        let validator_tokens = validator.to_token_stream();
        let doc_comment = format!(
            "Constructs a new {} if it conforms to [`{}`]",
            self.ty, validator_tokens
        );

        let doc_comment_unsafe = format!(
            "Constructs a new {} without validation\n\
        \n\
        ## Safety\n\
        \n\
        Consumers of this function must ensure that values conform to [`{}`]. \
        Failure to maintain this invariant may lead to undefined behavior.",
            self.ty, validator_tokens
        );

        let validator = crate::as_validator(validator);
        let param = self.field.name.input_name();
        let create = self.field.self_constructor();
        let ref_ty = self.ref_ty;
        let wrapped_type = self.field.ty;

        quote! {
            #[doc = #doc_comment]
            #[inline]
            pub fn new(#param: #wrapped_type) -> Result<Self, #validator::Error> {
                #validator::validate(#param.as_str())?;
                Ok(#create)
            }

            #[doc = #doc_comment_unsafe]
            #[inline]
            pub const unsafe fn new_unchecked(#param: #wrapped_type) -> Self {
                #create
            }

            #[inline]
            #[doc = #doc_comment]
            #[doc = ""]
            #[doc = "## Panics"]
            #[doc = ""]
            #[doc = "This function will panic if the provided raw string is not valid."]
            #[track_caller]
            pub fn from_static(raw: &'static str) -> Self {
                #ref_ty::from_static(raw).to_owned()
            }
        }
    }

    fn normalized_constructor(&self, normalizer: &syn::Type) -> proc_macro2::TokenStream {
        let normalizer_tokens = normalizer.to_token_stream();
        let doc_comment = format!(
            "Constructs a new {} if it conforms to [`{}`] and normalizes the input",
            self.ty, normalizer_tokens
        );

        let doc_comment_unsafe = format!(
            "Constructs a new {} without validation or normalization\n\
            \n\
            ## Safety\n\
            \n\
            Consumers of this function must ensure that values conform to [`{}`] and \
            are in normalized form. Failure to maintain this invariant may lead to \
            undefined behavior.",
            self.ty, normalizer_tokens
        );

        let ty = self.ty;
        let normalizer = crate::as_normalizer(normalizer);
        let param = self.field.name.input_name();
        let create = self.field.self_constructor();
        let ref_ty = self.ref_ty;
        let field_ty = self.field.ty;

        quote! {
            #[doc = #doc_comment]
            #[inline]
            pub fn new(#param: #field_ty) -> Result<Self, #normalizer::Error> {
                let #param = #normalizer::normalize(#param.as_str())?.into_owned();
                Ok(#create)
            }

            #[doc = #doc_comment_unsafe]
            #[inline]
            pub const unsafe fn new_unchecked(#param: #field_ty) -> Self {
                #create
            }

            #[inline]
            #[doc = #doc_comment]
            #[doc = ""]
            #[doc = "## Panics"]
            #[doc = ""]
            #[doc = "This function will panic if the provided raw string is not valid."]
            #[track_caller]
            pub fn from_static(raw: &'static str) -> Self {
                #ref_ty::from_str(raw).expect(concat!("invalid ", stringify!(#ty))).into_owned()
            }
        }
    }

    fn make_into_boxed_ref(&self) -> proc_macro2::TokenStream {
        let doc = format!(
            "Converts this `{}` into a [`Box`]`<`[`{}`]`>`\n\
            \n\
            This will drop any excess capacity.",
            self.ty,
            self.ref_ty.to_token_stream(),
        );

        let ref_type = self.ref_ty;
        let field = self.field.name;
        let box_pointer_reinterpret_safety_comment = {
            let doc = format!(
                "SAFETY: `{ty}` is `#[repr(transparent)]` around a single `str` \
                field, so a `*mut str` can be safely reinterpreted as a \
                `*mut {ty}`",
                ty = self.ref_ty.to_token_stream(),
            );

            quote! {
                #[doc = #doc]
                fn ptr_safety_comment() {}
            }
        };

        quote! {
            #[doc = #doc]
            #[inline]
            #[allow(unsafe_code)]
            pub fn into_boxed_ref(self) -> Box<#ref_type> {
                #box_pointer_reinterpret_safety_comment
                let box_str = self.#field.into_boxed_str();
                unsafe { ::std::boxed::Box::from_raw(::std::boxed::Box::into_raw(box_str) as *mut #ref_type) }
            }
        }
    }

    fn make_into_string(&self) -> proc_macro2::TokenStream {
        let field = self.field.name;
        let wrapped_type = self.field.ty;
        let doc = format!(
            "Unwraps the underlying [`{}`] value",
            wrapped_type.to_token_stream()
        );

        quote! {
            #[doc = #doc]
            #[inline]
            pub fn into_string(self) -> #wrapped_type {
                self.#field
            }
        }
    }

    fn inherent(&self) -> proc_macro2::TokenStream {
        let name = self.ty;
        let constructor = self.constructor();
        let into_boxed_ref = self.make_into_boxed_ref();
        let into_string = self.make_into_string();

        quote! {
            impl #name {
                #constructor
                #into_boxed_ref
                #into_string
            }
        }
    }

    fn common_conversion(&self) -> proc_macro2::TokenStream {
        let ty = self.ty;
        let ref_ty = self.ref_ty;

        quote! {
            impl From<&'_ #ref_ty> for #ty {
                #[inline]
                fn from(s: &#ref_ty) -> Self {
                    s.to_owned()
                }
            }

            impl ::std::borrow::Borrow<#ref_ty> for #ty {
                #[inline]
                fn borrow(&self) -> &#ref_ty {
                    ::std::ops::Deref::deref(self)
                }
            }

            impl AsRef<#ref_ty> for #ty {
                #[inline]
                fn as_ref(&self) -> &#ref_ty {
                    ::std::ops::Deref::deref(self)
                }
            }

            impl AsRef<str> for #ty {
                #[inline]
                fn as_ref(&self) -> &str {
                    self.as_str()
                }
            }


            impl From<#ty> for Box<#ref_ty> {
                #[inline]
                fn from(r: #ty) -> Self {
                    r.into_boxed_ref()
                }
            }

            impl From<Box<#ref_ty>> for #ty {
                #[inline]
                fn from(r: Box<#ref_ty>) -> Self {
                    r.into_owned()
                }
            }

            impl<'a> From<::std::borrow::Cow<'a, #ref_ty>> for #ty {
                #[inline]
                fn from(r: ::std::borrow::Cow<'a, #ref_ty>) -> Self {
                    match r {
                        ::std::borrow::Cow::Borrowed(b) => b.to_owned(),
                        ::std::borrow::Cow::Owned(o) => o,
                    }
                }
            }

            impl<'a> From<#ty> for ::std::borrow::Cow<'a, #ref_ty> {
                #[inline]
                fn from(owned: #ty) -> Self {
                    ::std::borrow::Cow::Owned(owned)
                }
            }
        }
    }

    fn infallible_conversion(&self) -> proc_macro2::TokenStream {
        let ty = self.ty;
        let ref_ty = self.ref_ty;
        let field_ty = self.field.ty;
        let field_name = self.field.name;

        quote! {
            impl From<#field_ty> for #ty {
                #[inline]
                fn from(s: #field_ty) -> Self {
                    Self::new(s)
                }
            }

            impl From<&'_ str> for #ty {
                #[inline]
                fn from(s: &str) -> Self {
                    Self::new(#field_ty::from(s))
                }
            }

            impl ::std::str::FromStr for #ty {
                type Err = ::std::convert::Infallible;

                #[inline]
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Ok(Self::from(s))
                }
            }

            impl ::std::borrow::Borrow<str> for #ty {
                #[inline]
                fn borrow(&self) -> &str {
                    self.as_str()
                }
            }

            impl ::std::ops::Deref for #ty {
                type Target = #ref_ty;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    #ref_ty::from_str(self.#field_name.as_str())
                }
            }
        }
    }

    fn fallible_conversion(&self, validator: &syn::Type) -> proc_macro2::TokenStream {
        let ty = self.ty;
        let ref_ty = self.ref_ty;
        let field_ty = self.field.ty;
        let field_name = self.field.name;
        let validator = crate::as_validator(validator);

        quote! {
            impl ::std::convert::TryFrom<#field_ty> for #ty {
                type Error = #validator::Error;

                #[inline]
                fn try_from(s: #field_ty) -> Result<Self, Self::Error> {
                    Self::new(s)
                }
            }

            impl ::std::convert::TryFrom<&'_ str> for #ty {
                type Error = #validator::Error;

                #[inline]
                fn try_from(s: &str) -> Result<Self, Self::Error> {
                    let ref_ty = #ref_ty::from_str(s)?;
                    Ok(ref_ty.to_owned())
                }
            }

            impl ::std::str::FromStr for #ty {
                type Err = #validator::Error;

                #[inline]
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let ref_ty = #ref_ty::from_str(s)?;
                    Ok(ref_ty.to_owned())
                }
            }

            impl ::std::borrow::Borrow<str> for #ty {
                #[inline]
                fn borrow(&self) -> &str {
                    self.as_str()
                }
            }

            impl ::std::ops::Deref for #ty {
                type Target = #ref_ty;

                #[inline]
                #[allow(unsafe_code)]
                fn deref(&self) -> &Self::Target {
                    // SAFETY: At this point, we are certain that the underlying string
                    // slice passes validation, so the implicit contract is satisfied.
                    unsafe { #ref_ty::from_str_unchecked(self.#field_name.as_str()) }
                }
            }
        }
    }

    fn normalized_conversion(&self, normalizer: &syn::Type) -> proc_macro2::TokenStream {
        let ty = self.ty;
        let ref_ty = self.ref_ty;
        let field_ty = self.field.ty;
        let field_name = self.field.name;
        let normalizer = crate::as_normalizer(normalizer);

        quote! {
            impl ::std::convert::TryFrom<#field_ty> for #ty {
                type Error = #normalizer::Error;

                #[inline]
                fn try_from(s: #field_ty) -> Result<Self, Self::Error> {
                    Self::new(s)
                }
            }

            impl ::std::convert::TryFrom<&'_ str> for #ty {
                type Error = #normalizer::Error;

                #[inline]
                fn try_from(s: &str) -> Result<Self, Self::Error> {
                    let ref_ty = #ref_ty::from_str(s)?;
                    Ok(ref_ty.into_owned())
                }
            }

            impl ::std::str::FromStr for #ty {
                type Err = #normalizer::Error;

                #[inline]
                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let ref_ty = #ref_ty::from_str(s)?;
                    Ok(ref_ty.into_owned())
                }
            }

            impl ::std::ops::Deref for #ty {
                type Target = #ref_ty;

                #[inline]
                #[allow(unsafe_code)]
                fn deref(&self) -> &Self::Target {
                    // SAFETY: At this point, we are certain that the underlying string
                    // slice passes validation, so the implicit contract is satisfied.
                    unsafe { #ref_ty::from_str_unchecked(&self.#field_name) }
                }
            }
        }
    }

    fn conversion(&self) -> proc_macro2::TokenStream {
        let common = self.common_conversion();
        let convert = match &self.check_mode {
            CheckMode::None => self.infallible_conversion(),
            CheckMode::Validate(validator) => self.fallible_conversion(validator),
            CheckMode::Normalize(normalizer) => self.normalized_conversion(normalizer),
        };

        quote! {
            #common
            #convert
        }
    }

    pub fn tokens(&self) -> proc_macro2::TokenStream {
        let clone = self.impls.clone.to_owned_impl(self);
        let display = self.impls.display.to_owned_impl(self);
        let debug = self.impls.debug.to_owned_impl(self);
        let serde = self.impls.serde.to_owned_impl(self);

        let owned_attrs: proc_macro2::TokenStream =
            self.attrs.iter().map(|a| quote! {#[#a]}).collect();
        let body = &self.body;
        let inherent = self.inherent();
        let conversion = self.conversion();

        quote! {
            #clone
            #[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
            #[repr(transparent)]
            #owned_attrs
            #body

            #inherent
            #conversion
            #debug
            #display
            #serde
        }
    }
}
