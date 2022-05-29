use super::{impls::ToImpl, AttrList, CheckMode, Field, FieldName, Impls};
use quote::{quote, ToTokens, TokenStreamExt};
use std::borrow::Cow;

pub struct RefCodeGen<'a> {
    pub doc: &'a [Cow<'a, syn::Lit>],
    pub common_attrs: &'a [syn::Attribute],
    pub attrs: &'a AttrList<'a>,
    pub vis: &'a syn::Visibility,
    pub ty: &'a syn::Type,
    pub ident: syn::Ident,
    pub field: Field<'a>,
    pub check_mode: &'a CheckMode,
    pub owned_ty: &'a syn::Ident,
    pub impls: &'a Impls,
}

impl<'a> RefCodeGen<'a> {
    fn inherent(&self) -> proc_macro2::TokenStream {
        let ty = &self.ty;
        let field_name = self.field.name;
        let inherent = self.check_inherent();

        quote! {
            impl #ty {
                #inherent

                /// Provides access to the underlying value as a string slice.
                #[inline]
                pub const fn as_str(&self) -> &str {
                    &self.#field_name
                }
            }
        }
    }

    fn check_inherent(&self) -> proc_macro2::TokenStream {
        match self.check_mode {
            CheckMode::None => self.infallible_inherent(),
            CheckMode::Validate(validator) => self.fallible_inherent(validator),
            CheckMode::Normalize(normalizer) => self.normalized_inherent(normalizer),
        }
    }

    fn pointer_reinterpret_safety_comment(&self, is_mut: bool) -> proc_macro2::TokenStream {
        let doc = format!(
            "SAFETY: `{ty}` is `#[repr(transparent)]` around a single `str` \
            field, so a `*{ptr} str` can be safely reinterpreted as a \
            `*{ptr} {ty}`",
            ty = self.ident,
            ptr = if is_mut { "mut" } else { "const" },
        );

        quote! {
            #[doc = #doc]
            fn ptr_safety_comment() {}
        }
    }

    fn unchecked_safety_comment(is_normalized: bool) -> proc_macro2::TokenStream {
        let doc = format!(
            "SAFETY: The value was just checked and found to already \
            conform to the required implicit contracts of the {}.",
            if is_normalized {
                "normalizer"
            } else {
                "validator"
            },
        );

        quote! {
            #[doc = #doc]
            fn unchecked_safety_comment() {}
        }
    }

    fn infallible_inherent(&self) -> proc_macro2::TokenStream {
        let ty = &self.ty;
        let owned_ty = self.owned_ty;

        let doc_comment = format!(
            "Transparently reinterprets the string slice as a strongly-typed {}",
            self.ident
        );

        let static_doc_comment = format!(
            "Transparently reinterprets the static string slice as a strongly-typed {}",
            self.ident
        );

        let into_owned_doc = format!(
            "Converts a [`Box<{}>`] into a [`{}`] without copying or allocating",
            self.ident, self.owned_ty,
        );

        let pointer_reinterpret_safety_comment = self.pointer_reinterpret_safety_comment(false);
        let box_pointer_reinterpret_safety_comment = self.pointer_reinterpret_safety_comment(true);

        quote! {
            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment]
            pub const fn from_str(raw: &str) -> &Self {
                let ptr: *const str = raw;
                #pointer_reinterpret_safety_comment
                unsafe {
                    &*(ptr as *const Self)
                }
            }

            #[inline]
            #[doc = #static_doc_comment]
            #[track_caller]
            pub const fn from_static(raw: &'static str) -> &'static Self {
                Self::from_str(raw)
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #into_owned_doc]
            pub fn into_owned(self: Box<#ty>) -> #owned_ty {
                #box_pointer_reinterpret_safety_comment
                let raw = ::std::boxed::Box::into_raw(self);
                let boxed = unsafe { ::std::boxed::Box::from_raw(raw as *mut str) };
                #owned_ty::new(boxed.into_string())
            }
        }
    }

    fn fallible_inherent(&self, validator: &syn::Type) -> proc_macro2::TokenStream {
        let doc_comment = format!(
            "Transparently reinterprets the string slice as a strongly-typed {} \
            if it conforms to [`{}`]",
            self.ident,
            validator.to_token_stream(),
        );

        let doc_comment_unsafe = format!(
            "Transparently reinterprets the string slice as a strongly-typed {} \
            without validating",
            self.ident,
        );

        let into_owned_doc = format!(
            "Converts a [`Box<{}>`] into a [`{}`] without copying or allocating",
            self.ident, self.owned_ty,
        );

        let ty = &self.ty;
        let owned_ty = self.owned_ty;
        let unchecked_safety_comment = Self::unchecked_safety_comment(false);
        let pointer_reinterpret_safety_comment = self.pointer_reinterpret_safety_comment(false);
        let box_pointer_reinterpret_safety_comment = self.pointer_reinterpret_safety_comment(true);

        let validator = crate::as_validator(validator);

        quote! {
            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment]
            pub fn from_str(raw: &str) -> Result<&Self, #validator::Error> {
                #validator::validate(raw)?;
                #unchecked_safety_comment
                Ok(unsafe { Self::from_str_unchecked(raw) })
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment_unsafe]
            pub const unsafe fn from_str_unchecked(raw: &str) -> &Self {
                let ptr: *const str = raw;
                #pointer_reinterpret_safety_comment
                unsafe {
                    &*(ptr as *const Self)
                }
            }

            #[inline]
            #[doc = #doc_comment]
            #[doc = ""]
            #[doc = "## Panics"]
            #[doc = ""]
            #[doc = "This function will panic if the provided raw string is not valid."]
            #[track_caller]
            pub fn from_static(raw: &'static str) -> &'static Self {
                Self::from_str(raw).expect(concat!("invalid ", stringify!(#ty)))
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #into_owned_doc]
            pub fn into_owned(self: Box<#ty>) -> #owned_ty {
                #box_pointer_reinterpret_safety_comment
                let raw = ::std::boxed::Box::into_raw(self);
                let boxed = unsafe { ::std::boxed::Box::from_raw(raw as *mut str) };
                let s = boxed.into_string();
                #unchecked_safety_comment
                unsafe { #owned_ty::new_unchecked(s) }
            }
        }
    }

    fn normalized_inherent(&self, normalizer: &syn::Type) -> proc_macro2::TokenStream {
        let doc_comment = format!(
            "Transparently reinterprets the string slice as a strongly-typed {} \
            if it conforms to [`{}`], normalizing if necessary",
            self.ident,
            normalizer.to_token_stream(),
        );

        let doc_comment_norm = format!(
            "Transparently reinterprets the string slice as a strongly-typed `{}` \
            if it conforms to [`{}`], producing an error if normalization is necessary",
            self.ident,
            normalizer.to_token_stream(),
        );

        let doc_comment_unsafe = format!(
            "Transparently reinterprets the string slice as a strongly-typed `{}` \
            without validating\n\
            \n\
            ## Safety\n\
            \n\
            Calls to this function must ensure that the value being passed conforms \
            to [`{}`] and is already in normalized form. Failure to do this may \
            result in undefined behavior if other code relies on this invariant.",
            self.ident,
            normalizer.to_token_stream(),
        );

        let doc_comment_cow_unsafe = format!(
            "Transparently reinterprets the [`Cow<str>`][std::borrow::Cow] as a \
            strongly-typed [`Cow`][std::borrow::Cow]`<{}>` without validating\n\
            \n\
            ## Safety\n\
            \n\
            Calls to this function must ensure that the value being passed conforms \
            to [`{}`] and is already in normalized form. Failure to do this may \
            result in undefined behavior if other code relies on this invariant.",
            self.ident,
            normalizer.to_token_stream(),
        );

        let into_owned_doc = format!(
            "Converts a [`Box<{}>`] into a [`{}`] without copying or allocating",
            self.ident,
            self.owned_ty.to_token_stream(),
        );

        let ty = &self.ty;
        let owned_ty = self.owned_ty;
        let unchecked_safety_comment = Self::unchecked_safety_comment(true);
        let pointer_reinterpret_safety_comment = self.pointer_reinterpret_safety_comment(false);
        let box_pointer_reinterpret_safety_comment = self.pointer_reinterpret_safety_comment(true);

        let validator = crate::as_validator(normalizer);
        let normalizer = crate::as_normalizer(normalizer);

        quote! {
            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment]
            pub fn from_str(raw: &str) -> Result<::std::borrow::Cow<Self>, #normalizer::Error> {
                let cow = #normalizer::normalize(raw)?;
                #unchecked_safety_comment
                Ok(unsafe { Self::from_cow_str_unchecked(cow) })
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment_cow_unsafe]
            unsafe fn from_cow_str_unchecked(cow: ::std::borrow::Cow<str>) -> ::std::borrow::Cow<Self> {
                match cow {
                    ::std::borrow::Cow::Borrowed(raw) => {
                        let value = Self::from_str_unchecked(raw);
                        ::std::borrow::Cow::Borrowed(value)
                    }
                    ::std::borrow::Cow::Owned(normalized) => {
                        let value = #owned_ty::new_unchecked(normalized);
                        ::std::borrow::Cow::Owned(value)
                    }
                }
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment_norm]
            pub fn from_normalized_str(raw: &str) -> Result<&Self, #validator::Error> {
                #validator::validate(raw)?;
                #unchecked_safety_comment
                Ok(unsafe { Self::from_str_unchecked(raw) })
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #doc_comment_unsafe]
            pub const unsafe fn from_str_unchecked(raw: &str) -> &Self {
                let ptr: *const str = raw;
                #pointer_reinterpret_safety_comment
                unsafe {
                    &*(ptr as *const Self)
                }
            }
            #[inline]
            #[doc = #doc_comment]
            #[doc = ""]
            #[doc = "## Panics"]
            #[doc = ""]
            #[doc = "This function will panic if the provided raw string is not normalized."]
            #[track_caller]
            pub fn from_static(raw: &'static str) -> &'static Self {
                Self::from_normalized_str(raw).expect(concat!("non-normalized ", stringify!(#ty)))
            }

            #[allow(unsafe_code)]
            #[inline]
            #[doc = #into_owned_doc]
            pub fn into_owned(self: Box<#ty>) -> #owned_ty {
                #box_pointer_reinterpret_safety_comment
                let raw = ::std::boxed::Box::into_raw(self);
                let boxed = unsafe { ::std::boxed::Box::from_raw(raw as *mut str) };
                let s = boxed.into_string();
                #unchecked_safety_comment
                unsafe { #owned_ty::new_unchecked(s) }
            }
        }
    }

    fn comparison(&self) -> proc_macro2::TokenStream {
        let ty = &self.ty;
        let owned_ty = self.owned_ty;

        let create = match self.field.name {
            FieldName::Unnamed => quote! { #owned_ty(self.0.to_owned()) },
            FieldName::Named(field_name) => {
                quote! { #owned_ty { #field_name: self.#field_name.to_owned() } }
            }
        };

        quote! {
            impl ToOwned for #ty {
                type Owned = #owned_ty;

                #[inline]
                fn to_owned(&self) -> Self::Owned {
                    #create
                }
            }

            impl PartialEq<#ty> for #owned_ty {
                #[inline]
                fn eq(&self, other: &#ty) -> bool {
                    self.as_str() == other.as_str()
                }
            }

            impl PartialEq<#owned_ty> for #ty {
                #[inline]
                fn eq(&self, other: &#owned_ty) -> bool {
                    self.as_str() == other.as_str()
                }
            }

            impl PartialEq<&'_ #ty> for #owned_ty {
                #[inline]
                fn eq(&self, other: &&#ty) -> bool {
                    self.as_str() == other.as_str()
                }
            }

            impl PartialEq<#owned_ty> for &'_ #ty {
                #[inline]
                fn eq(&self, other: &#owned_ty) -> bool {
                    self.as_str() == other.as_str()
                }
            }
        }
    }

    fn conversion(&self) -> proc_macro2::TokenStream {
        let ty = &self.ty;
        let field_name = self.field.name;

        let from_str = match &self.check_mode {
            CheckMode::None => quote! {
                impl<'a> From<&'a str> for &'a #ty {
                    #[inline]
                    fn from(s: &'a str) -> &'a #ty {
                        #ty::from_str(s)
                    }
                }

                impl ::std::borrow::Borrow<str> for #ty {
                    #[inline]
                    fn borrow(&self) -> &str {
                        &self.#field_name
                    }
                }
            },
            CheckMode::Validate(validator) => {
                let validator = crate::as_validator(validator);
                quote! {
                    impl<'a> std::convert::TryFrom<&'a str> for &'a #ty {
                        type Error = #validator::Error;

                        #[inline]
                        fn try_from(s: &'a str) -> Result<&'a #ty, Self::Error> {
                            #ty::from_str(s)
                        }
                    }

                    impl ::std::borrow::Borrow<str> for #ty {
                        #[inline]
                        fn borrow(&self) -> &str {
                            &self.#field_name
                        }
                    }
                }
            }
            CheckMode::Normalize(normalizer) => {
                let validator = crate::as_validator(normalizer);
                quote! {
                    impl<'a> std::convert::TryFrom<&'a str> for &'a #ty {
                        type Error = #validator::Error;

                        #[inline]
                        fn try_from(s: &'a str) -> Result<&'a #ty, Self::Error> {
                            #ty::from_normalized_str(s)
                        }
                    }
                }
            }
        };

        quote! {
            #from_str

            impl AsRef<str> for #ty {
                #[inline]
                fn as_ref(&self) -> &str {
                    &self.#field_name
                }
            }

            impl<'a> From<&'a #ty> for ::std::borrow::Cow<'a, #ty> {
                #[inline]
                fn from(r: &'a #ty) -> Self {
                    ::std::borrow::Cow::Borrowed(r)
                }
            }


            impl<'a, 'b: 'a> From<&'a ::std::borrow::Cow<'b, #ty>> for &'a #ty {
                #[inline]
                fn from(r: &'a ::std::borrow::Cow<'b, #ty>) -> &'a #ty {
                    ::std::borrow::Borrow::borrow(r)
                }
            }
        }
    }

    pub fn tokens(&self) -> proc_macro2::TokenStream {
        let inherent = self.inherent();
        let comparison = self.comparison();
        let conversion = self.conversion();
        let debug = self.impls.debug.to_borrowed_impl(self);
        let display = self.impls.display.to_borrowed_impl(self);
        let serde = self.impls.serde.to_borrowed_impl(self);

        let ref_doc: proc_macro2::TokenStream =
            self.doc.iter().map(|d| quote! { #[doc = #d] }).collect();
        let ref_attrs: proc_macro2::TokenStream =
            self.attrs.iter().map(|a| quote! {#[#a]}).collect();
        let common_attrs = {
            let mut attrs = proc_macro2::TokenStream::new();
            if !self.doc.is_empty() {
                attrs.append_all(self.common_attrs.iter().filter(|a| !is_doc_attribute(a)));
            } else {
                attrs.append_all(self.common_attrs);
            }
            attrs
        };
        let vis = self.vis;
        let ty = &self.ty;
        let field_attrs = {
            let mut attrs = proc_macro2::TokenStream::new();
            attrs.append_all(self.field.attrs);
            attrs
        };
        let body = match self.field.name {
            FieldName::Named(name) => quote! ( { #field_attrs #name: str } ),
            FieldName::Unnamed => quote! { ( #field_attrs str ); },
        };

        quote! {
            #[repr(transparent)]
            #[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
            #ref_doc
            #ref_attrs
            #common_attrs
            #vis struct #ty #body

            #inherent
            #comparison
            #conversion
            #debug
            #display
            #serde
        }
    }
}

fn is_doc_attribute(attr: &syn::Attribute) -> bool {
    if let Some(ident) = attr.path.get_ident() {
        ident == "doc"
    } else {
        false
    }
}
