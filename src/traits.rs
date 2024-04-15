//use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use trait_set::trait_set;

trait_set! 
{
    /// Trait for any type that can represent individual character in a [crossword](crate::crossword::Crossword).
    pub trait CrosswordChar = Eq + PartialEq + Ord + PartialOrd + Clone + Default + Debug + Send + Sync;
    
    /// Trait for any type that can represent individual word value in a [crossword](crate::crossword::Crossword).
    pub trait CrosswordString<CharT: CrosswordChar> = AsRef<[CharT]> + Eq + PartialEq + Ord + PartialOrd + Clone + Default + Debug + Send + Sync;
}
