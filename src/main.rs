#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hides console in release mode

mod app;
mod config;
mod ui;
mod utils;
mod video;

use app::App;

fn main() -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .run_with(App::new)
}
