// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod annotation;
mod app;
mod pdf;
mod viewer;

slint::include_modules!();

fn main() {
    let ui = App::new().unwrap();

    app::setup(&ui);

    ui.run().unwrap();
}
