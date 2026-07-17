//! A unit of source to evaluate, built by [`Runtime::load`](crate::Runtime::load).

use std::ptr;

use crate::convert::FromJs;
use crate::error::Result;
use crate::runtime::Runtime;
use crate::sys;
use crate::value::Value;

/// Which dialect the source is written in. TypeScript and JSX are transpiled by
/// Bun before evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Js,
    Ts,
    Jsx,
    Tsx,
}

impl Lang {
    fn to_sys(self) -> sys::LoafLang {
        match self {
            Lang::Js => sys::LoafLang::Js,
            Lang::Ts => sys::LoafLang::Ts,
            Lang::Jsx => sys::LoafLang::Jsx,
            Lang::Tsx => sys::LoafLang::Tsx,
        }
    }
}

/// A pending piece of source. Configure it, then finish with [`Chunk::eval`],
/// [`Chunk::exec`], or [`Chunk::into_value`].
#[derive(Debug)]
pub struct Chunk<'rt> {
    rt: &'rt Runtime,
    src: String,
    name: Option<String>,
    lang: Lang,
    module: bool,
}

impl<'rt> Chunk<'rt> {
    pub(crate) fn new(rt: &'rt Runtime, src: String) -> Self {
        Chunk {
            rt,
            src,
            name: None,
            lang: Lang::Js,
            module: false,
        }
    }

    /// Set the chunk name used in stack traces and module resolution.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the source dialect explicitly.
    pub fn lang(mut self, lang: Lang) -> Self {
        self.lang = lang;
        self
    }

    /// Treat the source as TypeScript.
    pub fn typescript(self) -> Self {
        self.lang(Lang::Ts)
    }

    /// Treat the source as JSX.
    pub fn jsx(self) -> Self {
        self.lang(Lang::Jsx)
    }

    /// Treat the source as TSX (TypeScript + JSX).
    pub fn tsx(self) -> Self {
        self.lang(Lang::Tsx)
    }

    /// Evaluate as an ES module rather than a classic script.
    pub fn module(mut self, yes: bool) -> Self {
        self.module = yes;
        self
    }

    fn run(&self) -> Result<Value> {
        let name = self.name.as_deref().unwrap_or("<loaf>");
        let opts = sys::LoafEvalOptions {
            lang: self.lang.to_sys(),
            module: if self.module {
                sys::LoafModuleKind::Module
            } else {
                sys::LoafModuleKind::Script
            },
            filename: name.as_ptr().cast(),
            filename_len: name.len(),
        };
        let mut out = ptr::null_mut();
        let st = unsafe {
            sys::loaf_eval(
                self.rt.inner.raw,
                self.src.as_ptr().cast(),
                self.src.len(),
                &opts,
                &mut out,
            )
        };
        self.rt.check(st)?;
        Ok(Value::from_raw(&self.rt.inner, out))
    }

    /// Evaluate the chunk and convert the result to `T`.
    pub fn eval<T: FromJs>(self) -> Result<T> {
        let value = self.run()?;
        T::from_js(value, self.rt)
    }

    /// Evaluate the chunk for its side effects, discarding the result.
    pub fn exec(self) -> Result<()> {
        self.run().map(|_| ())
    }

    /// Evaluate the chunk and return the raw [`Value`].
    pub fn into_value(self) -> Result<Value> {
        self.run()
    }
}
