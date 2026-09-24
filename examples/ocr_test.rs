fn main() {
    for p in std::env::args().skip(1) {
        let t = std::time::Instant::now();
        let img = calculator::ocr::prepare(image::open(&p).unwrap());
        let lines = calculator::ocr::recognize(&img).unwrap();
        let best = calculator::ocr::best_line(&lines);
        let toks = best.as_deref().map(calculator::ocr::to_tokens).unwrap_or_default();
        println!("{p}: lines={lines:?} best={best:?} tokens='{}' ({:?})", calculator::expr::display_tokens(&toks), t.elapsed());
    }
}
