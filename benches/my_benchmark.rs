use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use crossword_generator::{generator::{CrosswordGenerationRequest, CrosswordGenerator, CrosswordGeneratorSettings}, word::Word};
use tokio::runtime::Runtime;
use tokio_stream::StreamExt;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("crossword");

    group.bench_function(BenchmarkId::new("", ""),
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
                let mut str = generator.crossword_stream();
                str.request_crossword(CrosswordGenerationRequest::Endless).await;
                while let Some(_) = str.next().await {}
            });
        });
    });

    group.finish();

}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

// NOTEBOOK BATTERY ON
// Stream, without &[CharT] optimisation       124.33 ms
// Stream, with &[CharT] optimisation          60.610 ms