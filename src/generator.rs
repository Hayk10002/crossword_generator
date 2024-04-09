use std::{collections::BTreeSet, future::Future, pin::Pin, process::Output, task::{Context, Poll}};

use async_fn_traits::AsyncFnOnce2;
use async_recursion::async_recursion;
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc::{self, error::TryRecvError, Receiver, Sender}, task};
use tokio_stream::Stream;

use crate::{crossword::{Crossword, CrosswordSettings, WordCompatibilitySettings}, placed_word::PlacedWord, utils::{CrosswordChar, CrosswordString}, word::Word};

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct CrosswordGeneratorSettings
{
    pub crossword_settings: CrosswordSettings,
    pub word_compatibility_settings: WordCompatibilitySettings
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub struct CrosswordGenerator<CharT: CrosswordChar, StrT: CrosswordString<CharT>>
{
    pub words: BTreeSet<Word<CharT, StrT>>,
    pub settings: CrosswordGeneratorSettings,
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> CrosswordGenerator<CharT, StrT>
{
    pub fn crossword_stream(&self) -> CrosswordStream<CharT, StrT>
    {  
        let gen = self.clone();

        let gen_func = move |mut rr: Receiver<CrosswordGenerationRequest>, cs: Sender<Crossword<CharT, StrT>>| async move
        {

            let mut current_request = CrosswordGenerationRequest::Count(0);
            let mut current_crossword = Crossword::new(gen.settings.word_compatibility_settings.clone());
            let mut full_created_crossword_bases = BTreeSet::new();
            CrosswordGenerator::<CharT, StrT>::generator_impl(&gen.settings, &mut rr, &cs, &mut current_request, &mut current_crossword, &gen.words, &mut full_created_crossword_bases).await
               
        };

        CrosswordStream::new(gen_func)
    }
    // generate_crosswords(),

    #[async_recursion]
    async fn generator_impl(gen_settings: &CrosswordGeneratorSettings, rr: &mut Receiver<CrosswordGenerationRequest>, cs: &Sender<Crossword<CharT, StrT>>, current_request: &mut CrosswordGenerationRequest, current_crossword: &mut Crossword<CharT, StrT>, remained_words: &BTreeSet<Word<CharT, StrT>>, full_created_crossword_bases: &mut BTreeSet<Crossword<CharT, StrT>>)  
    {
        if !gen_settings.crossword_settings.check_nonrecoverables(current_crossword) 
        {
            return; 
        }

        if full_created_crossword_bases.iter().any(|cw| current_crossword.contains_crossword(cw))
        {
            return;
        }
        
        if remained_words.is_empty()
        {
            if gen_settings.crossword_settings.check_recoverables(current_crossword) 
            {
                while let CrosswordGenerationRequest::Count(0) = current_request
                {
                    match rr.recv().await
                    {
                        None | Some(CrosswordGenerationRequest::Stop) => { *current_request = CrosswordGenerationRequest::Stop; return },
                        Some(CrosswordGenerationRequest::Count(count)) => *current_request = CrosswordGenerationRequest::Count(count)
                    }
                }

                cs.send(current_crossword.clone()).await.unwrap();
            }
            return;
        }
        for current_word in remained_words.iter()
        {
            let mut new_remained_words = remained_words.clone();
            new_remained_words.remove(current_word);
            for step in current_crossword.calculate_possible_ways_to_add_word(current_word).iter()
            {
                current_crossword.add_word(step.clone()).unwrap();

                CrosswordGenerator::generator_impl(gen_settings, rr, cs, current_request, current_crossword, &new_remained_words, full_created_crossword_bases).await;

                if let CrosswordGenerationRequest::Stop = current_request { return; }
                
                let to_remove: Vec<Crossword<CharT, StrT>> = full_created_crossword_bases.iter().filter_map(|cw| cw.contains_crossword(&current_crossword).then_some(cw.clone())).collect();
                to_remove.into_iter().for_each(|cw| {full_created_crossword_bases.remove(&cw);});
                
                full_created_crossword_bases.insert(current_crossword.clone());

                current_crossword.remove_word(&step.value);
            }
        }

 
    }

}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Default, Debug, Serialize, Deserialize)]
pub enum CrosswordGenerationRequest
{
    #[default]
    Stop,
    Count(u32)
}

pub struct CrosswordStream<CharT: CrosswordChar + 'static, StrT: CrosswordString<CharT> + 'static>
{
    request_sender: Sender<CrosswordGenerationRequest>,
    crossword_reciever: Receiver<Crossword<CharT, StrT>>
}

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> CrosswordStream<CharT, StrT>
{
    
    pub fn new<F,Fut>(gen_func: F) -> CrosswordStream<CharT, StrT>
    where
        F: FnOnce(Receiver<CrosswordGenerationRequest>, Sender<Crossword<CharT, StrT>>) -> Fut,
        Fut: Future<Output=()> + Send + 'static
    {
        let (rs, rr) = mpsc::channel(100);
        let (cs, cr) = mpsc::channel(100);

        task::spawn(gen_func(rr, cs));
        
        CrosswordStream { request_sender: rs, crossword_reciever: cr }
    }

    pub async fn request_crossword(&self, req: CrosswordGenerationRequest)
    {
        self.request_sender.send(req).await.unwrap();
    }
}  

impl<CharT: CrosswordChar, StrT: CrosswordString<CharT>> Stream for CrosswordStream<CharT, StrT>
{
    type Item = Crossword<CharT, StrT>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>>
    {
        self.crossword_reciever.poll_recv(cx)
    }
}