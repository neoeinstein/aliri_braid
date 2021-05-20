use std::fmt::{self, Display};
use syn::{Ident, Path};

#[derive(Copy, Clone, Debug)]
pub struct Symbol(&'static str);

pub const NO_AUTO_REF: Symbol = Symbol("no_auto_ref");
pub const OWNED: Symbol = Symbol("owned");
pub const OMIT_CLONE: Symbol = Symbol("omit_clone");
pub const OMIT_DEBUG: Symbol = Symbol("omit_debug");
pub const OMIT_DISPLAY: Symbol = Symbol("omit_display");
pub const DEBUG_IMPL: Symbol = Symbol("debug_impl");
pub const DISPLAY_IMPL: Symbol = Symbol("display_impl");
pub const REF: Symbol = Symbol("ref");
pub const REF_DOC: Symbol = Symbol("ref_doc");
pub const SERDE: Symbol = Symbol("serde");
pub const VALIDATOR: Symbol = Symbol("validator");
pub const NORMALIZER: Symbol = Symbol("normalizer");

impl PartialEq<Symbol> for Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl<'a> PartialEq<Symbol> for &'a Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}
