error_chain!{

    //foreign_links {
    //    Fmt(::std::fmt::Error);
    //    Io(::std::io::Error) #[cfg(unix)];
    //}

}
use std::fmt::Debug;

pub type StubResult<T> = ::std::result::Result<T, String>;

pub trait IntoStub<T> {
    fn into_stub(self: Self) -> StubResult<T>;
}
impl<T> IntoStub<T> for Result<T> {
    fn into_stub(self: Result<T>) -> StubResult<T> {
        match self {
            Ok(t) => Ok(t),
            Err(Error(kind, _)) => Err(format!("{}", kind).to_string()),
        }
    }
}

pub fn cannot_verb(verb: &str, item: &Debug) -> String {
    format!("Cannot {} {:?}", verb, item).to_string()
}

pub fn cannot_write(path: &Debug) -> String {
    cannot_verb("write", path)
}

pub fn cannot_read(path: &Debug) -> String {
    cannot_verb("read", path)
}

pub fn cannot_create(path: &Debug) -> String {
    cannot_verb("create", path)
}

pub fn cannot_remove(path: &Debug) -> String {
    cannot_verb("remove", path)
}
