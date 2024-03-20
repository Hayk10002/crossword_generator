use trait_set::trait_set;

trait_set! 
{
    pub trait CrosswordChar = Ord + PartialOrd + Clone + Default;
    pub trait CrosswordString<CharT: CrosswordChar> = AsRef<[CharT]> + Ord + Clone;
}
