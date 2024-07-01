#![allow(unused)]

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use crossword_generator::{generator::{CrosswordGenerationRequest, CrosswordGenerator, CrosswordGeneratorSettings}, word::Word};
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossword");

    #[cfg(feature = "multi-thread")]
    group.bench_function(BenchmarkId::new("randomized", ""),
    |b|
    {
        let rt = Runtime::new().unwrap();
        b.iter(||
        {
            let mut generator = CrosswordGenerator::<u8, Vec<u8>>::default();
            generator.settings = CrosswordGeneratorSettings::default();
            generator.words = vec!["Hello", "world", "asdf", "myname", "sesame", "yeeee", "nouyt"].into_iter().map(|s| Word::new(<String as AsRef<[u8]>>::as_ref(&s.to_lowercase()).to_owned(), None)).collect();
            

            rt.block_on(async move
            {
                let mut str = generator.crossword_stream_randomized(ToOwned::to_owned);
                str.request_crossword(CrosswordGenerationRequest::All).await;
                while let Some(_) = str.next().await {}
            });
        });
    });

    #[cfg(feature = "multi-thread")]
    group.bench_function(BenchmarkId::new("sorted", ""),
    |b|
    {
        let rt = Runtime::new().unwrap();
        b.iter(||
        {
            let mut generator = CrosswordGenerator::<u8, Vec<u8>>::default();
            generator.settings = CrosswordGeneratorSettings::default();
            generator.words = vec!["Hello", "world", "asdf", "myname", "sesame", "yeeee", "nouyt"].into_iter().map(|s| Word::new(<String as AsRef<[u8]>>::as_ref(&s.to_lowercase()).to_owned(), None)).collect();
            

            rt.block_on(async move
            {
                let mut str = generator.crossword_stream_sorted(ToOwned::to_owned);
                str.request_crossword(CrosswordGenerationRequest::All).await;
                while let Some(_) = str.next().await {}
            });
        });
    });

    group.finish();

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);