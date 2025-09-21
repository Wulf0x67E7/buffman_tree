mod trie;
pub use trie::*;
pub mod testing;
#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn nop() {}
}

#[doc(hidden)]
#[allow(dead_code)]
#[macro_use]
pub mod util;
