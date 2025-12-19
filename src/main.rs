#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
// ^ Hide console in release builds

mod app;
mod audio;
mod config;
mod handlers;
mod icons;
mod state;
mod ui;
mod utils;
mod video;

use app::CrorecutApp;

fn main() -> iced::Result {
    env_logger::init();

    let icon =
        iced::window::icon::from_file_data(icons::WINDOW_ICON, Some(image::ImageFormat::Ico))
            .expect("Failed to load window icon");

    iced::application(CrorecutApp::new, CrorecutApp::update, CrorecutApp::view)
        .title(CrorecutApp::title)
        .subscription(CrorecutApp::subscription)
        .settings(iced::Settings {
            vsync: false,
            ..Default::default()
        })
        .window(iced::window::Settings {
            size: iced::Size::new(960.0, 600.0),
            min_size: Some(iced::Size::new(800.0, 540.0)),
            icon: Some(icon),
            ..Default::default()
        })
        .run()
}
