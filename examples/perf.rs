//! Repeatable host benchmark, not a measurement of Android touch latency.
use calculator::{app::{App,Action,draw},draw::{Canvas,Fonts}};
use std::time::Instant;
fn main() {
 let mut app=App::default(); app.handle(&Action::Key("1".into()));app.handle(&Action::Key("+".into()));app.handle(&Action::Key("2".into()));
 let fonts=Fonts::new(); let mut px=vec![0u8;720*1600*4];
 let t=Instant::now(); for _ in 0..30 { std::hint::black_box(app.build(720.,1600.,1.75)); } println!("layout ms: {:.3}",t.elapsed().as_secs_f64()*1000./30.);
 let frame=app.build(720.,1600.,1.75); let t=Instant::now();
 for _ in 0..30 { let mut cv=Canvas{px:&mut px,w:720,h:1600,stride:720};draw(&app,&frame,&fonts,&mut cv,1.75,None); }
 println!("draw ms: {:.3}",t.elapsed().as_secs_f64()*1000./30.);
 let t=Instant::now();for _ in 0..100 {let frame=app.build(720.,1600.,1.75);let mut cv=Canvas{px:&mut px,w:720,h:1600,stride:720};draw(&app,&frame,&fonts,&mut cv,1.75,None);}println!("layout + draw ms: {:.3}",t.elapsed().as_secs_f64()*1000./100.);
}
