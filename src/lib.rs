pub mod utils;
pub mod word;
pub mod placed_word;
pub mod crossword;
pub mod generator;


pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use self::{generator::{CrosswordGenerationRequest, CrosswordGenerator, CrosswordGeneratorSettings}, word::Word};
    use tokio_stream::StreamExt;

    use super::*;

    #[tokio::test]
    async fn it_works() {
        let gen = CrosswordGenerator::<u8, String>
        {
            words: [Word::<u8, String>::new("a".to_owned(), None),
                    Word::<u8, String>::new("accb".to_owned(), None),
                    Word::<u8, String>::new("b".to_owned(), None)].into_iter().collect(),
            settings: CrosswordGeneratorSettings::default()
        };

        let mut str = gen.crossword_stream();
        str.request_crossword(CrosswordGenerationRequest::Count(10)).await;
        str.request_crossword(CrosswordGenerationRequest::Stop).await;


        let mut crosswords = vec![];
        while let Some(cw) = str.next().await
        {
            crosswords.push(cw);
        }
        
        println!("{}", serde_json::to_string_pretty(&crosswords).unwrap());
    }
}
