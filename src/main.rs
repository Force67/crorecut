#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
// ^ Hide console in release builds

mod app;
mod config;
mod icons;
mod ui;
mod utils;
mod video;

use app::App;

fn main() -> iced::Result {
    env_logger::init();

    let icon =
        iced::window::icon::from_file_data(icons::WINDOW_ICON, Some(image::ImageFormat::Ico))
            .expect("Failed to load window icon");

    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .window(iced::window::Settings {
            size: iced::Size::new(960.0, 600.0),
            icon: Some(icon),
            ..Default::default()
        })
        .run()
}
