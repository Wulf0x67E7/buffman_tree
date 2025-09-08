use std::fmt::Debug;

pub trait Key: Debug + PartialEq {
    type Piece: Debug + Clone + Ord;
    fn pieces(&self) -> impl Iterator<Item = &Self::Piece>;
    type IntoPieces: Debug + Iterator<Item = Self::Piece>;
    fn into_pieces(self) -> Self::IntoPieces;
}
#[derive(Debug, PartialEq)]
#[repr(transparent)]
pub struct ByteString(pub String);
impl From<String> for ByteString {
    fn from(value: String) -> Self {
        Self(value)
    }
}
impl Into<String> for ByteString {
    fn into(self) -> String {
        self.0
    }
}
impl Key for ByteString {
    type Piece = u8;
    fn pieces(&self) -> impl Iterator<Item = &Self::Piece> {
        self.0.as_bytes().iter()
    }
    type IntoPieces = <Vec<u8> as IntoIterator>::IntoIter;
    fn into_pieces(self) -> Self::IntoPieces {
        self.0.into_bytes().into_iter()
    }
}
impl<T> Key for T
where
    T: Debug + PartialEq + IntoIterator<Item: Debug + Clone + Ord>,
    T::IntoIter: Debug,
    for<'a> &'a T: IntoIterator<Item = &'a T::Item>,
{
    type Piece = T::Item;
    fn pieces(&self) -> impl Iterator<Item = &Self::Piece> {
        self.into_iter()
    }
    type IntoPieces = T::IntoIter;
    fn into_pieces(self) -> Self::IntoPieces {
        self.into_iter()
    }
}
