#[macro_use]
extern crate criterion;

use criterion::Criterion;

use std::{borrow::Cow, path::Path};

#[inline]
pub fn file_pattern_match<S>(file_name: &str, patts: &[S]) -> bool
where
    S: AsRef<str>,
{
    let p = Path::new(file_name);
    if let Some(ext) = p.extension() {
        for patt in patts {
            if patt.as_ref().contains("*.") {
                if ext == &patt.as_ref()[2..patt.as_ref().len()] {
                    return true;
                }
            } else {
                if file_name == patt.as_ref() {
                    return true;
                }
            }
        }
    }
    false
}

fn bench_file_pattern_match(c: &mut Criterion) {
    c.bench_function("file_pattern_matcher", |b| {
        let patts = ["README.md", "*.iml", "*.zip", "*.rar", "*.iso", "*.war"];
        b.iter(|| file_pattern_match("test.iml", &patts))
    });
    c.bench_function("file_pattern_matcher_2", |b| {
        let patts: Vec<String> = vec![
            "README.md".to_string(),
            "*.iml".to_string(),
            "*.zip".to_string(),
            "*.rar".to_string(),
            "*.iso".to_string(),
            "*.war".to_string(),
        ];
        b.iter(|| file_pattern_match("test.iml", &patts[..]))
    });
}

criterion_group!(benches, bench_file_pattern_match);
criterion_main!(benches);
