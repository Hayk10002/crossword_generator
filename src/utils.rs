//use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use trait_set::trait_set;

trait_set! 
{
    pub trait CrosswordChar = Eq + PartialEq + Ord + PartialOrd + Clone + Default + Debug + Send + Sync;
    pub trait CrosswordString<CharT: CrosswordChar> = AsRef<[CharT]> + Eq + PartialEq + Ord + PartialOrd + Clone + Default + Debug + Send + Sync;
}
