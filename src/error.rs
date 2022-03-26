use std::borrow::Cow;
use std::error;
use std::fmt::{self, Debug, Display, Formatter};
use thiserror;
use std::convert::From;


#[derive(thiserror::Error, Debug)]
pub enum ErrorKind {
    // #[error("Deserialize from json string failed. Detailed message:\n{0}")]
    // DeserializeJsonError(#[from] serde_json::Error),
    // #[error("Data integrity checks failed: {0}")]
    // CorruptedDataError(String),
    #[error("File IO operation failed. Detailed message:\n{0}")]
    IOError(#[from] std::io::Error),
    #[error("ValueError: {0}")]
    ValueError(Cow<'static, str>),
    #[error("ValueError: {0}")]
    UnknownError(Cow<'static, str>),
}

pub struct Error {
    pub(crate) kind: ErrorKind,
    contexts: Vec<Cow<'static, str>>
}

impl<T: Into<ErrorKind>> From<T> for Error {
    fn from(e: T) -> Self {
        let kind: ErrorKind = e.into();
        Error::new(kind)
    }
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind, contexts: vec![]}
    }

    fn ext_context(mut self, ctx: Cow<'static, str>) -> Self {
        self.contexts.push(ctx);
        self
    }

    fn collect_contexts(&self) -> String {
        let mut res = String::new();
        self.contexts.iter()
            .rev()
            .fold(&mut res, |acc: &mut String, item| {
                acc.push_str("\n - ");
                acc.push_str(item);
                acc
            });
        res
    }
}


pub trait Context<T> {
    /// Wrap the error value with additional context that is evaluated lazily
    /// only once an error does occur.
    fn with_context<F>(self, f: F) -> Result<T, Error>
        where F: FnOnce() -> Cow<'static, str>;
}


// 对所有能够转换为 Error 错误类型的 Result, 都定义 with_context 函数.
impl<T, E> Context<T> for Result<T, E>
    where E: Into<Error>
{
    fn with_context<F>(self, context: F) -> Result<T, Error>
        where F: FnOnce() -> Cow<'static, str>
    {
        // 没有找到在编译器检查类型的方法,
        // 如下的判断是在运行期进行的, 导致编译出错
        // 目前来看, 当输入的错误恰好是 Error 类型时, 只能多一层map_err. 或者多写一个 impl 块.
        // if TypeId::of::<E>() == TypeId::of::<Error>() {
        //     self.map_err(|error| error.ext_context(context()))
        // } else {
        //     self.map_err(|e| e.into())
        //         .map_err(|error| error.ext_context(context()))
        // }
        self.map_err(|e| e.into())
            .map_err(|error| error.ext_context(context()))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.kind, f)?;
        if self.contexts.is_empty() {
            return Ok(())
        }
        write!(f, "\nExtra information: {}", self.collect_contexts())
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.kind, f)?;
        if self.contexts.is_empty() {
            return Ok(())
        }
        write!(f, "\nExtra information: {}", self.collect_contexts())
    }
}


impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use ErrorKind::*;
        match &self.kind {
            ValueError(_) => None,
            IOError(x) => Some(x),
            UnknownError(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use super::ErrorKind::*;

    fn inner_function() -> Result<(), Error> {
        let e = Error {
            kind: ErrorKind::ValueError { 0: "corrupted".into() },
            contexts: vec![]
        };
        Err(e)
    }

    fn middle_function() -> Result<(), Error> {
        inner_function()
            .with_context(|| "in middle_function".into())
    }

    #[test]
    #[should_panic]
    fn test_error_with_context() {
        let status = middle_function()
            .with_context(|| format!("in outer function: {}", "test_error_with_context").into());
        status.unwrap();
    }
}
